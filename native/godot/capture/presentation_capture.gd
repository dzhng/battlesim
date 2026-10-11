extends RefCounted

## Strict parser for one scene's `battle-presentation-capture`, written by the
## browser capture run. The capture is an observation stream owned by Rust and
## a list of frame records owned by the browser; native draws each record and
## never advances a simulation. Any other shape, including a stale one, is
## rejected with a reason.

const StrictJson = preload("res://core/strict_json.gd")
const JsText = preload("res://core/js_text.gd")

const SCHEMA := "battle-presentation-capture"
const ROOT_KEYS := ["schema", "workload", "tickHz", "warmTick", "side", "layout", "samples", "frames"]
const WORKLOAD_KEYS := ["id", "fingerprint", "scene", "map", "encounter", "seed"]
const SAMPLE_KEYS := ["tick", "digest", "publication"]
const FRAME_KEYS := ["index", "reelMs", "tick", "simTime", "eye", "target", "up", "canvas", "plate"]

## Returns `{valid: true, capture, layout}` or `{valid: false, reason}`.
## Each record's `simTime` is read as the browser reads it, to the nearest
## double: a clock on a tick's boundary must fall on the same side of it.
static func parse(text: String) -> Dictionary:
	var parser := JSON.new()
	if parser.parse(text) != OK:
		return _invalid("capture is not valid JSON")
	var data = parser.data
	if typeof(data) == TYPE_DICTIONARY and typeof(data.get("frames")) == TYPE_ARRAY:
		var times := RegEx.create_from_string("\"simTime\":(-?[0-9][0-9.eE+-]*)").search_all(text)
		if times.size() != data.frames.size():
			return _invalid("capture names %d simTimes for %d frames" % [times.size(), data.frames.size()])
		for at in times.size():
			if typeof(data.frames[at]) == TYPE_DICTIONARY:
				data.frames[at].simTime = JsText.parse_number(times[at].get_string(1))
	return validate(data)

static func validate(value: Variant) -> Dictionary:
	var shape := StrictJson.exact_keys(value, ROOT_KEYS, "capture")
	if not shape.is_empty():
		return _invalid(shape)
	var capture: Dictionary = value
	if typeof(capture.schema) != TYPE_STRING or capture.schema != SCHEMA:
		return _invalid("capture schema must be %s, not %s" % [SCHEMA, capture.schema])
	shape = StrictJson.exact_keys(capture.workload, WORKLOAD_KEYS, "capture workload")
	if not shape.is_empty():
		return _invalid(shape)
	for field in ["id", "fingerprint", "scene", "map", "encounter"]:
		if not StrictJson.is_name(capture.workload[field]):
			return _invalid("capture workload.%s must be a non-empty string" % field)
	if not StrictJson.is_integer(capture.workload.seed):
		return _invalid("capture workload.seed must be an integer")
	if not StrictJson.is_integer(capture.tickHz) or int(capture.tickHz) <= 0:
		return _invalid("capture tickHz must be a positive integer")
	if not StrictJson.is_integer(capture.warmTick) or int(capture.warmTick) < 0:
		return _invalid("capture warmTick must be a non-negative integer")
	if not capture.side in ["blue", "red"]:
		return _invalid("capture side must be blue or red")
	if not StrictJson.is_name(capture.layout):
		return _invalid("capture layout must be a non-empty JSON string")
	var layout = JSON.parse_string(String(capture.layout))
	if typeof(layout) != TYPE_DICTIONARY:
		return _invalid("capture layout must decode to an object")
	if typeof(capture.samples) != TYPE_ARRAY:
		return _invalid("capture samples must be an array")
	var previous_tick := -1
	for at in capture.samples.size():
		var reason := _sample_error(capture.samples[at], previous_tick)
		if not reason.is_empty():
			return _invalid("capture sample %d %s" % [at, reason])
		previous_tick = int(capture.samples[at].tick)
	if typeof(capture.frames) != TYPE_ARRAY or capture.frames.is_empty():
		return _invalid("capture frames must be a non-empty array")
	var previous: Dictionary = {}
	for at in capture.frames.size():
		var reason := _frame_error(capture.frames[at], at, previous)
		if not reason.is_empty():
			return _invalid("capture frame %d %s" % [at, reason])
		previous = capture.frames[at]
	return {"valid": true, "capture": capture, "layout": layout}

