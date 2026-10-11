extends Node3D

## A rough native placement of the saved map, kept until slice 13 ports the
## browser's owners: what stands where for buildings (`buildings`), props
## (`props`) and observed units (`models`), each layer drawn by its own
## `model_batches.gd`. Every category is reported as a budget that names its
## approximation and how many rows it drew and dropped. Every position and yaw
## goes through `Coords`; what has no height of its own stands on the ground's
## height field.

const Staging = preload("res://assets/staging.gd")
const ModelBatches = preload("res://world/model_batches.gd")
const Coords = preload("res://core/coords.gd")
const StrictJson = preload("res://core/strict_json.gd")

const OWNER := "native/legacy_map_layer"
## Map rows farther than this from every frame target are not drawn.
const REACH_M := 1000.0
## A frame target this close to an earlier one adds no reach of its own.
const TARGET_SPACING_M := 25.0

var _map: Dictionary
var _region := ""
var _centers: Array[Vector2] = []
## Kind name -> number of distinct units of that kind in the capture.
var _unit_kinds := {}
var _appearances := {}
var _units_catalog := {}
var _soldiers := {}
## Repository path -> staged resource path.
var _staged := {}
var _scenes := {}
## Kit and shell token -> its appearance in the buildings' batches, or -1.
var _shells := {}
var _units: ModelBatches
## Unit kind name -> its appearance in the units' batches.
var _unit_appearances := {}
var _side_color: Color
## The detail rules of `model_batches.gd`: a model's tier by its projected
## size, a building's by a metre's.
var _model_detail: Dictionary
var _building_detail: Dictionary
var _sun: Vector3
var _max_far_m: float
var _contact_mesh: CylinderMesh
## The ground's `height_field.gd`.
var _heights

## `targets` are the sim-space (x, y) frame targets the run draws; `samples`
## the capture's decoded samples, whose units it draws.
func _init(map: Dictionary, targets: Array, samples: Array, heights) -> void:
	name = "LegacyMapLayer"
	_map = map
	_heights = heights
	_region = String(map.get("regional_family", "")).to_lower()
	var ids := {}
	for sample in samples:
		for unit in sample.units:
			if not ids.has(unit.kindName):
				ids[unit.kindName] = {}
			ids[unit.kindName][unit.id] = true
	for kind in ids:
		_unit_kinds[kind] = ids[kind].size()
	for target in targets:
		if not _centers.any(func(center): return center.distance_to(target) < TARGET_SPACING_M):
			_centers.append(target)
	_load_catalogs()

## The repository assets the listed layers draw.
func required_assets(layers: Array) -> Array:
	var needed := {}
	if layers.has("buildings"):
		for building in _rows("buildings"):
			if _valid_building(building) and _near(_building_position(building)) and not _shell_token(String(building.template_id)).is_empty():
				needed[_kit_source(String(building.template_id))] = true
	if layers.has("props"):
		for prop in _rows("props"):
			if _valid_prop(prop) and _near(_prop_position(prop)):
				needed[_prop_source(String(prop.kind))] = true
	if layers.has("models"):
		for kind in _unit_kinds:
			needed[_unit_source(kind)] = true
	needed.erase("")
	return needed.keys()

## Builds the listed layers from staged assets, reporting every category;
## `light` is the manifest's, whose sun the models' shadows fall from.
func build(layers: Array, staged: Dictionary, side: String, light: Dictionary, report) -> void:
	_staged = staged
	_sun = Coords.sim_array_to_godot(light.sunDirection)
	_max_far_m = float(light.cascades.max_far_m)
	if layers.has("buildings"):
		_build_buildings(report)
	if layers.has("props"):
		_build_props(report)
	if layers.has("models"):
		_build_units(side, report)

## Chooses what `camera`, drawing into a viewport `size` pixels big, draws of
## every layer this frame.
func prepare(camera: Camera3D, size: Vector2) -> void:
	for batches in get_children():
		batches.prepare(camera, size)

