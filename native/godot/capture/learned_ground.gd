extends RefCounted

## The capture side's learned ground: the scar marks its publications carry,
## replayed as the browser's `GroundView` replays them (`web/src/battle/sim/
## ground.ts`). A full publication replaces what was learned; each run sets its
## span of one tile's cells to the run's marks. A cell holds its crater,
## scorch, tracks and trampled marks (one byte each) once anything marked it.

var _cell_m: float
var _cols: int
var _rows: int
var _tile: int
var _tiles_x: int
## Cell index (row-major) -> crater | scorch << 8 | tracks << 16 | trampled << 24.
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
		apply(samples[_next].ground)
		_next += 1

## Applies one publication's decoded ground (`{full, rows}`).
func apply(ground: Dictionary) -> void:
	if ground.full:
		_marks.clear()
	for run in ground.rows:
		var tile := int(run.tile)
		var word := int(run.craterScorch) | ((int(run.tracksTrampledCleared) & 0xffff) << 16)
		for local in range(int(run.start), int(run.start) + int(run.length)):
			var i := (tile % _tiles_x) * _tile + local % _tile
			var j := (tile / _tiles_x) * _tile + local / _tile
			if word == 0:
				_marks.erase(j * _cols + i)
			else:
				_marks[j * _cols + i] = word

## Whether the cell under sim `(x, y)` holds a crater, scorch or tracks: the
## marks that leave the browser's grass out (`grassPass.ts`, its `bare`).
func bare_at(x: float, y: float) -> bool:
	var i := floori(x / _cell_m)
	var j := floori(y / _cell_m)
	if i < 0 or j < 0 or i >= _cols or j >= _rows:
		return false
	return (int(_marks.get(j * _cols + i, 0)) & 0xffffff) != 0
