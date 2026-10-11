extends RefCounted

## Decodes one side's publication stream into observations shaped exactly like
## the browser's (`web/src/battle/sim/observation.ts` `ObservationDecoder`):
## the same keys, the same nulls, the same words. Every group, section, the fog
## field and the ground runs come out; the published layout
## (`crates/sim/src/publication.rs` `layout_json`) owns the packing, the
## baselines and every field's offset. Native draws from these records and
## never simulates.
##
## A capture is one ordered stream, so a stale or malformed record is an error,
## never skipped: `decode` returns null and `error` names the reason.

## The groups native knows, and the fields and sections it reads from each.
## A layout with any other group, or without one of these, is refused.
const READS := {
	"own": {
		"fields": ["id", "kind", "x", "y", "z", "yaw", "goalX", "goalY", "policy", "direction", "reversing", "state", "blocker", "engagement", "hp", "suppression", "concealed", "deployProgress", "deployTarget", "garrisonBuildingLo", "garrisonBuildingHi", "garrisonPhase", "garrisonProgress", "garrisonX", "garrisonY", "garrisonHalfX", "garrisonHalfY", "stock", "service", "sightForward", "sightFront", "sightSide", "sightRear", "sightRange", "finalFacing", "areaX", "areaY", "areaM", "withdrawing", "protectionCharges", "protectionCooldownProgress", "smoking"],
		"sections": {
			"route": [], "queue": [], "members": [], "memberHp": [], "sees": [], "sightEyes": [],
			"memberIds": ["idLo", "idHi", "slot", "activeMount"],
			"memberOrders": ["x", "y", "coverNow", "coverThere"],
			"memberLeans": ["side", "x", "y"],
			"mounts": ["mount", "loaded", "ammo0", "ammo1", "aim", "reload", "targetKind", "targetId", "targetX", "targetY", "targetZ", "reason", "kinds", "guiding", "reloadKind"],
			"weaponPoses": ["mount", "bearing", "elevation", "shotsLo", "shotsHi", "operatorLo", "operatorHi"],
		},
	},
	"identified": {
		"fields": ["id", "kind", "cost", "x", "y", "z", "yaw", "vx", "vy", "reversing", "smoking"],
		"sections": {
			"members": [],
			"memberIds": ["idLo", "idHi", "slot", "activeMount"],
			"memberLeans": ["side", "x", "y"],
			"weaponPoses": ["mount", "bearing", "elevation", "shotsLo", "shotsHi", "operatorLo", "operatorHi"],
		},
	},
	"contacts": {"fields": ["id", "source", "x", "y", "z", "layer", "aloft", "radius", "evidenceTick", "expiresTick", "kind", "heardLow", "heardHigh"], "sections": {}},
	"audible": {"fields": ["listener", "category", "sector", "band", "moving"], "sections": {}},
	"projectiles": {"fields": ["own", "kind", "shooterLo", "shooterHi", "hit", "nx", "ny", "nz"], "sections": {"path": [], "ricochets": ["point", "nx", "ny", "nz"]}},
	"blasts": {"fields": ["x", "y", "z", "radius", "kind"], "sections": {}},
	"guided": {"fields": ["idLo", "idHi", "x", "y", "z", "px", "py", "pz", "supported"], "sections": {}},
	"corpses": {"fields": ["x", "y", "z", "own", "soldierLo", "soldierHi", "kind", "slot", "yaw"], "sections": {}},
	"knownProps": {"fields": ["kind", "x", "y", "yaw", "hx", "hy", "hz", "baseZ", "replacesLo", "replacesHi", "destroyed", "idLo", "idHi", "buildingLo", "buildingHi", "structureOwnerLo", "structureOwnerHi", "authoredPropLo", "authoredPropHi", "wreckOf"], "sections": {}},
	"fallenBodies": {"fields": ["propLo", "propHi", "x", "y", "towardX", "towardY", "tick"], "sections": {}},
	"pendingPurchases": {"fields": ["idLo", "idHi", "kind", "x", "y", "confirmedLo", "confirmedHi", "blocked"], "sections": {}},
	"objectives": {"fields": ["id", "x", "y", "radius", "owner", "capturing", "captureProgress", "contested"], "sections": {}},
	"reportTotals": {"fields": ["spawned", "spent", "killed", "refunded"], "sections": {}},
	"reportEvents": {"fields": ["tickLo", "tickHi", "kind", "side", "unitIdLo", "unitIdHi", "unitKind", "otherSide", "otherIdLo", "otherIdHi", "otherKind", "amount", "otherAmount"], "sections": {}},
	"crashes": {"fields": ["id", "own", "kind", "x", "y", "z", "yaw", "pitch", "roll"], "sections": {}},
}
## Groups whose unchanged words keep their previous views.
const STATIC_GROUPS := ["corpses", "knownProps", "fallenBodies"]
## Weapon rows a word of a firing report's heard mask carries.
const HEARD_WORD_BITS := 24
const MODES := ["replacement", "snapshot", "copies", "packed"]
const SKIRMISH_PHASES := ["preparation", "active", "finished"]
const SIDES := ["blue", "red"]
const MATCH_RESULTS := ["blue", "red", "draw"]

## Why the last `decode` returned null; empty after a decoded record.
var error := ""

var _layout: Dictionary
var _layout_error: String
var _limb: float
var _max_words: int
var _epoch := 0
var _side := ""
var _revision := 0.0
var _ground_revision := 0.0
## The previous observation's fog view, or null before the first.
var _fog = null
## The previous record's group words (`PackedInt32Array` raw bits), or empty.
var _words: Array = []
## The previous observation's static group views, by group name.
var _static_views := {}

