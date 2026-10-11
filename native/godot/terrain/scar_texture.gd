extends RefCounted

## The side's learned scars as the ground's shaders read them: the bytes the
## browser's scar texture holds for every cell (`scarTexture.ts`), kept in
## one word table (`core/word_tables.gd`) that `ground.gdshaderinc` reads:
##
## - words `[0, NORMALIZATION)`: word `q` is the f32 `min(q, 4080) / 4080`,
##   the browser's normalisation words, which turn a filtered sixteenth of a
##   byte back into the value the browser's filter returns;
## - then the directory: per 16-cell tile, row-major, its page + 1, or 0
##   where no cell of the tile is marked;
## - then the pages, 256 words each: the tile's cells row-major, each
##   `crater | scorch << 8 | tracks << 16 | trampled << 24`.
##
## The browser caches pages in a hashed directory sized to the view; the
## whole learned ground fits here (a market-town battle marks a few hundred
## tiles), so every tile has a directory word and a marked tile a page. Only
## the tiles that changed are rewritten, and the table is uploaded whole.

const WordTables = preload("res://core/word_tables.gd")

const NORMALIZATION := 4096
## The browser's normalisation: a filtered value is in sixteenths of a byte,
## 255 × 16 at most.
const FULL_QUANTA := 4080
const TILE := 16
const PAGE_WORDS := TILE * TILE

var texture: ImageTexture
var _cols: int
var _rows: int
var _tiles_x: int
var _tiles_y: int
var _words := PackedByteArray()
## Tile index -> page.
var _pages := {}
var _free: Array[int] = []
var _page_count := 0

## The grid the manifest's scar params (terrain table `scars`, `{words,
## layout}`) name in their `pages` field; an empty grid without them.
func _init(params: Variant) -> void:
	var axes := _axes(params)
	_cols = axes.x
	_rows = axes.y
	_tiles_x = ceili(float(_cols) / TILE)
	_tiles_y = ceili(float(_rows) / TILE)
	_reset()
	_upload()

## Why the learned ground (`learned_ground.gd`) cannot fill this texture, or
## "" when it can.
func mismatch(ground) -> String:
	if ground.tile != TILE:
		return "the learned ground's tiles are %d cells, not the scar pages' %d" % [ground.tile, TILE]
	if Vector2i(ground.cols, ground.rows) != Vector2i(_cols, _rows):
		return "the scar params' grid is %d × %d cells, the learned ground's %d × %d" % [_cols, _rows, ground.cols, ground.rows]
	return ""

static func _axes(params: Variant) -> Vector2i:
	if not params is Dictionary:
		return Vector2i.ZERO
	for field in params.layout.get("fields", []):
		if field.name == "pages":
			var words: PackedByteArray = params.words
			return Vector2i(words.decode_u32(int(field.word) * 4), words.decode_u32(int(field.word) * 4 + 4))
	return Vector2i.ZERO

## Rewrites the tiles `ground` (`learned_ground.gd`) changed since the last
## call and uploads the table. True when anything changed.
func sync(ground) -> bool:
	var changes: Dictionary = ground.take_changes()
	var tiles: Array
	if changes.all:
		_reset()
		tiles = ground.marked_tiles().keys()
	else:
		tiles = changes.tiles
		if tiles.is_empty():
			return false
	for at in tiles:
		_write_tile(ground, at)
	_upload()
	return true

## Pages in use, for the report.
func pages() -> int:
	return _pages.size()

## No pages: the normalisation words and an empty directory.
func _reset() -> void:
	_pages.clear()
	_free.clear()
	_page_count = 0
	_words.resize(_pages_start() * 4)
	_words.fill(0)
	for q in NORMALIZATION:
		_words.encode_float(q * 4, float(mini(q, FULL_QUANTA)) / FULL_QUANTA)

func _pages_start() -> int:
	return NORMALIZATION + _tiles_x * _tiles_y

func _write_tile(ground, at: int) -> void:
	var x0 := (at % _tiles_x) * TILE
	var y0 := (at / _tiles_x) * TILE
	var cells := PackedInt64Array()
	cells.resize(PAGE_WORDS)
	var marked := false
	for local in PAGE_WORDS:
		cells[local] = ground.marks(x0 + local % TILE, y0 + local / TILE)
		marked = marked or cells[local] != 0
	var directory := (NORMALIZATION + at) * 4
	if not marked:
		if _pages.has(at):
			_free.append(_pages[at])
			_pages.erase(at)
			_words.encode_u32(directory, 0)
		return
	if not _pages.has(at):
		var page: int
		if _free.is_empty():
			page = _page_count
			_page_count += 1
			_words.resize((_pages_start() + _page_count * PAGE_WORDS) * 4)
		else:
			page = _free.pop_back()
		_pages[at] = page
		_words.encode_u32(directory, page + 1)
	var base: int = (_pages_start() + int(_pages[at]) * PAGE_WORDS) * 4
	for local in PAGE_WORDS:
		_words.encode_u32(base + local * 4, cells[local])

## The words as one texture: replaced in place when its size holds, so every
## material and compute binding of it stays valid.
func _upload() -> void:
	var count := _words.size() / 4
	var width := WordTables.WIDTH
	# Rows grow in powers of two, so a new page rarely resizes the texture.
	var height := 1
	while height * width < count:
		height *= 2
	var texels := _words.duplicate()
	texels.resize(width * height * 4)
	var image := Image.create_from_data(width, height, false, Image.FORMAT_RGBA8, texels)
	if texture == null:
		texture = ImageTexture.create_from_image(image)
	elif texture.get_size() == Vector2(width, height):
		texture.update(image)
	else:
		texture.set_image(image)
