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
	var layout = JSON.parse_string(String(capture.layout))
	if typeof(layout) != TYPE_DICTIONARY:
		return _invalid("capture layout must decode to an object")
	if capture.get("side") != "blue" and capture.get("side") != "red":
		return _invalid("capture side must be blue or red")
	if typeof(capture.get("samples")) != TYPE_ARRAY:
		return _invalid("capture samples must be an array")
	if typeof(capture.get("frames")) != TYPE_ARRAY:
		return _invalid("capture frames must be an array")
	var previous_frame := -1.0
	for raw_frame in capture.frames:
		if typeof(raw_frame) != TYPE_DICTIONARY or not _is_finite_number(raw_frame.get("elapsedMs")) or float(raw_frame.elapsedMs) < previous_frame:
			return _invalid("capture frame times must be increasing")
		if not _is_non_negative_integer(raw_frame.get("tick")) or not _valid_camera(raw_frame.get("camera")):
			return _invalid("capture frame is malformed")
		previous_frame = float(raw_frame.elapsedMs)

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
		"layout": layout,
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

## Reconstruct the Rust-owned publication groups from raw carrier words.
## This intentionally returns only the presentation fields needed by the reel;
## all packing, baselines and field offsets still come from the published layout.
static func decode_publication(capture_result: Dictionary, words: Array, baselines: Array) -> Dictionary:
	if not capture_result.get("valid", false):
		return {"valid": false, "units": [], "baselines": baselines}
	var layout: Dictionary = capture_result.layout
	var header: Array = layout.get("header", [])
	if words.size() < header.size():
		return {"valid": false, "units": [], "baselines": baselines}
	var cursor := header.size()
	var next_baselines: Array = baselines.duplicate(true)
	while next_baselines.size() < layout.groups.size():
		next_baselines.append(null)
	var payloads: Array = []
	for group_index in layout.groups.size():
		if cursor + 3 > words.size():
			return {"valid": false, "units": [], "baselines": baselines}
		var size := int(_carrier_float(words[cursor]))
		var encoding := int(_carrier_float(words[cursor + 1]))
		var payload_size := int(_carrier_float(words[cursor + 2]))
		cursor += 3
		if size < 0 or payload_size < 0 or cursor + payload_size > words.size():
			return {"valid": false, "units": [], "baselines": baselines}
		var payload_words: Array = words.slice(cursor, cursor + payload_size)
		cursor += payload_size
		var values: Array = _decode_group(layout, group_index, encoding, payload_words, size, next_baselines[group_index])
		if values.is_empty() and size > 0:
			return {"valid": false, "units": [], "baselines": baselines}
		payloads.append(values)
		next_baselines[group_index] = values
	var header_values: Dictionary = {}
	for i in header.size():
		header_values[String(header[i])] = _carrier_float(words[i])
	var fog_count := int(header_values.get(String(layout.fog.count), 0.0))
	var fog_full := int(header_values.get("fogFull", 0.0)) == 1
	var fog_nx := int(header_values.get("fogNx", 0.0))
	var fog_ny := int(header_values.get("fogNy", 0.0))
	var fog_words := int(ceil(float(fog_nx * fog_ny) / 32.0))
	var expected_fog_words := fog_words * 2 if fog_full else fog_count
	if fog_count < 0 or cursor + fog_count > words.size() or (fog_full and fog_count != expected_fog_words):
		return {"valid": false, "units": [], "baselines": baselines}
	cursor += fog_count
	var fog := {"full": fog_full, "nx": fog_nx, "ny": fog_ny, "cellM": float(header_values.get("fogCellM", 0.0)), "words": fog_words, "payloadWords": fog_count}
	var own: Array = []
	if payloads.size() > 0:
		var group: Dictionary = layout.groups[0]
		var fields: Array = group.fields
		var own_values: Array = payloads[0]
		var row_width := fields.size()
		var count_name := String(group.count)
		var count_index := header.find(count_name)
		var row_count := 0
		if count_index >= 0:
			row_count = int(_carrier_float(words[count_index]))
		for row in mini(row_count, int(own_values.size() / max(1, row_width))):
			var base := row * row_width
			var x := _carrier_float(own_values[base + fields.find("x")])
			var y := _carrier_float(own_values[base + fields.find("y")])
			var z := _carrier_float(own_values[base + fields.find("z")])
			var yaw := _carrier_float(own_values[base + fields.find("yaw")])
			var id := _carrier_float(own_values[base + fields.find("id")])
			var kind_index := int(_carrier_float(own_values[base + fields.find("kind")]))
			own.append({"id": id, "kind": kind_index, "position": [x, y, z], "yaw": yaw})
	return {"valid": true, "units": own, "baselines": next_baselines, "fog": fog}

