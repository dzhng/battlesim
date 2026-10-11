extends RefCounted

## Reader for the manifest's `units` and `modelCounts` (the browser's
## `web/src/battle/benchmark/unitManifest.ts`): the pose driver's feel, each
## fielded unit type's facts and appearance sets, each appearance's joints
## and clips (a skinned body) or nodes and rig (a vehicle), the clip tables
## as hashed artifacts, and what each plate posed. Strict: no defaults, no
## unknown fields.

const StrictJson = preload("res://core/strict_json.gd")

const UNITS_KEYS := ["pose", "leanHold", "detail", "tints", "types", "appearances", "tables"]
const TYPE_KEYS := ["hull", "airborne", "roles", "mounts", "draws"]
const MOUNT_KEYS := ["weapon", "on", "activePose"]
const DRAWS_KEYS := ["hull", "slots", "operated"]
const SKINNED_KEYS := ["kind", "joints", "table", "clips", "farPose", "corpsePoses"]
const ARTICULATED_KEYS := ["kind", "nodes", "rig"]
const NODE_KEYS := ["name", "parent", "bind"]
const CLIP_KEYS := ["offset", "frames", "duration", "rate", "loop", "stride"]
const RIG_KEYS := ["turret", "gun", "hmg", "hmgGun", "pitch", "stroke", "wheels", "tracks", "rotors", "deploy"]
const COUNT_KINDS := ["soldiers", "vehicles", "aircraft", "corpses", "fading"]
## `presentation.pose`, as `validatePoseFeel` requires it.
const FEEL_KEYS := {
	"gait": ["walk_mps", "run_mps", "crawl_mps", "fade_s", "facing_mps", "turn_rad_s"],
	"rest": ["turn_rad", "tempo", "watch_every", "settle_s"],
	"stance": ["fire_prone", "fire_stand", "pinned_kneel", "fall_back"],
	"lean": ["out_s", "back_s"],
	"mount": ["gun_elevation_rad_s", "hmg_elevation_rad_s", "recoil_m", "recoil_return_s"],
	"rotor": ["tip_mps"],
	"attitude": ["gain", "max_deg", "smoothing_s"],
	"corpses": ["max", "fade_s", "sink_m"],
}

## An empty string when `units` is a section the units layer can follow, or
## the first problem.
static func error(units: Variant) -> String:
	var shape := StrictJson.exact_keys(units, UNITS_KEYS, "units")
	if not shape.is_empty():
		return shape
	shape = _feel_error(units.pose)
	if not shape.is_empty():
		return shape
	if not StrictJson.is_number(units.leanHold) or float(units.leanHold) < 0.0:
		return "units.leanHold must be seconds"
	shape = StrictJson.exact_keys(units.detail, ["lod_px", "impostor_px"], "units.detail")
	if not shape.is_empty():
		return shape
	if not StrictJson.is_vector(units.detail.lod_px, 3) or not StrictJson.is_number(units.detail.impostor_px):
		return "units.detail needs three tier sizes and a card size"
	shape = StrictJson.exact_keys(units.tints, ["blue", "red"], "units.tints")
	if not shape.is_empty():
		return shape
	if not StrictJson.is_vector(units.tints.blue, 3) or not StrictJson.is_vector(units.tints.red, 3):
		return "units.tints needs an rgb per side"
	if typeof(units.tables) != TYPE_ARRAY:
		return "units.tables must be an array"
	var tables := {}
	for table in units.tables:
		shape = StrictJson.exact_keys(table, ["name", "sha256", "words", "layout"], "units table")
		if not shape.is_empty():
			return shape
		if not StrictJson.is_name(table.name) or not StrictJson.is_hex(table.sha256, 64) or not StrictJson.is_integer(table.words):
			return "units table %s needs a name, a sha256 and a word count" % table.name
		tables[table.name] = true
	if typeof(units.appearances) != TYPE_DICTIONARY:
		return "units.appearances must be an object"
	for name in units.appearances:
		shape = _appearance_error(units.appearances[name], tables)
		if not shape.is_empty():
			return "appearance %s: %s" % [name, shape]
	if typeof(units.types) != TYPE_DICTIONARY:
		return "units.types must be an object"
	for kind in units.types:
		shape = _type_error(units.types[kind], units.appearances)
		if not shape.is_empty():
			return "unit type %s: %s" % [kind, shape]
	return ""

