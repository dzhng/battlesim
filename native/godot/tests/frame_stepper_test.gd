extends "res://tests/check.gd"

const Stepper = preload("res://reel/frame_stepper.gd")

func run() -> bool:
	var frames: Array = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("capture.json")).frames
	check(_visited(frames, "all") == [0, 1, 2], "all visits every index once, in order")
	check(_visited(frames, "plates") == [1, 2], "plates visits only the records the browser shot")
	check(_visited(frames, "2,0") == [0, 2], "a list visits its indices once each, in record order")
	check(_visited(frames, "1,1") == [1], "a repeated index is visited once")

	for spec in ["3", "x", "", "1,,2"]:
		var refused := Stepper.select(frames, spec)
		check(not refused.ok and not refused.blocker.detail.is_empty(), "refuses frames '%s' with a reason" % spec)

	var no_plates: Array = frames.duplicate(true)
	for frame in no_plates:
		frame.plate = null
	check(not Stepper.select(no_plates, "plates").ok, "plates with no plate records is refused rather than rendering nothing")
	return true

func _visited(frames: Array, spec: String) -> Array:
	var selected := Stepper.select(frames, spec)
	if not selected.ok:
		return ["refused: " + selected.blocker.detail]
	var stepper = selected.stepper
	var visited := []
	var record = stepper.step()
	while record != null:
		visited.append(int(record.index))
		record = stepper.step()
	return visited
