extends RefCounted

## The capture side's learned ground: the scar marks its publications carry,
## replayed as the browser's `GroundView` replays them (`web/src/battle/sim/
## ground.ts`). A full publication replaces what was learned; each run sets its
## span of one tile's cells to the run's marks. A cell holds its crater,
## scorch, tracks and trampled marks and whether it was cleared (one byte
## each) once anything marked it.

var cell_m: float
var cols: int
var rows: int
## Cells a tile is wide and deep.
var tile: int
var tiles_x: int
## Cell index (row-major) -> crater | scorch << 8 | tracks << 16 |
## trampled << 24 | cleared << 32.
var _marks := {}
## The next sample `advance_to` applies.
var _next := 0
## What changed since `take_changes`: every tile, or these (tile index ->
## true).
var _all_changed := true
var _changed := {}

## `ground` is the publication layout's `ground` (`cellM`, `cols`, `rows`,
## `tileSize`).
func _init(ground: Dictionary) -> void:
	cell_m = float(ground.cellM)
	cols = int(ground.cols)
	rows = int(ground.rows)
	tile = int(ground.tileSize)
	tiles_x = ceili(float(cols) / tile)

## Applies every decoded sample (`presentation_capture.gd` `decode_samples`)
## up to and including `tick`, in order.
func advance_to(samples: Array, tick: int) -> void:
	while _next < samples.size() and int(samples[_next].tick) <= tick:
		apply(samples[_next].groundPatch)
		_next += 1

## Applies one publication's ground patch (`observation.gd`'s `groundPatch`):
## per run, its tile, `start + length * 256`, `crater + scorch * 256` and
## `tracks + trampled * 256 + cleared * 65536`.
func apply(patch: Dictionary) -> void:
	if patch.full:
		_marks.clear()
		_all_changed = true
	var runs: PackedFloat32Array = patch.runs
	for at in range(0, runs.size(), 4):
		var at_tile := int(runs[at])
		var span := int(runs[at + 1])
		var rest := int(runs[at + 3])
		var cell := int(runs[at + 2]) | ((rest & 0xffff) << 16) | ((rest >> 16) << 32)
		_changed[at_tile] = true
		for local in range(span % 256, span % 256 + span / 256):
			var i := (at_tile % tiles_x) * tile + local % tile
			var j := (at_tile / tiles_x) * tile + local / tile
			if cell == 0:
				_marks.erase(j * cols + i)
			else:
				_marks[j * cols + i] = cell

## What changed since the last call: `{all: true}` (a full publication, or
## nothing taken yet), or `{all: false, tiles: [tile index, ...]}`.
func take_changes() -> Dictionary:
	var changes := {"all": true} if _all_changed else {"all": false, "tiles": _changed.keys()}
	_all_changed = false
	_changed = {}
	return changes

## The tiles holding any mark or cleared cell (tile index -> true).
func marked_tiles() -> Dictionary:
	var tiles := {}
	for cell in _marks:
		tiles[(cell / cols / tile) * tiles_x + (cell % cols) / tile] = true
	return tiles

## The scar marks of the cell at column `i`, row `j`: crater | scorch << 8 |
## tracks << 16 | trampled << 24, the bytes the browser's scar texture holds
## for it (`scarTexture.ts`). Zero off the grid.
func marks(i: int, j: int) -> int:
	if i < 0 or j < 0 or i >= cols or j >= rows:
		return 0
	return int(_marks.get(j * cols + i, 0)) & 0xffffffff

## Whether the cell under sim `(x, y)` was cleared: a vehicle knocked its
## forest down (the browser's `GroundView.clearedRuns`).
func cleared_at(x: float, y: float) -> bool:
	var i := floori(x / cell_m)
	var j := floori(y / cell_m)
	if i < 0 or j < 0 or i >= cols or j >= rows:
		return false
	return (int(_marks.get(j * cols + i, 0)) >> 32) != 0