static func _sample_error(value: Variant, previous_tick: int) -> String:
	var shape := StrictJson.exact_keys(value, SAMPLE_KEYS, "sample")
	if not shape.is_empty():
		return "has the wrong shape: " + shape
	if not StrictJson.is_integer(value.tick) or int(value.tick) <= previous_tick:
		return "tick must be a non-negative integer above the previous sample's"
	if not StrictJson.is_hex(value.digest, 16):
		return "digest must be a 16-digit hex value"
	if typeof(value.publication) != TYPE_ARRAY:
		return "publication must be an array"
	for word in value.publication:
		if not StrictJson.is_integer(word) or float(word) < 0.0 or float(word) > 4294967295.0:
			return "publication contains an invalid u32 word"
	return ""

static func _frame_error(value: Variant, at: int, previous: Dictionary) -> String:
	var shape := StrictJson.exact_keys(value, FRAME_KEYS, "frame")
	if not shape.is_empty():
		return "has the wrong shape: " + shape
	if not StrictJson.is_integer(value.index) or int(value.index) != at:
		return "index must equal its position"
	if not StrictJson.is_number(value.reelMs) or not StrictJson.is_number(value.simTime) or not StrictJson.is_integer(value.tick) or int(value.tick) < 0:
		return "reelMs and simTime must be finite and tick a non-negative integer"
	if not previous.is_empty() and (float(value.reelMs) < float(previous.reelMs) or int(value.tick) < int(previous.tick) or float(value.simTime) < float(previous.simTime)):
		return "reelMs, tick and simTime must not decrease"
	for field in ["eye", "target", "up"]:
		if not StrictJson.is_vector(value[field], 3):
			return "%s must be three finite numbers" % field
	if StrictJson.vector3(value.up).is_zero_approx() or StrictJson.vector3(value.eye).is_equal_approx(StrictJson.vector3(value.target)):
		return "view is degenerate: up is zero or eye equals target"
	if not StrictJson.is_vector(value.canvas, 2) or not StrictJson.is_integer(value.canvas[0]) or not StrictJson.is_integer(value.canvas[1]) or int(value.canvas[0]) <= 0 or int(value.canvas[1]) <= 0:
		return "canvas must be two positive integers"
	if value.plate != null and value.plate != "plate-%d.png" % at:
		return "plate must be plate-%d.png or null" % at
	return ""

static func _invalid(reason: String) -> Dictionary:
	return {"valid": false, "reason": reason}

## Decodes every sample of a parsed capture in tick order, through one
## publication stream: the Rust reader, in the extension (`PublicationReader`). Returns `{ok: true, samples}`, one
## observation per sample shaped like the browser's, or `{ok: false, blocker}`:
## a sample that fails to decode breaks every later baseline, so it stops the
## run.
static func decode_samples(capture_result: Dictionary) -> Dictionary:
	var stream = ClassDB.instantiate("PublicationReader")
	if not stream.open(capture_result.capture.layout):
		return {"ok": false, "blocker": {"name": "publication", "detail": "the layout is refused: %s" % stream.error}}
	var samples: Array = []
	for sample in capture_result.capture.samples:
		var observation = stream.decode(sample.publication)
		if observation == null:
			return {"ok": false, "blocker": {"name": "publication", "detail": "the sample at tick %d does not decode: %s" % [sample.tick, stream.error]}}
		samples.append(observation)
	return {"ok": true, "samples": samples}