func _init(layout: Dictionary) -> void:
	_layout = layout
	_layout_error = _check_layout(layout)
	if _layout_error.is_empty():
		_limb = pow(2.0, float(layout.limbBits))
		_max_words = int(layout.ground.maxRecordBytes) / 4

## The layout's refusal, or "" when native reads every group it names.
static func _check_layout(layout: Dictionary) -> String:
	var names := {}
	for group in layout.groups:
		var name := String(group.name)
		if not READS.has(name):
			return "the layout's group %s is unknown" % name
		names[name] = true
		var reads: Dictionary = READS[name]
		for field in reads.fields:
			if not field in group.fields:
				return "the layout's group %s has no field %s" % [name, field]
		var sections := {}
		for section in group.sections:
			sections[String(section.name)] = section.fields
		for section in reads.sections:
			if not sections.has(section):
				return "the layout's group %s has no section %s" % [name, section]
			for field in reads.sections[section]:
				if not field in sections[section]:
					return "the layout's section %s.%s has no field %s" % [name, section, field]
	for name in READS:
		if not names.has(name):
			return "the layout has no group %s" % name
	for mode in layout.groupDelivery.encodings:
		if not mode in MODES:
			return "the layout's group encoding %s is unknown" % mode
	return ""

## Decodes the next record of the stream from its raw u32 carrier words.
## Returns the observation, or null with `error` set; a failed record changes
## no baseline.
func decode(publication: Array) -> Variant:
	error = ""
	if not _layout_error.is_empty():
		return _fail(_layout_error)
	var bytes := PackedByteArray()
	bytes.resize(publication.size() * 4)
	for i in publication.size():
		bytes.encode_u32(i * 4, int(publication[i]))
	var words := bytes.to_int32_array()
	var data := bytes.to_float32_array()
	var header_names: Array = _layout.header
	if data.size() * 4 > int(_layout.ground.maxRecordBytes) or data.size() < header_names.size():
		return _fail("publication exceeds its admitted record bound")
	var header := {}
	for i in header_names.size():
		header[String(header_names[i])] = data[i]
	var epoch: float = header.groundEpoch
	if not _is_int(epoch) or epoch <= 0.0:
		return _fail("publication epoch must be positive")
	if epoch < _epoch:
		return _fail("publication epoch %d is stale after %d" % [epoch, _epoch])
	var sides: Array = _layout.ground.sides
	var side_index: float = header.groundSide
	var side := String(sides[int(side_index)]) if _is_int(side_index) and side_index >= 0.0 and side_index < sides.size() else ""
	var base = _limb_integer(header, "fogBase")
	var revision = _limb_integer(header, "fogRevision")
	if base == null or revision == null:
		return _fail("fog revision limbs must be exact 16-bit integers")
	var fresh := epoch > _epoch
	var ground_base: float = header.groundBase
	var ground_revision: float = header.groundRevision
	if side.is_empty() or not _is_int(ground_base) or ground_base < 0.0 or ground_base >= 16777216.0 \
			or not _is_int(ground_revision) or ground_revision < ground_base or ground_revision >= 16777216.0 \
			or (fresh and (header.groundFull != 1.0 or ground_base != 0.0)) \
			or (not fresh and (header.groundFull != 0.0 or ground_base != _ground_revision or side != _side)):
		return _fail("ground publication does not follow its side/epoch/revision baseline")
	if (fresh and (header.fogFull != 1.0 or base != 0.0)) or (not fresh and (side != _side or base != _revision)):
		return _fail("fog publication does not follow its side/epoch/revision baseline")
	if revision <= base:
		return _fail("fog publication revision must advance")
	var reconstructed = _reconstruct_groups(words, data, [] if fresh else _words)
	if reconstructed == null:
		return null
	var observation = _decode_frame(words, data, header, null if fresh else _fog, reconstructed.groups, reconstructed.tail, {} if fresh else _static_views)
	if observation == null:
		return null
	# Commit only once the whole record has decoded.
	_epoch = int(epoch)
	_side = side
	_revision = revision
	_ground_revision = ground_revision
	_fog = observation.fog
	_words = reconstructed.groups
	for name in STATIC_GROUPS:
		_static_views[name] = observation[name]
	return observation

func _fail(reason: String) -> Variant:
	if error.is_empty():
		error = reason
	return null

## A `<name>Lo`/`<name>Hi` header pair as one exact integer, or null.
func _limb_integer(header: Dictionary, name: String) -> Variant:
	var low: float = header[name + "Lo"]
	var high: float = header[name + "Hi"]
	if not _is_int(low) or not _is_int(high) or low < 0.0 or high < 0.0 or low >= _limb or high >= _limb:
		return null
	return low + high * _limb

