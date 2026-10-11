extends RefCounted

## A layer's chunks gathered in square cells, so a view meets a cell before
## its chunks: a cell wholly out of view, and casting nothing into it, costs
## one test, and a cell wholly in view puts all its chunks in view untested.
## Exact: a chunk is reported in view, or casting into it, exactly when its
## own box is (`detail_view.gd`).

## Per cell: its chunks, the union of their boxes and their tallest.
var _members: Array[PackedInt32Array] = []
var _boxes: Array[AABB] = []
var _heights := PackedFloat32Array()
var _cell_of := PackedInt32Array()

## Chunks with `boxes` (Godot space) and `heights` (the tallest thing in
## each, for the shadows it throws), gathered in cells `cell_m` square.
func _init(boxes: Array[AABB], heights: PackedFloat32Array, cell_m: float) -> void:
	var cells := {}
	for c in boxes.size():
		var centre := boxes[c].get_center()
		var cell := Vector2i(floori(centre.x / cell_m), floori(centre.z / cell_m))
		if not cells.has(cell):
			cells[cell] = _members.size()
			_members.append(PackedInt32Array())
			_boxes.append(AABB())
			_heights.append(0.0)
		_members[cells[cell]].append(c)
		_cell_of.append(cells[cell])
	for k in _members.size():
		update(k, boxes, heights)

## Gathers cell `k`'s box and height again from its chunks'.
func update(k: int, boxes: Array[AABB], heights: PackedFloat32Array) -> void:
	var box := AABB()
	var height := 0.0
	var first := true
	for c in _members[k]:
		if boxes[c].size == Vector3.ZERO:
			continue
		box = boxes[c] if first else box.merge(boxes[c])
		height = maxf(height, heights[c])
		first = false
	_boxes[k] = box
	_heights[k] = height

## The cell chunk `c` is gathered in.
func cell_of(c: int) -> int:
	return _cell_of[c]

## The chunks `view` meets, and, with `casting`, those out of it whose shadow
## can land in it: `[in view, casting only]`. Chunks with no box are skipped.
func visit(view, boxes: Array[AABB], heights: PackedFloat32Array, casting: bool) -> Array:
	var seen := PackedInt32Array()
	var cast := PackedInt32Array()
	for k in _members.size():
		var box := _boxes[k]
		if box.size == Vector3.ZERO:
			continue
		if view.box_meets(box):
			var within: bool = view.box_within(box)
			for c in _members[k]:
				if boxes[c].size == Vector3.ZERO:
					continue
				if within or view.box_meets(boxes[c]):
					seen.append(c)
				elif casting and view.casts_into_view(boxes[c], heights[c]):
					cast.append(c)
		elif casting and view.casts_into_view(box, _heights[k]):
			for c in _members[k]:
				if boxes[c].size != Vector3.ZERO and view.casts_into_view(boxes[c], heights[c]):
					cast.append(c)
	seen.sort()
	cast.sort()
	return [seen, cast]