## Shows the units of one decoded publication, and no other unit.
func show_units(units: Array) -> void:
	if _units == null:
		return
	var placements := []
	for unit in units:
		if not _unit_appearances.has(unit.kindName):
			_unit_appearances[unit.kindName] = _unit_appearance(unit.kindName)
		placements.append([_unit_appearances[unit.kindName], Transform3D(Coords.yaw_basis(float(unit.yaw)), Coords.sim_array_to_godot(unit.position))])
	_units.show_movers(placements)

# Buildings --------------------------------------------------------------------

func _build_buildings(report) -> void:
	var batches := _batches("Buildings")
	var boxes: Array[Transform3D] = []
	var admitted := 0
	var near := 0
	var malformed := 0
	for building in _rows("buildings"):
		if not _valid_building(building):
			malformed += 1
			continue
		admitted += 1
		var position := _building_position(building)
		if not _near(position):
			continue
		near += 1
		var translation: Array = building.frame.translation
		var yaw := float(building.frame.get("yaw", 0.0))
		var shell := _shell(String(building.template_id), batches)
		if shell >= 0:
			batches.add_static(shell, Transform3D(Coords.yaw_basis(yaw), Coords.sim_array_to_godot(translation)))
		else:
			boxes.append(Transform3D(Coords.yaw_basis(yaw).scaled_local(Vector3(18.0, 8.0, 18.0)), Coords.sim_array_to_godot(translation) + Vector3(0.0, 4.0, 0.0)))
	if not boxes.is_empty():
		var box := _stand_in(batches, _box_mesh(Color("7b7d7b"), 0.8), _building_detail)
		for placement in boxes:
			batches.add_static(box, placement)
	batches.seal()
	report.add_budget(OWNER, "approximation: kit shells picked by template token with no facade modules, an open tier drawn over every coarser tier (until slice 13), within %d m of a frame target" % REACH_M, "map.buildings", admitted, near, admitted - near)
	if not boxes.is_empty():
		report.add_budget(OWNER, "approximation: an 18x8x18 m box where no kit shell matches the template", "map.buildings/boxes", boxes.size(), boxes.size(), 0)
	_report_malformed(report, "map.buildings", malformed)

## The appearance of a template's kit shell in `batches`, or -1.
func _shell(template_id: String, batches: ModelBatches) -> int:
	var token := _shell_token(template_id)
	var kit := _kit_source(template_id)
	if token.is_empty() or not _staged.has(kit):
		return -1
	var key := kit + "|" + token
	if not _shells.has(key):
		var catalog := _scene(kit).instantiate()
		var shell := catalog.get_node_or_null(NodePath(token))
		_shells[key] = batches.add_appearance(shell, _building_detail) if shell != null else -1
		catalog.free()
	return _shells[key]

## The catalog kit for a template's family, preferring the map's regional kit.
func _kit_source(template_id: String) -> String:
	var lower := template_id.to_lower()
	var family := ""
	for rule in [["apartment", "apartments"], ["farmstead", "farmsteads"], ["shed", "industry"], ["warehouse", "industry"], ["works", "industry"], ["depot", "industry"], ["tower", "towers"], ["home", "homes"], ["terrace", "homes"], ["townhouse", "homes"], ["shop", "homes"]]:
		if lower.contains(rule[0]):
			family = rule[1]
			break
	if family.is_empty():
		return ""
	for key in ["city_kit_%s_%s" % [family, _region], "city_kit_%s_%s" % [_region, family], "city_kit_%s" % family]:
		if _appearances.has(key):
			return _entry_source(_appearances[key])
	return ""