## Reconstructs each group's words from its delivery; the fog and ground tail
## stays in the record. Returns `{groups, tail}` or null.
func _reconstruct_groups(words: PackedInt32Array, data: PackedFloat32Array, previous: Array) -> Variant:
	var delivery: Dictionary = _layout.groupDelivery
	var cursor: int = _layout.header.size()
	var length := cursor
	var groups: Array = []
	for g in _layout.groups.size():
		if cursor + delivery.fields.size() > data.size():
			return _fail("group %s's delivery runs past the record's end" % _layout.groups[g].name)
		var metadata := {}
		for field in delivery.fields:
			metadata[String(field)] = data[cursor]
			cursor += 1
		var size: float = metadata.length
		var encoding: float = metadata.encoding
		var payload: float = metadata.floats
		var name := String(_layout.groups[g].name)
		if not _is_int(encoding) or encoding < 0.0 or encoding >= delivery.encodings.size():
			return _fail("group %s's encoding %s is not one of the layout's" % [name, encoding])
		var mode := String(delivery.encodings[int(encoding)])
		if not _is_int(size) or size < 0.0 or not _is_int(payload) or payload < 0.0 or length + size > _max_words:
			return _fail("group %s's length %s or payload %s is outside its bound" % [name, size, payload])
		if cursor + payload > data.size():
			return _fail("group %s's payload of %d words runs past the record's end" % [name, payload])
		if previous.is_empty() and mode != "snapshot" and mode != "packed":
			return _fail("group %s's %s delivery has no baseline" % [name, mode])
		var count := int(size)
		var end := cursor + int(payload)
		var old: PackedInt32Array = previous[g] if not previous.is_empty() else PackedInt32Array()
		var values: PackedInt32Array
		if mode == "packed":
			var unpacked = _unpack_group(g, words.slice(cursor, end), count, old, not previous.is_empty())
			if unpacked == null:
				return null
			values = unpacked
		elif mode == "snapshot":
			if int(payload) != count:
				return _fail("observation group snapshot length mismatch")
			values = words.slice(cursor, end)
		elif mode == "copies":
			var stride := _stride(g)
			if stride < 0 or count % stride != 0:
				return _fail("observation copy alignment disagrees with its group")
			values = PackedInt32Array()
			var at := cursor
			while at < end:
				if at + delivery.copy.size() > end:
					return _fail("invalid observation row copy")
				var operation := {}
				for field in delivery.copy:
					operation[String(field)] = data[at]
					at += 1
				var source: float = operation.source
				var run: float = operation.length
				if not _is_int(run) or run <= 0.0 or int(run) % stride != 0 or values.size() + run > count \
						or (at + run > end if source == -1.0 else (not _is_int(source) or source < 0.0 or int(source) % stride != 0 or source + run > old.size())):
					return _fail("invalid observation row copy")
				if source == -1.0:
					values.append_array(words.slice(at, at + int(run)))
					at += int(run)
				else:
					values.append_array(old.slice(int(source), int(source) + int(run)))
			if values.size() != count:
				return _fail("observation row copies do not fill their group")
		else:
			values = old.slice(0, count)
			values.resize(count)
			var last := 0
			var at := cursor
			while at < end:
				if at + delivery.range.size() > end:
					return _fail("invalid observation group replacement range")
				var range_value := {}
				for field in delivery.range:
					range_value[String(field)] = data[at]
					at += 1
				var start: float = range_value.start
				var run: float = range_value.length
				if not _is_int(start) or start < 0.0 or not _is_int(run) or run <= 0.0 or start < last \
						or start + run > count or at + run > end or (start > old.size() and last < start):
					return _fail("invalid observation group replacement range")
				for k in int(run):
					values[int(start) + k] = words[at + k]
				at += int(run)
				last = int(start + run)
			if count > old.size() and last < count:
				return _fail("observation group growth must supply its new words")
		groups.append(values)
		cursor = end
		length += count
		if length > _max_words:
			return _fail("reconstructed observation exceeds its admitted record bound")
	if length + data.size() - cursor > _max_words:
		return _fail("reconstructed observation exceeds its admitted record bound")
	return {"groups": groups, "tail": cursor}

## A copy's row alignment: a whole row for a group without sections, else one
## word; -1 when the layout's alignment disagrees.
func _stride(g: int) -> int:
	var group: Dictionary = _layout.groups[g]
	var stride := int(_layout.groupDelivery.copyAlignments[g])
	return stride if stride == (group.fields.size() if group.sections.is_empty() else 1) else -1

## The packed delivery's bit grammar (`groupDelivery.packed`).
func _unpack_group(g: int, packed: PackedInt32Array, size: int, baseline: PackedInt32Array, has_baseline: bool) -> Variant:
	var reader := PackedReader.new(packed)
	var form := reader.read(8)
	if not reader.valid or form > 2 or (not has_baseline and form != 1):
		return _fail("packed observation group requires its baseline")
	var result := PackedInt32Array()
	result.resize(size)
	if form == 1:
		for at in size:
			if not _literal(reader, form, result, baseline, at):
				return null
	elif form == 0:
		for at in mini(size, baseline.size()):
			result[at] = baseline[at]
		var operations := reader.integer()
		if not reader.valid or operations > size:
			return _fail("invalid packed observation replacement count")
		var last := 0
		for _operation in operations:
			var start := reader.integer()
			var count := reader.integer()
			if not reader.valid or count == 0 or start < last or start + count > size or (start > baseline.size() and last < start):
				return _fail("invalid packed observation replacement range")
			for at in range(start, start + count):
				if not _literal(reader, form, result, baseline, at):
					return null
			last = start + count
		if size > baseline.size() and last < size:
			return _fail("packed observation growth must supply its new words")
	else:
		var stride := _stride(g)
		if stride < 0 or size % stride != 0:
			return _fail("packed observation copy alignment disagrees with its group")
		var at := 0
		while at < size:
			var source_plus_one := reader.integer()
			var count := reader.integer()
			var source := source_plus_one - 1
			if not reader.valid or count == 0 or count % stride != 0 or at + count > size \
					or (source_plus_one != 0 and (source % stride != 0 or source + count > baseline.size())):
				return _fail("invalid packed observation source copy")
			if source_plus_one == 0:
				for k in count:
					if not _literal(reader, form, result, baseline, at + k):
						return null
			else:
				for k in count:
					result[at + k] = baseline[source + k]
			at += count
	reader.finish()
	if not reader.valid:
		return _fail("packed observation group %s is truncated or badly padded" % _layout.groups[g].name)
	return result

