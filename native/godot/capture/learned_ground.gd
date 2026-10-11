extends RefCounted

## The capture side's learned ground: the scar marks its publications carry,
## replayed as the browser's `GroundView` replays them (`web/src/battle/sim/
## ground.ts`). A full publication replaces what was learned; each run sets its
## span of one tile's cells to the run's marks. A cell holds its crater,
## scorch, tracks and trampled marks and whether it was cleared (one byte
## each) once anything marked it.

var _cell_m: float
var _cols: int
var _rows: int
var _tile: int
var _tiles_x: int
## Cell index (row-major) -> crater | scorch << 8 | tracks << 16 |
## trampled << 24 | cleared << 32.
var _marks := {}
## The next sample `advance_to` applies.
var _next := 0

## `ground` is the publication layout's `ground` (`cellM`, `cols`, `rows`,
## `tileSize`).
func _init(ground: Dictionary) -> void:
	_cell_m = float(ground.cellM)
	_cols = int(ground.cols)
	_rows = int(ground.rows)
	_tile = int(ground.tileSize)
	_tiles_x = ceili(float(_cols) / _tile)

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
	var runs: PackedFloat32Array = patch.runs
	for at in range(0, runs.size(), 4):
		var tile := int(runs[at])
		var span := int(runs[at + 1])
		var rest := int(runs[at + 3])
		var cell := int(runs[at + 2]) | ((rest & 0xffff) << 16) | ((rest >> 16) << 32)
		for local in range(span % 256, span % 256 + span / 256):
			var i := (tile % _tiles_x) * _tile + local % _tile
			var j := (tile / _tiles_x) * _tile + local / _tile
			if cell == 0:
				_marks.erase(j * _cols + i)
			else:
				_marks[j * _cols + i] = cell

## Whether the cell under sim `(x, y)` holds a crater, scorch or tracks: the
## marks that leave the browser's grass out (`grassPass.ts`, its `bare`).
func bare_at(x: float, y: float) -> bool:
	var i := floori(x / _cell_m)
	var j := floori(y / _cell_m)
	if i < 0 or j < 0 or i >= _cols or j >= _rows:
		return false
	return (int(_marks.get(j * _cols + i, 0)) & 0xffffff) != 0

## Whether the cell under sim `(x, y)` was cleared: a vehicle knocked its
## forest down (the browser's `GroundView.clearedRuns`).
func cleared_at(x: float, y: float) -> bool:
	var i := floori(x / _cell_m)
	var j := floori(y / _cell_m)
	if i < 0 or j < 0 or i >= _cols or j >= _rows:
		return false
	return (int(_marks.get(j * _cols + i, 0)) >> 32) != 0
