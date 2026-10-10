extends RefCounted

## Reader for one scene's `native-visual-presentation` manifest, written by the
## browser capture run beside its capture. The manifest names the battle (map,
## encounter, seed and the sha256 of the map and capture bytes), the viewport,
## the lens and the layers the browser drew. Native renders only a battle whose
## files match that identity exactly.

const Capture = preload("res://capture/presentation_capture.gd")

const SCHEMA := "native-visual-presentation"
const ROOT_KEYS := ["schema", "identity", "viewport", "lens", "layers", "omissions"]
const IDENTITY_KEYS := ["scene", "map", "encounter", "seed", "mapHash", "captureHash"]
const VIEWPORT_KEYS := ["width", "height", "scale"]
const LENS_KEYS := ["fovY", "near", "depth", "far"]
const OMISSION_KEYS := ["owner", "reason", "sourceId"]

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
	return ""

static func _is_sha256(value: Variant) -> bool:
	if typeof(value) != TYPE_STRING or String(value).length() != 64:
		return false
	for character in String(value):
		if not "0123456789abcdef".contains(character):
			return false
	return true