func _literal(reader: PackedReader, form: int, result: PackedInt32Array, baseline: PackedInt32Array, at: int) -> bool:
	var tag := reader.read(4)
	if not reader.valid or tag > 9 or (form == 1 and tag >= 5):
		_fail("invalid packed observation literal tag")
		return false
	var count := tag if tag < 5 else tag - 5
	var value := 0 if count == 0 else reader.read(count * 8)
	if tag >= 5 and at < baseline.size():
		value ^= baseline[at] & 0xffffffff
	result[at] = value
	if not reader.valid:
		_fail("truncated packed publication")
	return reader.valid

## Splits each group into rows and sections, reads the fog and the ground,
## and maps every row to the browser's view.
func _decode_frame(words: PackedInt32Array, data: PackedFloat32Array, header: Dictionary, previous_fog, payloads: Array, tail: int, static_views: Dictionary) -> Variant:
	var rows_by_group := {}
	var reused := {}
	var floats_by_group: Array = []
	for index in _layout.groups.size():
		var group: Dictionary = _layout.groups[index]
		var name := String(group.name)
		var payload: PackedFloat32Array = payloads[index].to_byte_array().to_float32_array()
		floats_by_group.append(payload)
		var row_count: float = header[String(group.count)]
		var width: int = group.fields.size()
		if not _is_int(row_count) or row_count < 0.0 or row_count * width > payload.size():
			return _fail("observation group %s's row count exceeds its payload" % name)
		if not static_views.is_empty() and name in STATIC_GROUPS and group.sections.is_empty() and payloads[index] == _words[index]:
			if int(row_count) * width != payload.size():
				return _fail("observation group %s's values do not match their counts" % name)
			reused[name] = true
			continue
		var at := {}
		for i in width:
			at[String(group.fields[i])] = i
		var rows: Array = []
		var offset := 0
		for _n in int(row_count):
			rows.append(Row.new(payload, offset, at))
			offset += width
		# Each row's variable sections follow all the rows, in row order.
		for row: Row in rows:
			for section in group.sections:
				var section_width: int = section.fields.size()
				var count := row.f(String(section.count))
				if not _is_int(count) or count < 0.0 or offset + count * section_width > payload.size():
					return _fail("observation group %s's section %s count exceeds its payload" % [name, section.name])
				var points: Array = []
				for _k in int(count):
					points.append(Array(payload.slice(offset, offset + section_width)))
					offset += section_width
				row.sections[String(section.name)] = points
		if offset != payload.size():
			return _fail("observation group %s's values do not match their counts" % name)
		rows_by_group[name] = rows

	var fog = _decode_fog(data, header, previous_fog, tail)
	if fog == null:
		return null
	var cursor: int = tail + int(header[String(_layout.fog.count)])
	var ground_runs: float = header[String(_layout.ground.count)]
	if not _is_int(ground_runs) or ground_runs < 0.0:
		return _fail("ground runs do not match their count")
	var logical: int = _layout.header.size() + int(header[String(_layout.fog.count)]) + int(ground_runs) * _layout.ground.fields.size()
	for payload in payloads:
		logical += payload.size()
	if cursor > data.size() or logical > _max_words:
		return _fail("publication length does not match its layout")
	var ground_patch = _decode_ground_patch(header, words.slice(cursor), int(ground_runs))
	if ground_patch == null:
		return null

	var view := {
		"tick": header.tick,
		"own": rows_by_group.own.map(_own),
		"identified": rows_by_group.identified.map(_identified),
		"contacts": rows_by_group.contacts.map(_contact),
		"audible": rows_by_group.audible.map(_audible),
		"knownProps": static_views.knownProps if reused.has("knownProps") else rows_by_group.knownProps.map(_known_prop),
		"projectiles": rows_by_group.projectiles.map(_projectile),
		"blasts": rows_by_group.blasts.map(_blast),
		"corpses": static_views.corpses if reused.has("corpses") else rows_by_group.corpses.map(_corpse),
		"fallenBodies": static_views.fallenBodies if reused.has("fallenBodies") else rows_by_group.fallenBodies.map(_fallen_body),
		"crashes": rows_by_group.crashes.map(_crash),
		"guided": rows_by_group.guided.map(_guided),
		"encounter": null if header.encounterResult < 0.0 else {"heldS": header.encounterHeldS, "result": _word("encounterResults", header.encounterResult)},
		"skirmish": null if header.skirmishPhase < 0.0 else _skirmish(header, rows_by_group),
		"fog": fog,
		"groundPatch": ground_patch,
	}
	return null if not error.is_empty() else view