## The kit node for each template, by the first fragment the template id contains.
const SHELL_TOKENS := [
	["slab-35x11", "slab_35x11_4f_shell"],
	["slab-47x11", "slab_47x11_5f_shell"],
	["slab-59x14", "slab_59x14_6f_shell"],
	["slab-53x14", "slab_53x14_8f_shell"],
	["slab-47x14", "slab_47x14_6f_shell"],
	["slab-35x14", "slab_35x14_8f_shell"],
	["point-20x20", "point_20x20_7f_shell"],
	["block-u", "block_u_5f_shell"],
	["block-court", "block_court_6f_shell"],
	["corner-shop", "corner_shop_3f_shell"],
	["home-8x11", "home_8x11_1f_shell"],
	["home-9x9", "home_9x9_2f_shell"],
	["home-10x8", "home_10x8_1f_shell"],
	["home-12x9", "home_12x9_2f_shell"],
	["home-ell", "home_ell_2f_shell"],
	["townhouse", "townhouse_2f_shell"],
	["shops-4x3", "shops_4x3f_unit"],
	["terrace-3x2", "terrace_3x2f_unit"],
	["terrace-5x3", "terrace_5x3f_unit"],
	["farmstead-long", "farm_long_shell"],
	["farmstead-small", "farm_small_shell"],
	["farmstead-yard", "farm_yard_shell"],
	["depot", "depot_shell"],
	["shed", "shed_shell"],
	["warehouse", "span_shell"],
	["works", "works_shell"],
]

static func _shell_token(template_id: String) -> String:
	for pair in SHELL_TOKENS:
		if template_id.contains(pair[0]):
			return pair[1]
	return ""

# Props ------------------------------------------------------------------------

func _build_props(report) -> void:
	var batches := _batches("Props")
	var kinds := {}
	var malformed := 0
	for prop in _rows("props"):
		if not _valid_prop(prop):
			malformed += 1
			continue
		var kind := String(prop.kind)
		if not kinds.has(kind):
			kinds[kind] = {"admitted": 0, "placements": [] as Array[Transform3D], "source": _prop_source(kind)}
		var entry: Dictionary = kinds[kind]
		entry.admitted += 1
		var position := _prop_position(prop)
		if not _near(position):
			continue
		var yaw := float(prop.get("yaw", 0.0))
		var half: Array = prop.half_extents
		if not entry.source.is_empty():
			var base := Coords.sim_to_godot(Vector3(position.x, position.y, float(prop.base_z))) if prop.get("base_z") != null else _seated(position, 0.0)
			entry.placements.append(Transform3D(Coords.yaw_basis(yaw), base))
		elif kind == "street_tree":
			entry.placements.append(Transform3D(Coords.yaw_basis(yaw).scaled_local(Vector3(1.4, 1.0, 1.4)), _seated(position, 2.5)))
		else:
			# Godot's local axes are sim x, up and -y, so the sim extents reorder.
			var scale := Vector3(maxf(0.2, float(half[0]) * 2.0), maxf(0.2, float(half[2]) * 2.0), maxf(0.2, float(half[1]) * 2.0))
			entry.placements.append(Transform3D(Coords.yaw_basis(yaw).scaled_local(scale), _seated(position, float(half[2]))))
	for kind in kinds:
		var entry: Dictionary = kinds[kind]
		var reason: String
		if not entry.source.is_empty():
			reason = "catalog model %s at its saved position" % entry.source
		elif kind == "street_tree":
			reason = "approximation: a cone; the catalog has no tree model"
		else:
			reason = "approximation: a box from its half extents; the catalog has no model"
		# Only a kind with rows in reach was staged.
		if not entry.placements.is_empty():
			var appearance: int
			if not entry.source.is_empty():
				var root := _scene(entry.source).instantiate()
				appearance = batches.add_appearance(root, _model_detail)
				root.free()
			elif kind == "street_tree":
				appearance = _stand_in(batches, _tree_mesh(), _model_detail)
			else:
				appearance = _stand_in(batches, _box_mesh(Color("7d6a50"), 0.55), _model_detail)
			for placement in entry.placements:
				batches.add_static(appearance, placement)
		report.add_budget(OWNER, "%s, within %d m of a frame target" % [reason, REACH_M], "prop:" + kind, entry.admitted, entry.placements.size(), entry.admitted - entry.placements.size())
	batches.seal()
	_report_malformed(report, "map.props", malformed)

