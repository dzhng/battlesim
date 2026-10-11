extends "res://tests/check.gd"

## Native decodes every sample of the fixture captures exactly as the
## browser's `observation.ts` does: every group, section and field, compared
## against the browser's own decode of the same words
## (`fixtures/publication/<scene>-observations.json`, written by
## `web/tests/nativePublicationDecode.test.ts`). A malformed stream is a named
## blocker.

const Capture = preload("res://capture/presentation_capture.gd")

const FIXTURES := "res://tests/fixtures/publication"

func run() -> bool:
	for scene in ["paris-corner", "market-town"]:
		var expected: Dictionary = read_json(FIXTURES.path_join("%s-observations.json" % scene))
		var parsed := Capture.parse(FileAccess.get_file_as_string("res://tests/fixtures".path_join(expected.capture)))
		check(parsed.valid, "%s: the capture is accepted: %s" % [scene, parsed.get("reason", "")])
		var decoded := Capture.decode_samples(parsed)
		check(decoded.ok, "%s: every sample decodes: %s" % [scene, decoded.get("blocker", {})])
		if not decoded.ok:
			continue
		check(decoded.samples.size() == expected.observations.size(), "%s: one observation per sample" % scene)
		for i in mini(decoded.samples.size(), expected.observations.size()):
			var difference := _difference(expected.observations[i], decoded.samples[i], "")
			if not difference.is_empty():
				check(false, "%s sample %d differs from the browser at %s" % [scene, i, difference])
				break

	var paris := FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("capture.json")
	_refuses(paris, "an unknown group", "group mystery is unknown", func(parsed):
		parsed.layout.groups.append({"name": "mystery", "count": "ownCount", "fields": [], "sections": []}))
	_refuses(paris, "a short payload", "group own's payload", func(parsed):
		var publication: Array = parsed.capture.samples[0].publication
		publication.resize(parsed.layout.header.size() + parsed.layout.groupDelivery.fields.size() + 1))
	_refuses(paris, "a bad encoding", "group own's encoding 9.0 is not one of the layout's", func(parsed):
		parsed.capture.samples[0].publication[parsed.layout.header.size() + 1] = _float_bits(9.0))
	_refuses(paris, "an encoding the layout names but native lacks", "encoding zip is unknown", func(parsed):
		parsed.layout.groupDelivery.encodings.append("zip"))
	return true

## Decoding the fixture after `mutate` stops at a `publication` blocker whose
## detail names `reason`.
func _refuses(path: String, what: String, reason: String, mutate: Callable) -> void:
	var parsed := Capture.parse(FileAccess.get_file_as_string(path))
	mutate.call(parsed)
	var decoded := Capture.decode_samples(parsed)
	var named: bool = not decoded.ok and decoded.blocker.name == "publication" and String(decoded.blocker.detail).contains(reason)
	check(named, "%s is a named publication blocker (%s): %s" % [what, reason, decoded.get("blocker", {})])

static func _float_bits(value: float) -> int:
	var bytes := PackedByteArray()
	bytes.resize(4)
	bytes.encode_float(0, value)
	return bytes.decode_u32(0)

## The path of the first place `actual` differs from the browser's
## `expected`, or "". Numbers compare as the float32s both decoders read;
## the browser's null stands for its NaN too, which JSON can't hold. The fog
## field's words are compared by digest.
static func _difference(expected: Variant, actual: Variant, path: String) -> String:
	match typeof(expected):
		TYPE_NIL:
			return "" if actual == null or (typeof(actual) == TYPE_FLOAT and is_nan(actual)) else path
		TYPE_INT, TYPE_FLOAT:
			if not typeof(actual) in [TYPE_INT, TYPE_FLOAT]:
				return path
			return "" if _f32(expected) == _f32(actual) else "%s (%s, not %s)" % [path, actual, expected]
		TYPE_BOOL:
			return "" if typeof(actual) == TYPE_BOOL and actual == expected else path
		TYPE_STRING:
			if String(expected).begins_with("sha256:") and actual is PackedInt64Array:
				return "" if expected == _digest(actual) else path
			return "" if typeof(actual) in [TYPE_STRING, TYPE_STRING_NAME] and String(actual) == expected else "%s (%s, not %s)" % [path, actual, expected]
		TYPE_ARRAY:
			if not (actual is Array or actual is PackedFloat32Array or actual is PackedInt64Array) or actual.size() != expected.size():
				return "%s (length)" % path
			for i in expected.size():
				var inner := _difference(expected[i], actual[i], "%s[%d]" % [path, i])
				if not inner.is_empty():
					return inner
			return ""
		TYPE_DICTIONARY:
			if typeof(actual) != TYPE_DICTIONARY or actual.size() != expected.size():
				return "%s (keys %s, not %s)" % [path, actual.keys() if actual is Dictionary else actual, expected.keys()]
			for key in expected:
				if not actual.has(key):
					return "%s.%s (missing)" % [path, key]
				var inner := _difference(expected[key], actual[key], "%s.%s" % [path, key] if not path.is_empty() else String(key))
				if not inner.is_empty():
					return inner
			return ""
	return path

static func _f32(value: float) -> float:
	return PackedFloat32Array([value])[0]

static func _digest(words: PackedInt64Array) -> String:
	var bytes := PackedByteArray()
	bytes.resize(words.size() * 4)
	for i in words.size():
		bytes.encode_u32(i * 4, words[i])
	var hashing := HashingContext.new()
	hashing.start(HashingContext.HASH_SHA256)
	hashing.update(bytes)
	return "sha256:" + hashing.finish().hex_encode()