func _decode_fog(data: PackedFloat32Array, header: Dictionary, previous, cursor: int) -> Variant:
	var nx: float = header.fogNx
	var ny: float = header.fogNy
	var cell_m: float = header.fogCellM
	if not _is_int(nx) or not _is_int(ny) or nx < 0.0 or ny < 0.0:
		return _fail("fog field dimensions exceed the admitted delivery bound")
	var cells := int(nx) * int(ny)
	var words := ceili(cells / 32.0)
	if words > int(_layout.fog.maxWords) or (cells > 0 and not cell_m > 0.0):
		return _fail("fog field dimensions exceed the admitted delivery bound")
	var count: float = header[String(_layout.fog.count)]
	var full: bool = header.fogFull == 1.0
	if header.fogFull != 0.0 and not full:
		return _fail("unknown fog payload encoding")
	if not _is_int(count) or count < 0.0 or (count != words * 2 if full else (int(count) % 3 != 0 or count > words * 2)):
		return _fail("fog payload does not match its encoding")
	if (not full and previous == null) or (previous != null and (previous.nx != nx or previous.ny != ny or previous.cellM != cell_m)):
		return _fail("fog delta has no matching field baseline")
	if cursor + count > data.size():
		return _fail("publication length does not match its layout")
	var bits := PackedInt64Array()
	if full:
		bits.resize(words)
	else:
		bits = previous.bits if count == 0.0 else previous.bits.duplicate()
	if full:
		for i in words:
			var word = _fog_word(data, cursor + i * 2)
			if word == null:
				return null
			bits[i] = word
	else:
		var last := -1.0
		for at in range(cursor, cursor + int(count), 3):
			var index := data[at]
			if not _is_int(index) or index <= last or index >= words:
				return _fail("fog delta word indices must be ordered and inside the field")
			var word = _fog_word(data, at + 1)
			if word == null:
				return null
			bits[int(index)] = word
			last = index
	if cells % 32 != 0 and (bits[words - 1] >> (cells % 32)) != 0:
		return _fail("fog padding bits must be zero")
	return {"cellM": cell_m, "nx": nx, "ny": ny, "bits": bits}

## One fog word from its two limbs.
func _fog_word(data: PackedFloat32Array, at: int) -> Variant:
	var low := data[at]
	var high := data[at + 1]
	if not _is_int(low) or not _is_int(high) or low < 0.0 or high < 0.0 or low >= _limb or high >= _limb:
		return _fail("fog word limbs must be exact 16-bit integers")
	return int(low) | (int(high) << int(_layout.limbBits))

## The ground's run tail, as the browser's `GroundRunsPatch`: per run, its
## tile, `start + length * 256`, `crater + scorch * 256` and
## `tracks + trampled * 256 + cleared * 65536`.
func _decode_ground_patch(header: Dictionary, tail: PackedInt32Array, n: int) -> Variant:
	var ground: Dictionary = _layout.ground
	var fields: Array = ground.fields
	var tile_size := int(ground.tileSize)
	var cols := int(ground.cols)
	var rows := int(ground.rows)
	if fields.size() != 4 or tile_size != 16 or cols <= 0 or rows <= 0 or float(cols) * rows > 4294967296.0:
		return _fail("ground grid does not match its admitted tile codec")
	if n * fields.size() > _max_words or n * 29 > tail.size() * 32:
		return _fail("ground runs do not match their count")
	var reader := PackedReader.new(tail)
	var runs := PackedFloat32Array()
	runs.resize(n * 4)
	var tile := 0
	for row in range(0, n * 4, 4):
		tile += reader.integer()
		var start := reader.read(8)
		var length := reader.read(8) + 1
		var mask := reader.read(5)
		var a := 0
		var b := 0
		for mark in 5:
			if mask & (1 << mark) == 0:
				continue
			var value := reader.read(8)
			if reader.valid and value == 0:
				return _fail("ground mark presence must encode a nonzero byte")
			if mark < 2:
				a += value << (mark * 8)
			else:
				b += value << ((mark - 2) * 8)
		if not reader.valid:
			return _fail("truncated ground runs")
		if tile >= 16777216:
			return _fail("ground tile exceeds its exact address range")
		runs[row] = tile
		runs[row + 1] = start + length * 256
		runs[row + 2] = a
		runs[row + 3] = b
	reader.finish()
	if not reader.valid:
		return _fail("ground runs are truncated or badly padded")
	var at := {}
	for i in fields.size():
		at[String(fields[i])] = i
	var tiles_x := ceili(float(cols) / tile_size)
	var tiles_y := ceili(float(rows) / tile_size)
	var prior := -1
	var end := 0
	for row in range(0, n * 4, 4):
		var run_tile := int(runs[row + int(at.tile)])
		var span := int(runs[row + int(at.span)])
		var a := runs[row + int(at.craterScorch)]
		var b := runs[row + int(at.tracksTrampledCleared)]
		var start := span % 256
		var length := span / 256
		if run_tile < 0 or run_tile >= tiles_x * tiles_y or length <= 0 or start + length > 256 \
				or a < 0.0 or a > 65535.0 or b < 0.0 or b > 16777215.0 or run_tile < prior or (run_tile == prior and start < end):
			return _fail("ground run is unordered, overlapping or outside its exact encoding")
		var x := (run_tile % tiles_x) * tile_size
		var y := (run_tile / tiles_x) * tile_size
		if y + (start + length - 1) / tile_size >= rows or x + start % tile_size >= cols \
				or (x + tile_size > cols and ((start / tile_size) != (start + length - 1) / tile_size or x + (start + length - 1) % tile_size >= cols)):
			return _fail("ground run crosses unused cells in a partial edge tile")
		prior = run_tile
		end = start + length
	return {
		"epoch": header.groundEpoch,
		"side": _layout.ground.sides[int(header.groundSide)],
		"baseRevision": header.groundBase,
		"revision": header.groundRevision,
		"full": header.groundFull == 1.0,
		"runs": runs,
	}

# Row views -----------------------------------------------------------------

## A word from one of the layout's tables; an index outside it is an error.
func _word(table: String, index: float) -> Variant:
	var words: Array = _layout[table]
	if not _is_int(index) or index < 0.0 or index >= words.size():
		return _fail("%s index %s is outside its table" % [table, index])
	return words[int(index)]