func _prop_source(kind: String) -> String:
	var preferred: Array[String] = []
	if kind == "street_tree":
		preferred = ["tree_broadleaf"]
	else:
		if not _region.is_empty():
			preferred.append("%s_%s" % [kind, _region])
		preferred.append(kind)
	for key in preferred:
		var source := _entry_source(_appearances.get(key, {}))
		if not source.is_empty():
			return source
	var scenery := "tree" if kind == "street_tree" else kind
	for key in _appearances:
		var entry = _appearances[key]
		if typeof(entry) != TYPE_DICTIONARY or entry.get("unit") != "scenery" or entry.get("scenery") != scenery:
			continue
		var entry_region := String(entry.get("regional_family", ""))
		if not entry_region.is_empty() and entry_region != _region:
			continue
		var source := _entry_source(entry)
		if not source.is_empty():
			return source
	return ""

# Units ------------------------------------------------------------------------

func _build_units(side: String, report) -> void:
	_units = _batches("ObservedUnits")
	_side_color = Color("d95c5c") if side == "red" else Color("4d9be6")
	_contact_mesh = CylinderMesh.new()
	_contact_mesh.top_radius = 0.95
	_contact_mesh.bottom_radius = 0.95
	_contact_mesh.height = 0.05
	_contact_mesh.material = _material(Color(_side_color, 0.85), 1.0)
	_contact_mesh.material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	for kind in _unit_kinds:
		var source := _unit_source(kind)
		var reason := "catalog model %s at its authored size, unposed" % source if not source.is_empty() else "approximation: a capsule; the unit kind has no catalog appearance"
		report.add_budget(OWNER, reason, "unit:" + kind, _unit_kinds[kind], _unit_kinds[kind], 0)

## A unit kind's appearance: its catalog model, or a capsule in the side's
## colour, on a contact disc in that colour.
func _unit_appearance(kind: String) -> int:
	var holder := Node3D.new()
	var source := _unit_source(kind)
	if _staged.has(source):
		holder.add_child(_scene(source).instantiate())
	else:
		var capsule := MeshInstance3D.new()
		capsule.mesh = CapsuleMesh.new()
		capsule.mesh.radius = 0.5
		capsule.mesh.height = 2.0
		capsule.mesh.material = _material(_side_color, 1.0)
		capsule.mesh.material.emission_enabled = true
		capsule.mesh.material.emission = _side_color
		capsule.mesh.material.emission_energy_multiplier = 0.25
		capsule.position.y = 0.7
		holder.add_child(capsule)
	var contact := MeshInstance3D.new()
	contact.mesh = _contact_mesh
	contact.position.y = 0.03
	holder.add_child(contact)
	var appearance := _units.add_appearance(holder, _model_detail)
	holder.free()
	return appearance

## The unit's catalog appearance, or its first soldier's.
func _unit_source(kind: String) -> String:
	var unit: Dictionary = _units_catalog.get(kind, {})
	var appearance := String(unit.get("appearance", ""))
	if appearance.is_empty():
		var slots: Array = unit.get("body", {}).get("squad", {}).get("slots", [])
		if not slots.is_empty():
			var appearances: Array = _soldiers.get(String(slots[0]), {}).get("appearance", [])
			if not appearances.is_empty():
				appearance = String(appearances[0])
	return _entry_source(_appearances.get(appearance, {}))

# Shared -----------------------------------------------------------------------

func _load_catalogs() -> void:
	# The browser's detail tiers (`presentation.models` and `.buildings`).
	var presentation: Dictionary = _read_json("fixtures/game.json").presentation
	_model_detail = {"lod_px": presentation.models.lod_px, "sized": true}
	# A kit shell's walls are cut open for its facade modules at tiers 0 and 1
	# (`OPEN_TIERS`, packages/scene-assets/blender/city/kit.py); from tier 2
	# the shell is the whole building. This layer places no modules, so an
	# open tier draws over every coarser tier, which fill what the modules
	# would (a Paris dormer is in tier 1, a balcony's front in tier 3), and a
	# closed tier draws alone.
	_building_detail = {"lod_px": presentation.buildings.lod_px_per_m, "sized": false, "closed_tier": 2}
	var appearances = _read_json("assets/catalog.json")
	if typeof(appearances) == TYPE_DICTIONARY:
		_appearances = appearances.get("appearances", {})
	for path in ["fixtures/catalog.json", "fixtures/units/menu/units.json"]:
		var catalog = _read_json(path)
		if typeof(catalog) != TYPE_DICTIONARY:
			continue
		var units = catalog.get("units", {})
		if typeof(units) == TYPE_ARRAY:
			for unit in units:
				if typeof(unit) == TYPE_DICTIONARY and unit.has("id"):
					_units_catalog[String(unit.id)] = unit
		else:
			for id in units:
				_units_catalog[String(id)] = units[id]
		var soldiers: Dictionary = catalog.get("soldiers", {})
		for id in soldiers:
			_soldiers[String(id)] = soldiers[id]

