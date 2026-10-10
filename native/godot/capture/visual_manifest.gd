extends RefCounted

## Reader for one scene's `native-visual-presentation` manifest, written by the
## browser capture run beside its capture. The manifest names the battle (map,
## encounter, seed and the sha256 of the map and capture bytes), the viewport,
## the lens, the light, the shader tuning tables, the layers the browser drew,
## the terrain and grass tables (as hashed artifacts beside the manifest) and
## the grass counts per plate.
## Native renders only a battle whose files match that identity exactly.

const Capture = preload("res://capture/presentation_capture.gd")
const Light = preload("res://environment/light.gd")

const SCHEMA := "native-visual-presentation"
const ROOT_KEYS := ["schema", "identity", "viewport", "lens", "light", "tuning", "layers", "omissions", "terrain", "grass", "grassCounts"]
const IDENTITY_KEYS := ["scene", "map", "encounter", "seed", "mapHash", "captureHash"]
const VIEWPORT_KEYS := ["width", "height", "scale"]
const LENS_KEYS := ["fovY", "near", "depth", "far"]
const OMISSION_KEYS := ["owner", "reason", "sourceId"]
const ARTIFACT_KEYS := ["name", "sha256", "words", "layout"]
const GRASS_KEYS := ["tables", "constants", "kinds"]
const GRASS_COUNT_KEYS := ["plate", "cells"]

## Reads `<dir>/manifest.json` and `<dir>/capture.json`, and the map named by
## the manifest from `<maps_dir>/<map>/map.json`.
## Returns `{ok: true, manifest, capture, map_path}` or
## `{ok: false, blocker: {name, detail}}`.
static func load(dir: String, maps_dir: String) -> Dictionary:
	var manifest_path := dir.path_join("manifest.json")
	if not FileAccess.file_exists(manifest_path):
		return _refuse("manifest", "missing " + manifest_path)
	var manifest = JSON.parse_string(FileAccess.get_file_as_string(manifest_path))
	var shape := _manifest_error(manifest)
	if not shape.is_empty():
		return _refuse("manifest", "%s: %s" % [manifest_path, shape])
	var identity: Dictionary = manifest.identity

	var capture_path := dir.path_join("capture.json")
	var map_path := maps_dir.path_join(identity.map).path_join("map.json")
	for path in [capture_path, map_path]:
		if not FileAccess.file_exists(path):
			return _refuse("identity", "missing " + path)
	var mismatches: Array[String] = []
	if FileAccess.get_sha256(capture_path) != identity.captureHash:
		mismatches.append("captureHash differs from %s" % capture_path)
	if FileAccess.get_sha256(map_path) != identity.mapHash:
		mismatches.append("mapHash differs from %s" % map_path)
	if not mismatches.is_empty():
		return _refuse("identity", "; ".join(mismatches))

	var capture := Capture.parse(FileAccess.get_file_as_string(capture_path))
	if not capture.valid:
		return _refuse("capture", "%s: %s" % [capture_path, capture.reason])
	var workload: Dictionary = capture.capture.workload
	for field in ["scene", "map", "encounter", "seed"]:
		if workload[field] != identity[field]:
			mismatches.append("%s is %s in the manifest and %s in the capture" % [field, identity[field], workload[field]])
	if not mismatches.is_empty():
		return _refuse("identity", "; ".join(mismatches))
	return {"ok": true, "manifest": manifest, "capture": capture, "map_path": map_path}

## Reads a group's hashed artifacts (`<dir>/<group>-<name>.u32`), each
## refused unless its bytes match its sha256 and word count. Returns
## `{ok: true, tables: {name: {words, layout}}}` or `{ok: false, blocker}`.
static func read_tables(dir: String, artifacts: Array, group: String) -> Dictionary:
	var tables := {}
	for artifact in artifacts:
		var path := dir.path_join("%s-%s.u32" % [group, artifact.name])
		if not FileAccess.file_exists(path):
			return _refuse("artifact", "missing " + path)
		if FileAccess.get_sha256(path) != artifact.sha256:
			return _refuse("artifact", "%s differs from its manifest sha256" % path)
		var words := FileAccess.get_file_as_bytes(path)
		if words.size() != int(artifact.words) * 4:
			return _refuse("artifact", "%s holds %d bytes, not %d words" % [path, words.size(), int(artifact.words)])
		tables[artifact.name] = {"words": words, "layout": artifact.layout}
	return {"ok": true, "tables": tables}

static func _refuse(name: String, detail: String) -> Dictionary:
	return {"ok": false, "blocker": {"name": name, "detail": detail}}

