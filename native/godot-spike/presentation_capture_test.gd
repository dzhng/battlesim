extends SceneTree

const CaptureDecoder = preload("res://presentation_capture.gd")

const VALID := {
	"schema": "battle-presentation-capture/v1",
	"workload": {"id": "menu-reel", "fingerprint": "deadbeef", "scene": "market-town", "map": "menu", "encounter": "menu", "seed": 1},
	"tickHz": 30,
	"warmTick": 90,
	"side": "blue",
	"layout": "{\"schema\":\"test\"}",
	"samples": [{"tick": 90, "digest": "0123456789abcdef", "publication": [1, 4294967294], "camera": {"target": [0.0, 0.0], "distance": 20.0, "yaw": 0.0, "pitch": 0.2}}],
}

func _init() -> void:
	var valid: Dictionary = CaptureDecoder.validate(VALID)
	assert(valid.valid)
	assert(valid.comparison_ready)
	assert(valid.sample_count == 1)
	assert(valid.publication_words == 2)

	var reordered: Dictionary = VALID.duplicate(true)
	reordered.samples = [VALID.samples[0], VALID.samples[0].duplicate(true)]
	reordered.samples[1].tick = 90
	var reordered_result: Dictionary = CaptureDecoder.validate(reordered)
	assert(not reordered_result.valid)
	assert(not reordered_result.comparison_ready)

	var malformed_digest: Dictionary = VALID.duplicate(true)
	malformed_digest.samples[0].digest = "nope"
	assert(not CaptureDecoder.validate(malformed_digest).valid)
	assert(not CaptureDecoder.decode_json("{").valid)

	print(JSON.stringify({"candidate": "battle-presentation-capture-decoder", "valid": true}))
	quit(0)