## An empty string when `counts` holds one row of whole counts per plate.
static func counts_error(counts: Variant) -> String:
	if typeof(counts) != TYPE_ARRAY:
		return "modelCounts must be an array"
	for row in counts:
		var shape := StrictJson.exact_keys(row, ["plate"] + COUNT_KINDS, "model count")
		if not shape.is_empty():
			return shape
		if not StrictJson.is_name(row.plate) or not COUNT_KINDS.all(func(k): return StrictJson.is_integer(row[k]) and int(row[k]) >= 0):
			return "model count %s needs a plate and whole counts" % row.plate
	return ""

static func _feel_error(feel: Variant) -> String:
	if typeof(feel) != TYPE_DICTIONARY:
		return "units.pose must be an object"
	for group in FEEL_KEYS:
		if typeof(feel.get(group)) != TYPE_DICTIONARY:
			return "units.pose.%s must be an object" % group
		for key in FEEL_KEYS[group]:
			var value = feel[group].get(key)
			if not (StrictJson.is_number(value) or StrictJson.is_vector(value, 2)):
				return "units.pose.%s.%s must be a number" % [group, key]
	if typeof(feel.get("gauge")) != TYPE_DICTIONARY:
		return "units.pose.gauge must be an object"
	if not (float(feel.gait.run_mps) > float(feel.gait.walk_mps) and float(feel.gait.fade_s) > 0.0 and float(feel.corpses.fade_s) > 0.0 and int(feel.corpses.max) >= 1 and int(feel.rest.watch_every) >= 3):
		return "units.pose is out of range"
	return ""

static func _hierarchy_error(nodes: Variant) -> String:
	if typeof(nodes) != TYPE_ARRAY or nodes.is_empty():
		return "a non-empty hierarchy"
	for i in nodes.size():
		var node = nodes[i]
		var shape := StrictJson.exact_keys(node, NODE_KEYS, "node")
		if not shape.is_empty():
			return shape
		if not StrictJson.is_name(node.name) or not StrictJson.is_integer(node.parent) or int(node.parent) < -1 or int(node.parent) >= i:
			return "node %d needs a name and an earlier parent" % i
		shape = StrictJson.exact_keys(node.bind, ["t", "r", "s"], "bind")
		if not shape.is_empty():
			return shape
		if not (StrictJson.is_vector(node.bind.t, 3) and StrictJson.is_vector(node.bind.r, 4) and StrictJson.is_vector(node.bind.s, 3)):
			return "node %s's bind needs t, r and s" % node.name
	return ""

static func _pose_ok(pose: Variant) -> bool:
	return StrictJson.exact_keys(pose, ["clip", "phase"], "pose").is_empty() and StrictJson.is_name(pose.clip) and StrictJson.is_number(pose.phase)