static func _manifest_error(manifest: Variant) -> String:
	var shape := Capture.exact_keys(manifest, ROOT_KEYS, "manifest")
	if not shape.is_empty():
		return shape
	if typeof(manifest.schema) != TYPE_STRING or manifest.schema != SCHEMA:
		return "schema must be %s, not %s" % [SCHEMA, manifest.schema]
	for section in [["identity", IDENTITY_KEYS], ["viewport", VIEWPORT_KEYS], ["lens", LENS_KEYS]]:
		shape = Capture.exact_keys(manifest[section[0]], section[1], section[0])
		if not shape.is_empty():
			return shape
	var identity: Dictionary = manifest.identity
	for field in ["scene", "map", "encounter"]:
		if not Capture.is_name(identity[field]):
			return "identity.%s must be a non-empty string" % field
	if not Capture.is_integer(identity.seed):
		return "identity.seed must be an integer"
	for field in ["mapHash", "captureHash"]:
		if not _is_sha256(identity[field]):
			return "identity.%s must be a lowercase hex sha256" % field
	var viewport: Dictionary = manifest.viewport
	for field in ["width", "height"]:
		if not Capture.is_integer(viewport[field]) or int(viewport[field]) <= 0:
			return "viewport.%s must be a positive integer" % field
	if not Capture.is_number(viewport.scale) or float(viewport.scale) != 1.0:
		return "viewport.scale must be 1"
	var lens: Dictionary = manifest.lens
	if not Capture.is_number(lens.fovY) or float(lens.fovY) <= 0.0 or float(lens.fovY) >= PI:
		return "lens.fovY must be an angle between 0 and pi"
	if not Capture.is_number(lens.near) or float(lens.near) <= 0.0:
		return "lens.near must be a positive number"
	if not (typeof(lens.depth) == TYPE_STRING and lens.depth == "reverse-z" and typeof(lens.far) == TYPE_STRING and lens.far == "infinite"):
		return "lens must be reverse-z with an infinite far plane"
	shape = Light.error(manifest.light)
	if not shape.is_empty():
		return shape
	if typeof(manifest.layers) != TYPE_ARRAY:
		return "layers must be an array"
	for at in manifest.layers.size():
		var layer = manifest.layers[at]
		if not Capture.is_name(layer) or manifest.layers.find(layer) != at:
			return "layers must be unique non-empty names"
	if typeof(manifest.omissions) != TYPE_ARRAY:
		return "omissions must be an array"
	for omission in manifest.omissions:
		shape = Capture.exact_keys(omission, OMISSION_KEYS, "omission")
		if not shape.is_empty():
			return shape
		for field in OMISSION_KEYS:
			if not Capture.is_name(omission[field]):
				return "omission.%s must be a non-empty string" % field
	# Tuning, terrain and grass are checked for shape only until the slices
	# that read them (05a to 06b) decode them.
	shape = Capture.exact_keys(manifest.tuning, ["terrain", "grass"], "tuning")
	if not shape.is_empty():
		return shape
	if typeof(manifest.tuning.terrain) != TYPE_DICTIONARY or typeof(manifest.tuning.grass) != TYPE_DICTIONARY:
		return "tuning.terrain and tuning.grass must be objects"
	shape = _artifacts_error(manifest.terrain, "terrain")
	if not shape.is_empty():
		return shape
	shape = Capture.exact_keys(manifest.grass, GRASS_KEYS, "grass")
	if not shape.is_empty():
		return shape
	shape = _artifacts_error(manifest.grass.tables, "grass.tables")
	if not shape.is_empty():
		return shape
	if typeof(manifest.grass.constants) != TYPE_DICTIONARY:
		return "grass.constants must be an object"
	if typeof(manifest.grass.kinds) != TYPE_ARRAY or manifest.grass.kinds.is_empty() or not manifest.grass.kinds.all(func(kind): return Capture.is_name(kind)):
		return "grass.kinds must be a non-empty list of names"
	if typeof(manifest.grassCounts) != TYPE_ARRAY:
		return "grassCounts must be an array"
	for count in manifest.grassCounts:
		shape = Capture.exact_keys(count, GRASS_COUNT_KEYS, "grass count")
		if not shape.is_empty():
			return shape
		if not Capture.is_name(count.plate) or typeof(count.cells) != TYPE_ARRAY or not count.cells.all(func(cell): return Capture.is_integer(cell) and int(cell) >= 0):
			return "grass count %s needs a plate and non-negative cell counts" % count.plate
	return ""

## An empty string when `artifacts` is a list of hashed-artifact records
## `{name, sha256, words, layout}`.
static func _artifacts_error(artifacts: Variant, what: String) -> String:
	if typeof(artifacts) != TYPE_ARRAY:
		return "%s must be an array" % what
	for artifact in artifacts:
		var shape := Capture.exact_keys(artifact, ARTIFACT_KEYS, what + " artifact")
		if not shape.is_empty():
			return shape
		if not Capture.is_name(artifact.name) or not _is_sha256(artifact.sha256) or not Capture.is_integer(artifact.words) or int(artifact.words) < 0 or typeof(artifact.layout) != TYPE_DICTIONARY:
			return "%s artifact %s needs a name, a sha256, a word count and a layout" % [what, artifact.name]
	return ""

static func _is_sha256(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING or String(value).length() != 64:
		return false
	for character in String(value):
		if not "0123456789abcdef".contains(character):
			return false
	return true