## An exact integer from its limbs; null when absent.
func _limbs(row: Row, name: String) -> Variant:
	var low := row.f(name + "Lo")
	return null if low < 0.0 else low + row.f(name + "Hi") * _limb

func _section_limbs(point: Array, at: Dictionary, name: String) -> Variant:
	var low: float = point[at[name + "Lo"]]
	return null if low < 0.0 else low + float(point[at[name + "Hi"]]) * _limb

## Field name -> index in one section's points.
func _section_at(group: String, section: String) -> Dictionary:
	for g in _layout.groups:
		if g.name == group:
			for s in g.sections:
				if s.name == section:
					var at := {}
					for i in s.fields.size():
						at[String(s.fields[i])] = i
					return at
	return {}

func _member_ids(points: Array, at: Dictionary) -> Array:
	return points.map(func(p): return _section_limbs(p, at, "id"))

func _member_slots(points: Array, at: Dictionary) -> Array:
	return points.map(func(p): return p[at.slot])

func _member_active_mounts(points: Array, at: Dictionary) -> Array:
	return points.map(func(p): return null if p[at.activeMount] < 0.0 else p[at.activeMount])

func _leans(points: Array, at: Dictionary) -> Array:
	var leans: Array = []
	for p in points:
		leans.append(null if p[at.side] < 0.0 else {"side": _word("leanSides", p[at.side]), "at": [p[at.x], p[at.y]]})
	return leans

func _poses(points: Array, at: Dictionary) -> Array:
	var poses: Array = []
	for p in points:
		poses.append({
			"mount": p[at.mount],
			"bearing": p[at.bearing],
			"elevation": p[at.elevation],
			"shots": _section_limbs(p, at, "shots"),
			"operator": _section_limbs(p, at, "operator"),
		})
	return poses

func _orders(points: Array, at: Dictionary) -> Array:
	var orders: Array = []
	for p in points:
		orders.append({"spot": [p[at.x], p[at.y]], "coverNow": _tier(p[at.coverNow]), "coverThere": _tier(p[at.coverThere])})
	return orders

func _own(row: Row) -> Dictionary:
	var s := row.sections
	var ids := _section_at("own", "memberIds")
	var mounts := _section_at("own", "mounts")
	var policy := row.f("policy")
	var direction := row.f("direction")
	var blocker := row.f("blocker")
	var deploy_target := row.f("deployTarget")
	var garrison_phase := row.f("garrisonPhase")
	return {
		"id": row.f("id"),
		"kind": _word("unitKinds", row.f("kind")),
		"position": [row.f("x"), row.f("y"), row.f("z")],
		"yaw": row.f("yaw"),
		"goal": null if is_nan(row.f("goalX")) else [row.f("goalX"), row.f("goalY")],
		"policy": null if policy < 0.0 else _word("policies", policy),
		"direction": null if direction < 0.0 else _word("directions", direction),
		"reversing": row.f("reversing") == 1.0,
		"withdrawing": row.f("withdrawing") == 1.0,
		"protection": null if row.f("protectionCharges") < 0.0 else {
			"charges": row.f("protectionCharges"),
			"cooldown": null if row.f("protectionCooldownProgress") < 0.0 else row.f("protectionCooldownProgress"),
		},
		"state": _word("moveStates", row.f("state")),
		"blocker": null if blocker < 0.0 else blocker,
		"route": s.route,
		"queue": s.queue,
		"members": s.members,
		"memberIds": _member_ids(s.memberIds, ids),
		"memberSlots": _member_slots(s.memberIds, ids),
		"memberActiveMounts": _member_active_mounts(s.memberIds, ids),
		"memberOrders": _orders(s.memberOrders, _section_at("own", "memberOrders")),
		"memberLeans": _leans(s.memberLeans, _section_at("own", "memberLeans")),
		# The area fields are NaN for a vehicle.
		"area": null if is_nan(row.f("areaM")) else {"anchor": [row.f("areaX"), row.f("areaY")], "radius": row.f("areaM")},
		"finalFacing": row.f("finalFacing"),
		"sees": s.sees.map(func(p): return p[0]),
		"engagement": _word("engagements", row.f("engagement")),
		"mounts": s.mounts.map(func(p): return _mount(p, mounts)),
		"weaponPoses": _poses(s.weaponPoses, _section_at("own", "weaponPoses")),
		"hp": row.f("hp"),
		"smoking": row.f("smoking") == 1.0,
		"memberHp": s.memberHp.map(func(p): return p[0]),
		"suppression": _word("suppressionTiers", row.f("suppression")),
		"concealed": row.f("concealed") == 1.0,
		# Both deployment fields are -1 for units that never deploy.
		"deployment": null if deploy_target < 0.0 else {"progress": row.f("deployProgress"), "target": _word("postures", deploy_target)},
		# All three garrison fields are -1 without a building.
		"garrison": null if garrison_phase < 0.0 else {
			"building": _limbs(row, "garrisonBuilding"),
			"phase": _word("garrisonPhases", garrison_phase),
			"progress": row.f("garrisonProgress"),
			"center": [row.f("garrisonX"), row.f("garrisonY")],
			"half": [row.f("garrisonHalfX"), row.f("garrisonHalfY")],
		},
		"stock": null if row.f("stock") < 0.0 else row.f("stock"),
		"service": _word("serviceStatuses", row.f("service")),
		"sight": {
			"eyes": s.sightEyes,
			"forward": row.f("sightForward"),
			"shape": {"front": row.f("sightFront"), "side": row.f("sightSide"), "rear": row.f("sightRear")},
			"range": row.f("sightRange"),
		},
	}