static func _appearance_error(appearance: Variant, tables: Dictionary) -> String:
	if typeof(appearance) != TYPE_DICTIONARY:
		return "must be an object"
	if appearance.get("kind") == "skinned":
		var shape := StrictJson.exact_keys(appearance, SKINNED_KEYS, "skinned appearance")
		if shape.is_empty():
			shape = _hierarchy_error(appearance.joints)
		if not shape.is_empty():
			return shape
		if not tables.has(appearance.table):
			return "its clip table %s is not listed" % appearance.table
		if typeof(appearance.clips) != TYPE_DICTIONARY:
			return "clips must be an object"
		for name in appearance.clips:
			var clip = appearance.clips[name]
			shape = StrictJson.exact_keys(clip, CLIP_KEYS, "clip " + name)
			if not shape.is_empty():
				return shape
			if not (StrictJson.is_integer(clip.offset) and StrictJson.is_integer(clip.frames) and int(clip.frames) > 0 and StrictJson.is_number(clip.duration) and float(clip.duration) > 0.0 and StrictJson.is_number(clip.rate) and typeof(clip.loop) == TYPE_BOOL and (clip.stride == null or StrictJson.is_number(clip.stride))):
				return "clip %s needs an offset, frames, a duration, a rate, loop and a stride or null" % name
		if not _pose_ok(appearance.farPose) or typeof(appearance.corpsePoses) != TYPE_ARRAY or not appearance.corpsePoses.all(_pose_ok):
			return "a far pose and corpse poses"
		return ""
	if appearance.get("kind") == "articulated":
		var shape := StrictJson.exact_keys(appearance, ARTICULATED_KEYS, "articulated appearance")
		if shape.is_empty():
			shape = _hierarchy_error(appearance.nodes)
		if shape.is_empty():
			shape = StrictJson.exact_keys(appearance.rig, RIG_KEYS, "rig")
		if not shape.is_empty():
			return shape
		var rig: Dictionary = appearance.rig
		var count: int = appearance.nodes.size()
		var node := func(v): return StrictJson.is_integer(v) and int(v) >= -1 and int(v) < count
		if not [rig.turret, rig.gun, rig.hmg, rig.hmgGun].all(node):
			return "rig nodes must index its nodes"
		if not (StrictJson.is_vector(rig.pitch.get("gun"), 2) and StrictJson.is_vector(rig.pitch.get("hmg"), 2)) or not (rig.stroke == null or StrictJson.is_number(rig.stroke)):
			return "rig pitch limits and stroke"
		for list in ["wheels", "tracks", "rotors", "deploy"]:
			if typeof(rig[list]) != TYPE_ARRAY or not rig[list].all(func(part): return typeof(part) == TYPE_DICTIONARY and node.call(part.get("node"))):
				return "rig %s must name its nodes" % list
		return ""
	return "kind must be skinned or articulated"

static func _type_error(type: Variant, appearances: Dictionary) -> String:
	var shape := StrictJson.exact_keys(type, TYPE_KEYS, "type")
	if not shape.is_empty():
		return shape
	if not (type.hull == null or (StrictJson.exact_keys(type.hull, ["halfTrack"], "hull").is_empty() and StrictJson.is_number(type.hull.halfTrack))):
		return "hull must be null or a half track"
	if typeof(type.airborne) != TYPE_BOOL or typeof(type.roles) != TYPE_ARRAY or not type.roles.all(StrictJson.is_name):
		return "airborne and roles"
	if typeof(type.mounts) != TYPE_ARRAY or type.mounts.size() != type.roles.size():
		return "one mount per role"
	for mount in type.mounts:
		shape = StrictJson.exact_keys(mount, MOUNT_KEYS, "mount")
		if not shape.is_empty():
			return shape
		if not StrictJson.is_name(mount.weapon) or not (mount.on == null or StrictJson.is_integer(mount.on)) or not (mount.activePose == null or _pose_ok(mount.activePose)):
			return "mounts need a weapon, a carrier or null and a hold or null"
	shape = StrictJson.exact_keys(type.draws, ["blue", "red"], "draws")
	if not shape.is_empty():
		return shape
	var known := func(list): return typeof(list) == TYPE_ARRAY and list.all(func(name): return appearances.has(name))
	for side in ["blue", "red"]:
		var draws = type.draws[side]
		shape = StrictJson.exact_keys(draws, DRAWS_KEYS, "draws." + side)
		if not shape.is_empty():
			return shape
		if not known.call(draws.hull) or typeof(draws.slots) != TYPE_ARRAY or not draws.slots.all(known) or typeof(draws.operated) != TYPE_ARRAY:
			return "draws.%s must name listed appearances" % side
		for operated in draws.operated:
			if operated != null and not (StrictJson.exact_keys(operated, ["active", "carried"], "operated").is_empty() and known.call(operated.active) and known.call(operated.carried)):
				return "draws.%s.operated must name listed appearances" % side
	return ""
