extends RefCounted

## The ground's height: the browser's paged height grid (`height-pages`, as
## `packTerrainHeights` uploads it) read by the simulation's triangle rule
## (`terrainGrid.ts` `groundHeight`). Each cell of the vertex grid splits on
## its south-west to north-east diagonal; an absent page is flat at zero; a
## point past the grid's edge takes the edge's height. Everything native seats
## on the ground seats on this.

var nx: int
var ny: int
var spacing: float
var _page_size: int
var _cols: int
## Page id -> present page + 1, or 0.
var _directory: PackedInt32Array
var _heights: PackedFloat32Array

## Decodes a `height-pages` table. Returns `{ok: true, heights}` or
## `{ok: false, detail}`.
static func decode(words: PackedByteArray, layout: Dictionary) -> Dictionary:
	for key in ["nx", "ny", "spacing", "pageSize"]:
		if not (layout.get(key) is float or layout.get(key) is int) or float(layout[key]) <= 0.0:
			return {"ok": false, "detail": "height-pages layout needs a positive %s" % key}
	if words.size() < 12 or words.size() % 4 != 0:
		return {"ok": false, "detail": "height pages need three header words"}
	var field := new()
	field.nx = int(layout.nx)
	field.ny = int(layout.ny)
	field.spacing = float(layout.spacing)
	field._page_size = words.decode_u32(0)
	field._cols = words.decode_u32(4)
	var count := words.decode_u32(8)
	if field.nx < 2 or field.ny < 2 or field._page_size != int(layout.pageSize) or field._cols != ceili(float(field.nx) / field._page_size):
		return {"ok": false, "detail": "the height pages' header disagrees with their layout"}
	if count != 0 and count != field._cols * ceili(float(field.ny) / field._page_size):
		return {"ok": false, "detail": "the height directory has %d entries, not one per page" % count}
	var present := (words.size() / 4 - 3 - count) / (field._page_size * field._page_size)
	if words.size() / 4 != 3 + count + present * field._page_size * field._page_size:
		return {"ok": false, "detail": "the height table's length is not its header, directory and whole pages"}
	field._directory = words.slice(12, 12 + count * 4).to_int32_array()
	for entry in field._directory:
		if entry < 0 or entry > present:
			return {"ok": false, "detail": "the height directory names a page the table does not hold"}
	field._heights = words.slice(12 + count * 4).to_float32_array()
	return {"ok": true, "heights": field}

## The height of vertex `(i, j)`.
func sample(i: int, j: int) -> float:
	if _directory.is_empty():
		return 0.0
	var page := _directory[(j / _page_size) * _cols + i / _page_size]
	if page == 0:
		return 0.0
	return _heights[(page - 1) * _page_size * _page_size + (j % _page_size) * _page_size + i % _page_size]

## The lowest vertex height: an absent page is flat at zero.
func lowest() -> float:
	var low := INF
	for height in _heights:
		low = minf(low, height)
	if _directory.is_empty() or _directory.has(0):
		low = minf(low, 0.0)
	return low

## Whether any page holds relief in the vertex rectangle `[i0, j0]` to
## `[i1, j1]` (inclusive): elsewhere the ground is flat at zero.
func has_relief(i0: int, j0: int, i1: int, j1: int) -> bool:
	if _directory.is_empty():
		return false
	for pj in range(j0 / _page_size, j1 / _page_size + 1):
		for pi in range(i0 / _page_size, i1 / _page_size + 1):
			if _directory[pj * _cols + pi] != 0:
				return true
	return false

## The ground height at sim `(x, y)`, clamped to the grid's edge.
func height_at(x: float, y: float) -> float:
	var fx := clampf(x / spacing, 0.0, nx - 1)
	var fy := clampf(y / spacing, 0.0, ny - 1)
	var i := mini(floori(fx), nx - 2)
	var j := mini(floori(fy), ny - 2)
	var u := fx - i
	var v := fy - j
	var h00 := sample(i, j)
	var h11 := sample(i + 1, j + 1)
	if u >= v:
		var h10 := sample(i + 1, j)
		return h00 + u * (h10 - h00) + v * (h11 - h10)
	var h01 := sample(i, j + 1)
	return h00 + v * (h01 - h00) + u * (h11 - h01)
