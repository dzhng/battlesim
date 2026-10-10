extends RefCounted

## Strict parser for one scene's `battle-presentation-capture`, written by the
## browser capture run. The capture is an observation stream owned by Rust and
## a list of frame records owned by the browser; native draws each record and
## never advances a simulation. Any other shape, including a stale one, is
## rejected with a reason.

const SCHEMA := "battle-presentation-capture"
const ROOT_KEYS := ["schema", "workload", "tickHz", "warmTick", "side", "layout", "samples", "frames"]
const WORKLOAD_KEYS := ["id", "fingerprint", "scene", "map", "encounter", "seed"]
const SAMPLE_KEYS := ["tick", "digest", "publication"]
const FRAME_KEYS := ["index", "reelMs", "tick", "simTime", "eye", "target", "up", "canvas", "plate"]

## Returns `{valid: true, capture, layout}` or `{valid: false, reason}`.
static func parse(text: String) -> Dictionary:
	var parser := JSON.new()
	if parser.parse(text) != OK:
		return _invalid("capture is not valid JSON")
	return validate(parser.data)

static func validate(value: Variant) -> Dictionary:
	var shape := exact_keys(value, ROOT_KEYS, "capture")
	if not shape.is_empty():
		return _invalid(shape)
	var capture: Dictionary = value
	if typeof(capture.schema) != TYPE_STRING or capture.schema != SCHEMA:
		return _invalid("capture schema must be %s, not %s" % [SCHEMA, capture.schema])
	shape = exact_keys(capture.workload, WORKLOAD_KEYS, "capture workload")
	if not shape.is_empty():
		return _invalid(shape)
	for field in ["id", "fingerprint", "scene", "map", "encounter"]:
		if not is_name(capture.workload[field]):
			return _invalid("capture workload.%s must be a non-empty string" % field)
	if not is_integer(capture.workload.seed):
		return _invalid("capture workload.seed must be an integer")
	if not is_integer(capture.tickHz) or int(capture.tickHz) <= 0:
		return _invalid("capture tickHz must be a positive integer")
	if not is_integer(capture.warmTick) or int(capture.warmTick) < 0:
		return _invalid("capture warmTick must be a non-negative integer")
	if not capture.side in ["blue", "red"]:
		return _invalid("capture side must be blue or red")
	if not is_name(capture.layout):
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
	var shape := exact_keys(value, SAMPLE_KEYS, "sample")
	if not shape.is_empty():
		return "has the wrong shape: " + shape
	if not is_integer(value.tick) or int(value.tick) <= previous_tick:
		return "tick must be a non-negative integer above the previous sample's"
	if not _is_digest(value.digest):
		return "digest must be a 16-digit hex value"
	if typeof(value.publication) != TYPE_ARRAY:
		return "publication must be an array"
	for word in value.publication:
		if not is_integer(word) or float(word) < 0.0 or float(word) > 4294967295.0:
			return "publication contains an invalid u32 word"
	return ""

static func _frame_error(value: Variant, at: int, previous: Dictionary) -> String:
	var shape := exact_keys(value, FRAME_KEYS, "frame")
	if not shape.is_empty():
		return "has the wrong shape: " + shape
	if not is_integer(value.index) or int(value.index) != at:
		return "index must equal its position"
	if not is_number(value.reelMs) or not is_number(value.simTime) or not is_integer(value.tick) or int(value.tick) < 0:
		return "reelMs and simTime must be finite and tick a non-negative integer"
	if not previous.is_empty() and (float(value.reelMs) < float(previous.reelMs) or int(value.tick) < int(previous.tick) or float(value.simTime) < float(previous.simTime)):
		return "reelMs, tick and simTime must not decrease"
	for field in ["eye", "target", "up"]:
		if not is_vector(value[field], 3):
			return "%s must be three finite numbers" % field
	if vector3(value.up).is_zero_approx() or vector3(value.eye).is_equal_approx(vector3(value.target)):
		return "view is degenerate: up is zero or eye equals target"
	if not is_vector(value.canvas, 2) or not is_integer(value.canvas[0]) or not is_integer(value.canvas[1]) or int(value.canvas[0]) <= 0 or int(value.canvas[1]) <= 0:
		return "canvas must be two positive integers"
	if value.plate != null and value.plate != "plate-%d.png" % at:
		return "plate must be plate-%d.png or null" % at
	return ""

static func _invalid(reason: String) -> Dictionary:
	return {"valid": false, "reason": reason}

