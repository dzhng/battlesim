extends "res://tests/check.gd"

const Capture = preload("res://capture/presentation_capture.gd")

func run() -> bool:
	var fixture: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("capture.json"))
	var accepted := Capture.validate(fixture)
	check(accepted.valid, "the fixture capture is accepted: %s" % accepted.get("reason", ""))
	check(accepted.get("layout", {}).has("groups"), "the publication layout is decoded from its JSON string")

	_rejects(func(c): c.schema = "battle-presentation-capture/v1", "the /v1 id")
	_rejects(func(c): c.frames[0] = {"elapsedMs": 0.0, "tick": 3, "camera": {"target": [0, 0], "distance": 20, "yaw": 0, "pitch": 0.2}}, "the old orbit frame")
	_rejects(func(c): c.samples[0]["camera"] = {"target": [0, 0], "distance": 20, "yaw": 0, "pitch": 0.2}, "a sample that still carries a camera")
	_rejects(func(c): c.frames[1].index = 5, "a frame whose index is not its position")
	_rejects(func(c): c.frames[2].tick = 1, "a frame whose tick goes backwards")
	_rejects(func(c): c.frames[2].simTime = 0.0, "a frame whose sim time goes backwards")
	_rejects(func(c): c.frames[0].eye = [1, 2], "an eye with two coordinates")
	_rejects(func(c): c.frames[0].up = [0, 0, 0], "a zero up vector")
	_rejects(func(c): c.frames[0].canvas = [1280, 0], "an empty canvas")
	_rejects(func(c): c.frames[1].plate = "shot.png", "a plate not named for its frame")
	_rejects(func(c): c.samples[1].tick = c.samples[0].tick, "samples that do not increase by tick")
	_rejects(func(c): c.samples[0].digest = "nope", "a malformed digest")
	_rejects(func(c): c.erase("layout"), "a missing layout")
	check(not Capture.parse("{").valid, "text that is not JSON is rejected")
	return true

func _rejects(mutate: Callable, what: String) -> void:
	var capture: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("capture.json"))
	mutate.call(capture)
	var result := Capture.validate(capture)
	check(not result.valid and not String(result.get("reason", "")).is_empty(), "rejects %s with a reason" % what)