func _tier(index: float) -> Variant:
	return null if index < 0.0 else _word("coverTiers", index)

func _mount(p: Array, at: Dictionary) -> Dictionary:
	var loaded: float = p[at.loaded]
	# Ammo per kind: -1 unlimited, -2 no such kind on this mount.
	var ammo := [p[at.ammo0], p[at.ammo1]].slice(0, maxi(0, int(p[at.kinds]))).map(func(n): return null if n == -1.0 else n)
	var kind = _word("targetKinds", p[at.targetKind])
	var target = null
	if kind == "ground":
		target = {"kind": kind, "point": [p[at.targetX], p[at.targetY], p[at.targetZ]]}
	elif kind != "none":
		target = {"kind": kind, "id": p[at.targetId]}
	return {
		"mount": p[at.mount],
		"loaded": null if loaded < 0.0 else loaded,
		"ammo": ammo,
		"aim": p[at.aim],
		"reload": p[at.reload],
		"target": target,
		"reason": _word("actionReasons", p[at.reason]),
		"guiding": p[at.guiding] == 1.0,
		"reloading": null if p[at.reloadKind] < 0.0 else p[at.reloadKind],
	}

func _identified(row: Row) -> Dictionary:
	var s := row.sections
	var ids := _section_at("identified", "memberIds")
	return {
		"id": row.f("id"),
		"kind": _word("unitKinds", row.f("kind")),
		"cost": row.f("cost"),
		"position": [row.f("x"), row.f("y"), row.f("z")],
		"yaw": row.f("yaw"),
		"velocity": [row.f("vx"), row.f("vy")],
		"members": s.members,
		"memberIds": _member_ids(s.memberIds, ids),
		"memberSlots": _member_slots(s.memberIds, ids),
		"memberActiveMounts": _member_active_mounts(s.memberIds, ids),
		"memberLeans": _leans(s.memberLeans, _section_at("identified", "memberLeans")),
		"weaponPoses": _poses(s.weaponPoses, _section_at("identified", "weaponPoses")),
		"reversing": row.f("reversing") == 1.0,
		"smoking": row.f("smoking") == 1.0,
	}

func _contact(row: Row) -> Dictionary:
	var heard: Array = []
	var round_kinds: Array = _layout.roundKinds
	for k in round_kinds.size():
		var bit := (int(row.f("heardLow")) >> k) if k < HEARD_WORD_BITS else (int(row.f("heardHigh")) >> (k - HEARD_WORD_BITS))
		if bit & 1:
			heard.append(round_kinds[k])
	return {
		"id": row.f("id"),
		"source": _word("contactSources", row.f("source")),
		"center": [row.f("x"), row.f("y"), row.f("z")],
		"layer": _word("layers", row.f("layer")),
		"aloft": row.f("aloft") == 1.0,
		"radius": row.f("radius"),
		"evidenceTick": row.f("evidenceTick"),
		"expiresTick": row.f("expiresTick"),
		"kind": null if row.f("kind") < 0.0 else _word("unitKinds", row.f("kind")),
		"heard": heard,
	}

func _audible(row: Row) -> Dictionary:
	return {
		"listener": row.f("listener"),
		"category": _word("soundCategories", row.f("category")),
		"sector": row.f("sector"),
		"band": _word("soundBands", row.f("band")),
		"moving": row.f("moving") == 1.0,
	}

func _known_prop(row: Row) -> Dictionary:
	return {
		"kind": _word("propKinds", row.f("kind")),
		"center": [row.f("x"), row.f("y")],
		"yaw": row.f("yaw"),
		"half": [row.f("hx"), row.f("hy"), row.f("hz")],
		"baseZ": row.f("baseZ"),
		"replaces": _limbs(row, "replaces"),
		"id": _limbs(row, "id"),
		"building": _limbs(row, "building"),
		"structureOwner": _limbs(row, "structureOwner"),
		"authoredProp": _limbs(row, "authoredProp"),
		"destroyed": row.f("destroyed") == 1.0,
		"wreckOf": null if row.f("wreckOf") < 0.0 else _word("unitKinds", row.f("wreckOf")),
	}

func _projectile(row: Row) -> Dictionary:
	var bounce := _section_at("projectiles", "ricochets")
	var hit = _word("hitKinds", row.f("hit"))
	return {
		"path": row.sections.path,
		"ricochets": row.sections.ricochets.map(func(p): return {"point": p[bounce.point], "normal": [p[bounce.nx], p[bounce.ny], p[bounce.nz]]}),
		"own": row.f("own") == 1.0,
		"kind": _word("roundKinds", row.f("kind")),
		"shooterMember": _limbs(row, "shooter"),
		"hit": hit,
		"impactNormal": null if hit == "none" else [row.f("nx"), row.f("ny"), row.f("nz")],
	}

func _blast(row: Row) -> Dictionary:
	return {"point": [row.f("x"), row.f("y"), row.f("z")], "radius": row.f("radius"), "kind": _word("roundKinds", row.f("kind"))}

func _guided(row: Row) -> Dictionary:
	return {
		"id": _limbs(row, "id"),
		"position": [row.f("x"), row.f("y"), row.f("z")],
		"point": [row.f("px"), row.f("py"), row.f("pz")],
		"supported": row.f("supported") == 1.0,
	}

