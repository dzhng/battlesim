extends RefCounted

## Steps a capture's frame records by index. Each selected record is returned
## once, in record order; there is no delta, frame rate or wall clock, so a
## slow frame cannot skip a record.

var _records: Array = []
var _next := 0

## Selects records by `spec`: `plates` (the records the browser shot), `all`,
## or a comma-separated list of indices. Returns `{ok: true, stepper}` or
## `{ok: false, blocker: {name, detail}}`.
static func select(frames: Array, spec: String) -> Dictionary:
	var chosen: Array = []
	if spec == "all":
		chosen = frames
	elif spec == "plates":
		chosen = frames.filter(func(frame): return frame.plate != null)
		if chosen.is_empty():
			return _refuse("the capture has no plate records")
	else:
		var wanted := {}
		for part in spec.split(","):
			if not part.is_valid_int() or int(part) < 0 or int(part) >= frames.size():
				return _refuse("'%s' is not a frame index in 0..%d" % [part, frames.size() - 1])
			wanted[int(part)] = true
		chosen = frames.filter(func(frame): return wanted.has(int(frame.index)))
	var stepper := new()
	stepper._records = chosen
	return {"ok": true, "stepper": stepper}

static func _refuse(detail: String) -> Dictionary:
	return {"ok": false, "blocker": {"name": "frames", "detail": detail}}

## The next record, or null once every selected record has been returned.
func step() -> Variant:
	if _next >= _records.size():
		return null
	_next += 1
	return _records[_next - 1]

## Every selected record, in the order `step` returns them.
func records() -> Array:
	return _records