static func _read_json(relative: String) -> Variant:
	return JSON.parse_string(FileAccess.get_file_as_string(Staging.repo_path(relative)))

static func _entry_source(entry: Variant) -> String:
	if typeof(entry) != TYPE_DICTIONARY:
		return ""
	if typeof(entry.get("source")) == TYPE_STRING:
		return entry.source
	var states: Dictionary = entry.get("states", {})
	for key in ["default", "summer"]:
		if typeof(states.get(key)) == TYPE_STRING:
			return states[key]
	for value in states.values():
		if typeof(value) == TYPE_STRING:
			return value
	return ""

func _scene(relative: String) -> PackedScene:
	if not _scenes.has(relative):
		_scenes[relative] = load(_staged[relative])
	return _scenes[relative]

## A one-mesh stand-in's appearance in `batches`.
static func _stand_in(batches: ModelBatches, mesh: Mesh, detail: Dictionary) -> int:
	var root := Node3D.new()
	var part := MeshInstance3D.new()
	part.mesh = mesh
	root.add_child(part)
	var appearance := batches.add_appearance(root, detail)
	root.free()
	return appearance

static func _box_mesh(color: Color, roughness: float) -> BoxMesh:
	var box := BoxMesh.new()
	box.material = _material(color, roughness)
	return box

static func _tree_mesh() -> CylinderMesh:
	var cone := CylinderMesh.new()
	cone.top_radius = 0.15
	cone.bottom_radius = 0.9
	cone.height = 5.0
	cone.material = _material(Color("385941"), 1.0)
	return cone

static func _material(color: Color, roughness: float) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	material.roughness = roughness
	return material

func _batches(batches_name: String) -> ModelBatches:
	var batches := ModelBatches.new(batches_name, _sun, _max_far_m)
	add_child(batches)
	return batches

func _near(position: Vector2) -> bool:
	for center in _centers:
		if position.distance_squared_to(center) <= REACH_M * REACH_M:
			return true
	return false

func _rows(key: String) -> Array:
	var rows = _map.get(key, [])
	return rows if typeof(rows) == TYPE_ARRAY else []

static func _report_malformed(report, source_id: String, count: int) -> void:
	if count > 0:
		report.add_budget(OWNER, "malformed rows skipped", source_id, count, 0, count)

static func _valid_building(building: Variant) -> bool:
	return typeof(building) == TYPE_DICTIONARY and typeof(building.get("template_id")) == TYPE_STRING and typeof(building.get("frame")) == TYPE_DICTIONARY and StrictJson.is_vector(building.frame.get("translation"), 3)

static func _valid_prop(prop: Variant) -> bool:
	return typeof(prop) == TYPE_DICTIONARY and typeof(prop.get("kind")) == TYPE_STRING and StrictJson.is_vector(prop.get("center"), 2) and StrictJson.is_vector(prop.get("half_extents"), 3)

static func _building_position(building: Dictionary) -> Vector2:
	return Vector2(float(building.frame.translation[0]), float(building.frame.translation[1]))

static func _prop_position(prop: Dictionary) -> Vector2:
	return Vector2(float(prop.center[0]), float(prop.center[1]))

## A Godot point `above` metres over the ground at sim `(x, y)`.
func _seated(position: Vector2, above: float) -> Vector3:
	return Coords.sim_to_godot(Vector3(position.x, position.y, _heights.height_at(position.x, position.y) + above))