func _corpse(row: Row) -> Dictionary:
	return {
		"position": [row.f("x"), row.f("y"), row.f("z")],
		"own": row.f("own") == 1.0,
		"soldier": _limbs(row, "soldier"),
		"kind": _word("unitKinds", row.f("kind")),
		"slot": row.f("slot"),
		"yaw": row.f("yaw"),
	}

func _fallen_body(row: Row) -> Dictionary:
	return {
		"prop": _limbs(row, "prop"),
		"at": [row.f("x"), row.f("y")],
		"toward": [row.f("towardX"), row.f("towardY")],
		"tick": row.f("tick"),
	}

func _crash(row: Row) -> Dictionary:
	return {
		"id": row.f("id"),
		"own": row.f("own") == 1.0,
		"kind": _word("unitKinds", row.f("kind")),
		"position": [row.f("x"), row.f("y"), row.f("z")],
		"yaw": row.f("yaw"),
		"pitch": row.f("pitch"),
		"roll": row.f("roll"),
	}

func _skirmish(header: Dictionary, rows: Dictionary) -> Dictionary:
	var objectives: Array = []
	for row: Row in rows.objectives:
		objectives.append({
			"id": _word("objectiveIds", row.f("id")),
			"center": [row.f("x"), row.f("y")],
			"radiusM": row.f("radius"),
			"owner": null if row.f("owner") < 0.0 else SIDES[int(row.f("owner"))],
			"capturing": null if row.f("capturing") < 0.0 else SIDES[int(row.f("capturing"))],
			"captureProgress": row.f("captureProgress"),
			"contested": row.f("contested") != 0.0,
		})
	var pending: Array = []
	for row: Row in rows.pendingPurchases:
		pending.append({
			"id": row.f("idLo") + row.f("idHi") * _limb,
			"kind": _word("unitKinds", row.f("kind")),
			"destination": [row.f("x"), row.f("y")],
			"confirmedTick": row.f("confirmedLo") + row.f("confirmedHi") * _limb,
			"blocked": row.f("blocked") != 0.0,
		})
	var report = null
	if header.reportTotalsCount > 0.0 or header.reportEventCount > 0.0:
		report = {"totals": rows.reportTotals.map(_report_totals), "events": rows.reportEvents.map(_report_event)}
	return {
		"phase": SKIRMISH_PHASES[int(header.skirmishPhase)],
		"ready": [header.readyBlue != 0.0, header.readyRed != 0.0],
		"preparationRemainingS": header.preparationRemainingS,
		"credits": header.credits,
		"scores": [header.scoreBlue, header.scoreRed],
		"result": null if header.matchResult < 0.0 else MATCH_RESULTS[int(header.matchResult)],
		"objectives": objectives,
		"occupiedSlots": header.occupiedSlots,
		"maxUnits": header.maxUnits,
		"pending": pending,
		"report": report,
	}

func _report_totals(row: Row) -> Dictionary:
	return {"spawned": row.f("spawned"), "spent": row.f("spent"), "killed": row.f("killed"), "refunded": row.f("refunded")}

## A match report event: a spawn, a refund, or a kill with its killer (null
## when unknown).
func _report_event(row: Row) -> Dictionary:
	var kind := row.f("kind")
	var event := {
		"tick": _limbs(row, "tick"),
		"side": _layout.ground.sides[int(row.f("side"))],
		"unitId": _limbs(row, "unitId"),
		"unitKind": _word("unitKinds", row.f("unitKind")),
	}
	if kind == 0.0 or kind == 1.0:
		event.kind = "spawn" if kind == 0.0 else "refund"
		event.amount = row.f("amount")
		return event
	event.kind = "kill"
	event.victimCost = row.f("amount")
	var other := row.f("otherSide")
	event.killer = null if other < 0.0 else {
		"side": _layout.ground.sides[int(other)],
		"unitId": _limbs(row, "otherId"),
		"unitKind": _word("unitKinds", row.f("otherKind")),
		"cost": row.f("otherAmount"),
	}
	return event

static func _is_int(value: float) -> bool:
	return is_finite(value) and value == floorf(value) and absf(value) <= 9007199254740991.0

## One row of a group: its fields by name, and its sections' points.
class Row:
	var payload: PackedFloat32Array
	var base: int
	var at: Dictionary
	var sections := {}
	func _init(source: PackedFloat32Array, offset: int, fields: Dictionary) -> void:
		payload = source
		base = offset
		at = fields
	func f(name: String) -> float:
		return payload[base + int(at[name])]

## Reads lsb-first bit fields from raw u32 carrier words; any overrun or bad
## form clears `valid`.
class PackedReader:
	var words: PackedInt32Array
	var bit := 0
	var valid := true
	func _init(source: PackedInt32Array) -> void:
		words = source
	func read(count: int) -> int:
		if not valid or bit + count > words.size() * 32:
			valid = false
			return 0
		var index := bit / 32
		var shift := bit % 32
		var value := (words[index] & 0xffffffff) >> shift
		if shift + count > 32:
			value |= (words[index + 1] & 0xffffffff) << (32 - shift)
		bit += count
		return value & ((1 << count) - 1)
	## A canonical unsigned LEB128 of at most four bytes.
	func integer() -> int:
		var value := 0
		for byte in 4:
			var part := read(8)
			if not valid:
				return 0
			value += (part & 127) << (byte * 7)
			if part < 128:
				if byte != 0 and part == 0:
					valid = false
				return value
		valid = false
		return 0
	## Zero padding to the next carrier word, and no more.
	func finish() -> void:
		if not valid or words.size() * 32 - bit > 31:
			valid = false
			return
		while bit < words.size() * 32:
			if read(1) != 0:
				valid = false
				return