## An empty string when `value` is an object with exactly `keys`.
static func exact_keys(value: Variant, keys: Array, what: String) -> String:
	if typeof(value) != TYPE_DICTIONARY:
		return "%s must be an object" % what
	for key in keys:
		if not value.has(key):
			return "%s is missing %s" % [what, key]
	for key in value:
		if not keys.has(key):
			return "%s has unknown field %s" % [what, key]
	return ""

static func vector3(value: Array) -> Vector3:
	return Vector3(float(value[0]), float(value[1]), float(value[2]))

static func is_vector(value: Variant, size: int) -> bool:
	if typeof(value) != TYPE_ARRAY or value.size() != size:
		return false
	for item in value:
		if not is_number(item):
			return false
	return true

static func is_name(value: Variant) -> bool:
	return typeof(value) == TYPE_STRING and not String(value).is_empty()

static func is_number(value: Variant) -> bool:
	return (typeof(value) == TYPE_INT or typeof(value) == TYPE_FLOAT) and is_finite(float(value))

static func is_integer(value: Variant) -> bool:
	return is_number(value) and float(value) == floor(float(value))

static func _is_digest(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING or String(value).length() != 16:
		return false
	for character in String(value).to_lower():
		if not "0123456789abcdef".contains(character):
			return false
	return true

## Decodes every sample of a parsed capture in tick order, carrying group and
## fog baselines from one publication to the next. Returns
## `{ok: true, samples: [{tick, units, fog, ground}]}` or `{ok: false, blocker}`: one
## sample that fails to decode breaks every later baseline, so it stops the run.
static func decode_samples(capture_result: Dictionary) -> Dictionary:
	var baselines: Array = []
	var fog_bits: Array = []
	var samples: Array = []
	for sample in capture_result.capture.samples:
		var decoded := decode_publication(capture_result, sample.publication, baselines, fog_bits)
		if not decoded.valid:
			return {"ok": false, "blocker": {"name": "publication", "detail": "the sample at tick %d does not decode: %s" % [sample.tick, decoded.get("error", "malformed carrier")]}}
		baselines = decoded.baselines
		fog_bits = decoded.fog.bits
		samples.append({"tick": int(sample.tick), "units": decoded.units, "fog": decoded.fog, "ground": decoded.ground})
	return {"ok": true, "samples": samples}

## Reconstruct the Rust-owned publication groups from raw carrier words.
## This intentionally returns only the presentation fields native draws;
## all packing, baselines and field offsets still come from the published layout.
static func decode_publication(capture_result: Dictionary, words: Array, baselines: Array, previous_fog: Array = []) -> Dictionary:
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
			return {"valid": false, "units": [], "baselines": baselines, "error": "group %d %s size %d payload %d old %d" % [group_index, String(layout.groupDelivery.encodings[encoding]) if encoding >= 0 and encoding < layout.groupDelivery.encodings.size() else "?", size, payload_size, -1 if next_baselines[group_index] == null else next_baselines[group_index].size()]}
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
	var fog_bits: Array = []
	if fog_full:
		if fog_count != fog_words * 2:
			return {"valid": false, "units": [], "baselines": baselines}
		for i in fog_words:
			var lo := int(_carrier_float(words[cursor + i * 2]))
			var hi := int(_carrier_float(words[cursor + i * 2 + 1]))
			fog_bits.append((lo & 65535) | ((hi & 65535) << 16))
	else:
		if previous_fog.size() != fog_words:
			return {"valid": false, "units": [], "baselines": baselines}
		fog_bits = previous_fog.duplicate()
		for at in range(0, fog_count, 3):
			if at + 2 >= fog_count:
				return {"valid": false, "units": [], "baselines": baselines}
			var index := int(_carrier_float(words[cursor + at]))
			if index < 0 or index >= fog_words:
				return {"valid": false, "units": [], "baselines": baselines}
			fog_bits[index] = (int(_carrier_float(words[cursor + at + 1])) & 65535) | ((int(_carrier_float(words[cursor + at + 2])) & 65535) << 16)
	cursor += fog_count
	var fog := {"full": fog_full, "nx": fog_nx, "ny": fog_ny, "cellM": float(header_values.get("fogCellM", 0.0)), "words": fog_words, "payloadWords": fog_count, "bits": fog_bits}
	var ground := _decode_ground(layout, header_values, words, cursor)
	if not ground.valid:
		return {"valid": false, "units": [], "baselines": baselines, "error": "ground"}
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
			var kind_name := ""
			var unit_kinds: Array = layout.get("unitKinds", [])
			if kind_index >= 0 and kind_index < unit_kinds.size():
				kind_name = String(unit_kinds[kind_index])
			own.append({"id": id, "kind": kind_index, "kindName": kind_name, "position": [x, y, z], "yaw": yaw})
	return {"valid": true, "units": own, "baselines": next_baselines, "fog": fog, "ground": ground}

static func _decode_ground(layout: Dictionary, header: Dictionary, words: Array, cursor: int) -> Dictionary:
	var count := int(header.get(String(layout.ground.count), 0.0))
	if count < 0 or cursor > words.size():
		return {"valid": false}
	var reader := _PackedReader.new(words.slice(cursor, words.size()))
	var rows: Array = []
	var tile := 0
	for _i in count:
		tile += reader.integer()
		var start := reader.read(8)
		var length := reader.read(8) + 1
		var mask := reader.read(5)
		if not reader.valid or tile < 0 or tile >= 2 ** 24 or start + length > 256:
			return {"valid": false}
		var crater := 0
		var tracks := 0
		for mark in 5:
			if (mask & (1 << mark)) == 0:
				continue
			var value := reader.read(8)
			if not reader.valid or value == 0:
				return {"valid": false}
			if mark < 2:
				crater += value * (2 ** (mark * 8))
			else:
				tracks += value * (2 ** ((mark - 2) * 8))
		rows.append({"tile": tile, "start": start, "length": length, "craterScorch": crater, "tracksTrampledCleared": tracks})
	reader.finish()
	return {"valid": true, "count": count, "rows": rows, "full": int(header.get("groundFull", 0.0)) == 1, "packedValid": reader.valid}

static func _decode_group(layout: Dictionary, group_index: int, encoding: int, packed: Array, size: int, old: Variant) -> Array:
	var modes: Array = layout.groupDelivery.encodings
	if encoding < 0 or encoding >= modes.size():
		return []
	var mode := String(modes[encoding])
	if mode == "snapshot":
		return packed.duplicate() if packed.size() == size else []
	if mode == "replacement":
		if old == null:
			return []
		var values: Array = old.duplicate()
		values.resize(size)
		if packed.is_empty():
			return values
		var cursor := 0
		var last := 0
		while cursor < packed.size():
			if cursor + 2 > packed.size():
				return []
			var start := int(_carrier_float(packed[cursor])); cursor += 1
			var count := int(_carrier_float(packed[cursor])); cursor += 1
			if count <= 0 or start < last or start + count > size:
				return []
			if cursor + count > packed.size():
				return []
			for at in range(start, start + count):
				values[at] = packed[cursor]
				cursor += 1
			last = start + count
		return values if cursor == packed.size() else []
	if mode == "copies":
		if old == null:
			return []
		var values: Array = []
		var cursor := 0
		var group: Dictionary = layout.groups[group_index]
		var stride := int(layout.groupDelivery.copyAlignments[group_index])
		if group.sections.size() > 0:
			stride = 1
		while values.size() < size:
			if cursor + 2 > packed.size():
				return []
			var source := int(_carrier_float(packed[cursor])); cursor += 1
			var count := int(_carrier_float(packed[cursor])); cursor += 1
			if count <= 0 or count % max(1, stride) != 0 or values.size() + count > size:
				return []
			if source == -1:
				if cursor + count > packed.size():
					return []
				for _j in count:
					values.append(packed[cursor]); cursor += 1
			else:
				if source < 0 or source + count > old.size():
					return []
				for j in count:
					values.append(old[source + j])
		return values if cursor == packed.size() else []
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
	var valid := true
	func _init(source: Array):
		words = source
	func read(count: int) -> int:
		if count <= 0 or count > 32 or bit + count > words.size() * 32:
			valid = false
			return 0
		var index := bit / 32
		var shift := bit % 32
		var value: int = (int(words[index]) >> shift) & 0xffffffff
		if shift + count > 32:
			value |= (int(words[index + 1]) << (32 - shift))
		bit += count
		return value & ((1 << count) - 1 if count < 32 else 0xffffffff)
	func integer() -> int:
		var value := 0
		for byte_index in 4:
			var part := read(8)
			if not valid:
				return -1
			value += (part & 127) << (byte_index * 7)
			if part < 128:
				if byte_index > 0 and part == 0:
					valid = false
				return value
		valid = false
		return -1
	func literal(baseline: Variant, xor_value: bool, at := 0) -> int:
		var tag := read(4)
		if not valid or tag > 9 or (xor_value == false and tag >= 5):
			valid = false
			return 0
		var count := tag if tag < 5 else tag - 5
		var value := 0 if count == 0 else read(count * 8)
		if not valid:
			return 0
		if tag >= 5 and baseline != null and at < baseline.size():
			value = value ^ int(baseline[at])
		return value
	func finish() -> void:
		if not valid or words.size() * 32 - bit > 31:
			valid = false
			return
		while bit < words.size() * 32:
			if read(1) != 0:
				valid = false
				return
