class_name BattlePresentationCapture
extends RefCounted

## Decoder for the renderer-only battle-presentation-capture/v1 boundary.
##
## This artifact is an observation stream owned by Rust.  The decoder never
## advances a simulation and never turns malformed input into benchmark data.
## Callers receive an explicit result so a report can distinguish a valid
## capture from a synthetic or incomplete renderer probe.

const SCHEMA := "battle-presentation-capture/v1"

static func decode_json(text: String) -> Dictionary:
	var parser := JSON.new()
	if parser.parse(text) != OK:
		return _invalid("capture is not valid JSON")
	return validate(parser.data)

static func validate(value: Variant) -> Dictionary:
	if typeof(value) != TYPE_DICTIONARY:
		return _invalid("capture root must be an object")
	var capture: Dictionary = value
	if capture.get("schema", "") != SCHEMA:
		return _invalid("unsupported presentation capture schema")
	if typeof(capture.get("workload")) != TYPE_DICTIONARY:
		return _invalid("capture workload must be an object")
	var workload: Dictionary = capture.workload
	for field in ["id", "fingerprint", "scene", "map", "encounter"]:
		if typeof(workload.get(field)) != TYPE_STRING or String(workload[field]).is_empty():
			return _invalid("capture workload.%s must be a non-empty string" % field)
	if not _is_integer(workload.get("seed")):
		return _invalid("capture workload.seed must be an integer")
	if not _is_positive_integer(capture.get("tickHz")):
		return _invalid("capture tickHz must be positive")
	if not _is_non_negative_integer(capture.get("warmTick")):
		return _invalid("capture warmTick must be non-negative")
	if typeof(capture.get("layout")) != TYPE_STRING or String(capture.layout).is_empty():
		return _invalid("capture layout must be a non-empty JSON string")
	if capture.get("side") != "blue" and capture.get("side") != "red":
		return _invalid("capture side must be blue or red")
	if typeof(capture.get("samples")) != TYPE_ARRAY:
		return _invalid("capture samples must be an array")

	var previous_tick := -1
	var sample_index := 0
	for raw_sample in capture.samples:
		if typeof(raw_sample) != TYPE_DICTIONARY:
			return _invalid("capture sample %d must be an object" % sample_index)
		var sample: Dictionary = raw_sample
		if not _is_integer(sample.get("tick")) or int(sample.tick) < 0 or int(sample.tick) <= previous_tick:
			return _invalid("capture samples must increase by non-negative tick")
		if not _is_digest(sample.get("digest")):
			return _invalid("capture sample %d digest must be a 16-digit hex value" % sample_index)
		if typeof(sample.get("publication")) != TYPE_ARRAY:
			return _invalid("capture sample %d publication must be an array" % sample_index)
		for item in sample.publication:
			if not _is_u32(item):
				return _invalid("capture sample %d publication contains an invalid u32 word" % sample_index)
		var camera = sample.get("camera")
		if not _valid_camera(camera):
			return _invalid("capture sample %d camera pose is malformed" % sample_index)
		previous_tick = int(sample.tick)
		sample_index += 1

	return {
		"valid": true,
		"comparison_ready": true,
		"comparison_blocker": "",
		"capture": capture,
		"sample_count": sample_index,
		"publication_words": _publication_count(capture.samples),
	}

static func _invalid(reason: String) -> Dictionary:
	return {
		"valid": false,
		"comparison_ready": false,
		"comparison_blocker": reason,
	}

static func _publication_count(samples: Array) -> int:
	var count := 0
	for sample in samples:
		if typeof(sample) == TYPE_DICTIONARY and typeof(sample.get("publication")) == TYPE_ARRAY:
			count += sample.publication.size()
	return count

static func _is_integer(value: Variant) -> bool:
	return (typeof(value) == TYPE_INT or typeof(value) == TYPE_FLOAT) and is_finite(float(value)) and float(value) == floor(float(value))

static func _is_positive_integer(value: Variant) -> bool:
	return _is_integer(value) and int(value) > 0

static func _is_non_negative_integer(value: Variant) -> bool:
	return _is_integer(value) and int(value) >= 0

static func _is_finite_number(value: Variant) -> bool:
	return (typeof(value) == TYPE_INT or typeof(value) == TYPE_FLOAT) and is_finite(float(value))

static func _is_u32(value: Variant) -> bool:
	return _is_integer(value) and float(value) >= 0.0 and float(value) <= 4294967295.0

static func _is_digest(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING:
		return false
	var text := String(value)
	if text.length() != 16:
		return false
	for character in text:
		if not (character >= "0" and character <= "9") and not (character.to_lower() >= "a" and character.to_lower() <= "f"):
			return false
	return true

static func _valid_camera(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY:
		return false
	var camera: Dictionary = value
	var target = camera.get("target")
	if typeof(target) != TYPE_ARRAY or target.size() != 2:
		return false
	for coordinate in target:
		if not _is_finite_number(coordinate):
			return false
	for field in ["distance", "yaw", "pitch"]:
		if not _is_finite_number(camera.get(field)):
			return false
	return true