static func mini(a: int, b: int) -> int:
	return a if a < b else b

static func _decode_group(layout: Dictionary, group_index: int, encoding: int, packed: Array, size: int, old: Variant) -> Array:
	var modes: Array = layout.groupDelivery.encodings
	if encoding < 0 or encoding >= modes.size():
		return []
	var mode := String(modes[encoding])
	if mode == "snapshot":
		return packed.duplicate()
	if mode == "replacement":
		if old == null:
			return []
		var values: Array = old.duplicate()
		var reader := _PackedReader.new(packed)
		var operations := reader.integer()
		var last := 0
		for _i in operations:
			var start := reader.integer()
			var count := reader.integer()
			if count <= 0 or start < last or start + count > size:
				return []
			if values.size() < size:
				values.resize(size)
			for at in range(start, start + count):
				values[at] = reader.literal(old, false)
			last = start + count
		reader.finish()
		return values
	if mode == "copies":
		if old == null:
			return []
		var values: Array = []
		var reader := _PackedReader.new(packed)
		var group: Dictionary = layout.groups[group_index]
		var stride := int(layout.groupDelivery.copyAlignments[group_index])
		if group.sections.size() > 0:
			stride = 1
		while values.size() < size:
			var source_plus := reader.integer()
			var count := reader.integer()
			if count <= 0 or count % max(1, stride) != 0:
				return []
			if source_plus == 0:
				for _j in count:
					values.append(reader.literal(old, false))
			else:
				var source := source_plus - 1
				if source < 0 or source + count > old.size():
					return []
				for j in count:
					values.append(old[source + j])
		reader.finish()
		return values
	if mode != "packed":
		return []
	if old == null and size == 0:
		return []
	var reader := _PackedReader.new(packed)
	var form := reader.read(8)
	var values: Array = []
	if form == 1:
		for _i in size:
			values.append(reader.literal(null, false))
	elif form == 0:
		if old == null:
			return []
		values = old.duplicate()
		values.resize(size)
		var operations := reader.integer()
		var last := 0
		for _i in operations:
			var start := reader.integer()
			var count := reader.integer()
			if count <= 0 or start < last or start + count > size:
				return []
			for at in range(start, start + count):
				values[at] = reader.literal(old, true, at)
			last = start + count
	elif form == 2:
		if old == null:
			return []
		var stride := int(layout.groupDelivery.copyAlignments[group_index])
		var group: Dictionary = layout.groups[group_index]
		if group.sections.size() > 0:
			stride = 1
		while values.size() < size:
			var source_plus := reader.integer()
			var count := reader.integer()
			if count <= 0 or count % max(1, stride) != 0:
				return []
			if source_plus == 0:
				for _j in count:
					values.append(reader.literal(old, true, values.size()))
			else:
				var source := source_plus - 1
				if source < 0 or source + count > old.size():
					return []
				for j in count:
					values.append(old[source + j])
	else:
		return []
	reader.finish()
	return values

static func _carrier_float(word: Variant) -> float:
	var bytes := PackedByteArray()
	bytes.resize(4)
	bytes.encode_u32(0, int(word))
	return bytes.decode_float(0)

class _PackedReader:
	var words: Array
	var bit := 0
	func _init(source: Array):
		words = source
	func read(count: int) -> int:
		if count <= 0 or bit + count > words.size() * 32:
			return 0
		var index := bit / 32
		var shift := bit % 32
		var value: int = (int(words[index]) >> shift) & 0xffffffff
		if shift + count > 32 and index + 1 < words.size():
			value |= (int(words[index + 1]) << (32 - shift))
		bit += count
		return value & ((1 << count) - 1 if count < 32 else 0xffffffff)
	func integer() -> int:
		var value := 0
		for byte_index in 4:
			var part := read(8)
			value += (part & 127) << (byte_index * 7)
			if part < 128:
				return value
		return -1
	func literal(baseline: Variant, xor_value: bool, at := 0) -> int:
		var tag := read(4)
		var count := tag if tag < 5 else tag - 5
		var value := 0 if count == 0 else read(count * 8)
		if tag >= 5 and baseline != null and at < baseline.size():
			value = value ^ int(baseline[at])
		return value
	func finish() -> void:
		while bit < words.size() * 32:
			if read(1) != 0:
				return
