extends Node3D

## A rough native drawing of the saved map, kept until each layer's slice ports
## the browser's owner: a flat ground with saved surfaces, road segments and forest
## patches (`terrain`), catalog grass tufts (`grass`), buildings (`buildings`),
## props (`props`) and observed units (`models`). Every category is reported as
## a budget that names its approximation and how many rows it drew and dropped.
## Every position and yaw goes through `Coords`.

const Staging = preload("res://assets/staging.gd")
const Coords = preload("res://core/coords.gd")

const OWNER := "native/legacy_map_layer"
## Map rows farther than this from every frame target are not drawn.
const REACH_M := 1000.0
## A frame target this close to an earlier one adds no reach of its own.
const TARGET_SPACING_M := 25.0

var _map: Dictionary
var _region := ""
var _centers: Array[Vector2] = []
## Kind name -> number of distinct units of that kind in the capture.
var _unit_kinds: Dictionary
var _appearances := {}
var _units_catalog := {}
var _soldiers := {}
## Repository path -> staged resource path.
var _staged := {}
var _scenes := {}
var _shell_prototypes := {}
var _grass: Array = []
var _units_holder: Node3D
## Unit id -> its node, so a unit keeps its node however the rows are ordered.
var _unit_nodes := {}
var _unit_material: StandardMaterial3D
var _contact_mesh: CylinderMesh

## `targets` are the sim-space (x, y) frame targets the run draws.
func _init(map: Dictionary, targets: Array, unit_kinds: Dictionary) -> void:
	name = "LegacyMapLayer"
	_map = map
	_region = String(map.get("regional_family", "")).to_lower()
	_unit_kinds = unit_kinds
	for target in targets:
		if not _centers.any(func(center): return center.distance_to(target) < TARGET_SPACING_M):
			_centers.append(target)
	_grass = _grass_centers()
	_load_catalogs()

func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE:
		for prototype in _shell_prototypes.values():
			if prototype != null:
				prototype.free()

## The repository assets the listed layers draw.
func required_assets(layers: Array) -> Array:
	var needed := {}
	if layers.has("grass") and not _grass.is_empty():
		needed[_prop_source("grass")] = true
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

## Builds the listed layers from staged assets, reporting every category.
func build(layers: Array, staged: Dictionary, side: String, report) -> void:
	_staged = staged
	if layers.has("terrain"):
		_build_terrain(report)
	if layers.has("grass"):
		_build_grass(report)
	if layers.has("buildings"):
		_build_buildings(report)
	if layers.has("props"):
		_build_props(report)
	if layers.has("models"):
		_build_units(side, report)

## Shows the units of one decoded publication and hides every other unit.
func show_units(units: Array) -> void:
	if _units_holder == null:
		return
	var seen := {}
	for unit in units:
		var node: Node3D = _unit_nodes.get(unit.id)
		if node == null or node.get_meta("kind") != unit.kindName:
			if node != null:
				node.free()
			node = _unit_node(unit.kindName)
			_units_holder.add_child(node)
			_unit_nodes[unit.id] = node
		node.position = Coords.sim_to_godot(Vector3(float(unit.position[0]), float(unit.position[1]), maxf(0.0, float(unit.position[2]))))
		node.rotation.y = float(unit.yaw)
		seen[unit.id] = true
	for id in _unit_nodes:
		_unit_nodes[id].visible = seen.has(id)

# Terrain ----------------------------------------------------------------------

func _build_terrain(report) -> void:
	var holder := _holder("Terrain")
	var size := Vector2(float(_map.size[0]), float(_map.size[1]))
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = size
	plane.material = _terrain_material()
	ground.mesh = plane
	ground.position = Coords.sim_to_godot(Vector3(size.x * 0.5, size.y * 0.5, -0.08))
	holder.add_child(ground)
	report.add_budget(OWNER, "approximation: one flat plane with a %s field shader" % ("noise" if _region == "china" else "flat"), "map.size", 1, 1, 0)

	var rings: Array = []
	var roads: Array[Transform3D] = []
	var counts := {"polygons": 0, "segments": 0, "malformed": 0}
	for surface in _rows("surfaces"):
		var shape = surface.get("shape") if typeof(surface) == TYPE_DICTIONARY else null
		if typeof(shape) == TYPE_DICTIONARY and shape.get("kind") == "polygon" and _is_ring(shape.get("ring")):
			counts.polygons += 1
			if _near(_ring_center(shape.ring)):
				rings.append(shape.ring)
		elif typeof(shape) == TYPE_DICTIONARY and shape.get("kind") == "stroke" and _is_ring(shape.get("points"), 2):
			var points: Array = shape.points
			for i in points.size() - 1:
				var start := Vector2(float(points[i][0]), float(points[i][1]))
				var end := Vector2(float(points[i + 1][0]), float(points[i + 1][1]))
				counts.segments += 1
				if _near((start + end) * 0.5) and start.distance_to(end) > 0.01:
					var basis := Coords.yaw_basis(atan2(end.y - start.y, end.x - start.x)).scaled_local(Vector3(start.distance_to(end), 0.035, float(shape.get("width_m", 8.0))))
					roads.append(Transform3D(basis, _ground_point((start + end) * 0.5, 0.0)))
		else:
			counts.malformed += 1
	var paving := _polygon_mesh(rings)
	if paving != null:
		var node := MeshInstance3D.new()
		node.mesh = paving
		node.material_override = _material(Color("a8a69e"), 0.96)
		node.position.y = -0.05
		holder.add_child(node)
	_add_instances(holder, [_box_part(Color("817e70"), 0.92)], roads)
	report.add_budget(OWNER, "approximation: flat surface polygons within %d m of a frame target" % REACH_M, "map.surfaces/polygons", counts.polygons, rings.size(), counts.polygons - rings.size())
	report.add_budget(OWNER, "approximation: road segments as flat boxes within %d m of a frame target" % REACH_M, "map.surfaces/strokes", counts.segments, roads.size(), counts.segments - roads.size())
	_report_malformed(report, "map.surfaces", counts.malformed)

	var forests: Array = []
	var malformed := 0
	for forest in _rows("forests"):
		if typeof(forest) == TYPE_DICTIONARY and typeof(forest.get("shape")) == TYPE_DICTIONARY and forest.shape.get("kind") == "polygon" and _is_ring(forest.shape.get("ring")):
			forests.append(forest.shape.ring)
		else:
			malformed += 1
	var floor_mesh := _polygon_mesh(forests)
	if floor_mesh != null:
		var node := MeshInstance3D.new()
		node.mesh = floor_mesh
		node.material_override = _material(Color("304b3b"), 1.0)
		node.position.y = -0.01
		holder.add_child(node)
	report.add_budget(OWNER, "approximation: flat forest-floor patches", "map.forests", forests.size(), forests.size(), 0)
	_report_malformed(report, "map.forests", malformed)

func _terrain_material() -> Material:
	if _region != "china":
		return _material(Color("6d7378"), 0.96)
	var material := ShaderMaterial.new()
	material.shader = Shader.new()
	material.shader.code = TERRAIN_SHADER
	return material

# Grass ------------------------------------------------------------------------

func _build_grass(report) -> void:
	var source := _prop_source("grass")
	if _grass.is_empty():
		report.add_budget(OWNER, "no forest ring lies near a frame target", "grass", 0, 0, 0)
		return
	if source.is_empty():
		report.add_budget(OWNER, "the catalog has no grass appearance", "grass", _grass.size(), 0, _grass.size())
		return
	var placements: Array[Transform3D] = []
	for i in _grass.size():
		var basis := Coords.yaw_basis(deg_to_rad(float((i * 37) % 360))).scaled_local(Vector3.ONE * 8.0)
		placements.append(Transform3D(basis, _ground_point(_grass[i], 0.0)))
	_add_instances(_holder("Grass"), _mesh_parts(source), placements, _material(Color("55743a"), 0.98))
	report.add_budget(OWNER, "approximation: %s tufts scaled 8x on a fixed grid in forest rings%s" % [source, " and around frame targets" if _region == "china" else ""], "grass", _grass.size(), _grass.size(), 0)

## Tuft positions: a 3x3 grid inside each forest ring near a frame target and,
## for China, a 7x7 meadow grid at 8 m spacing around each frame target.
func _grass_centers() -> Array:
	var centers: Array = []
	var seen := {}
	for forest in _rows("forests"):
		if typeof(forest) != TYPE_DICTIONARY or typeof(forest.get("shape")) != TYPE_DICTIONARY or not _is_ring(forest.shape.get("ring")):
			continue
		var polygon := _polygon(forest.shape.ring)
		var bounds := Rect2(polygon[0], Vector2.ZERO)
		for point in polygon:
			bounds = bounds.expand(point)
		if bounds.size.x <= 0.0 or bounds.size.y <= 0.0 or not _near(bounds.get_center()):
			continue
		for y in 3:
			for x in 3:
				var candidate := bounds.get_center() + bounds.size * 0.35 * Vector2(float(x - 1), float(y - 1))
				if Geometry2D.is_point_in_polygon(candidate, polygon) and _near(candidate):
					_add_once(centers, seen, candidate)
	if _region == "china":
		for center in _centers:
			for y in range(-3, 4):
				for x in range(-3, 4):
					_add_once(centers, seen, center + Vector2(float(x * 8), float(y * 8)))
	return centers

static func _add_once(points: Array, seen: Dictionary, point: Vector2) -> void:
	var key := "%0.2f:%0.2f" % [point.x, point.y]
	if not seen.has(key):
		seen[key] = true
		points.append(point)

# Buildings --------------------------------------------------------------------

func _build_buildings(report) -> void:
	var holder := _holder("Buildings")
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
		var shell := _shell(String(building.template_id))
		if shell != null:
			shell.transform = Transform3D(Coords.yaw_basis(yaw), _ground_point(position, float(translation[2])))
			holder.add_child(shell)
		else:
			boxes.append(Transform3D(Coords.yaw_basis(yaw).scaled_local(Vector3(18.0, 8.0, 18.0)), _ground_point(position, 4.0 + float(translation[2]))))
	_add_instances(holder, [_box_part(Color("7b7d7b"), 0.8)], boxes)
	report.add_budget(OWNER, "approximation: kit shells picked by template token (until slice 07), within %d m of a frame target" % REACH_M, "map.buildings", admitted, near, admitted - near)
	if not boxes.is_empty():
		report.add_budget(OWNER, "approximation: an 18x8x18 m box where no kit shell matches the template", "map.buildings/boxes", boxes.size(), boxes.size(), 0)
	_report_malformed(report, "map.buildings", malformed)

func _shell(template_id: String) -> Node3D:
	var token := _shell_token(template_id)
	var kit := _kit_source(template_id)
	if token.is_empty() or not _staged.has(kit):
		return null
	var key := kit + "|" + token
	if not _shell_prototypes.has(key):
		var catalog := _scene(kit).instantiate()
		var shell := catalog.get_node_or_null(NodePath(token)) as Node3D
		_shell_prototypes[key] = shell.duplicate() if shell != null else null
		catalog.free()
	var prototype: Node3D = _shell_prototypes[key]
	return prototype.duplicate() as Node3D if prototype != null else null

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
	var holder := _holder("Props")
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
			var base := float(prop.base_z) if prop.get("base_z") != null else 0.0
			entry.placements.append(Transform3D(Coords.yaw_basis(yaw), _ground_point(position, base)))
		elif kind == "street_tree":
			entry.placements.append(Transform3D(Coords.yaw_basis(yaw).scaled_local(Vector3(1.4, 1.0, 1.4)), _ground_point(position, 2.5)))
		else:
			# Godot's local axes are sim x, up and -y, so the sim extents reorder.
			var scale := Vector3(maxf(0.2, float(half[0]) * 2.0), maxf(0.2, float(half[2]) * 2.0), maxf(0.2, float(half[1]) * 2.0))
			entry.placements.append(Transform3D(Coords.yaw_basis(yaw).scaled_local(scale), _ground_point(position, float(half[2]))))
	for kind in kinds:
		var entry: Dictionary = kinds[kind]
		var parts: Array
		var reason: String
		if not entry.source.is_empty():
			# Only a kind with rows in reach was staged.
			parts = _mesh_parts(entry.source) if not entry.placements.is_empty() else []
			reason = "catalog model %s at its saved position" % entry.source
		elif kind == "street_tree":
			parts = [_tree_part()]
			reason = "approximation: a cone; the catalog has no tree model"
		else:
			parts = [_box_part(Color("7d6a50"), 0.55)]
			reason = "approximation: a box from its half extents; the catalog has no model"
		_add_instances(holder, parts, entry.placements)
		report.add_budget(OWNER, "%s, within %d m of a frame target" % [reason, REACH_M], "prop:" + kind, entry.admitted, entry.placements.size(), entry.admitted - entry.placements.size())
	_report_malformed(report, "map.props", malformed)

func _prop_source(kind: String) -> String:
	var preferred: Array[String] = []
	if kind == "street_tree":
		preferred = ["tree_broadleaf"]
	elif kind == "grass":
		preferred = ["grass_meadow"]
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
	_units_holder = _holder("ObservedUnits")
	var color := Color("d95c5c") if side == "red" else Color("4d9be6")
	_unit_material = _material(color, 1.0)
	_unit_material.emission_enabled = true
	_unit_material.emission = color
	_unit_material.emission_energy_multiplier = 0.25
	_contact_mesh = CylinderMesh.new()
	_contact_mesh.top_radius = 0.95
	_contact_mesh.bottom_radius = 0.95
	_contact_mesh.height = 0.05
	_contact_mesh.material = _material(Color(color, 0.85), 1.0)
	_contact_mesh.material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	for kind in _unit_kinds:
		var source := _unit_source(kind)
		var reason := "catalog model %s at its authored size, unposed" % source if not source.is_empty() else "approximation: a capsule; the unit kind has no catalog appearance"
		report.add_budget(OWNER, reason, "unit:" + kind, _unit_kinds[kind], _unit_kinds[kind], 0)

func _unit_node(kind: String) -> Node3D:
	var holder := Node3D.new()
	holder.set_meta("kind", kind)
	var source := _unit_source(kind)
	if _staged.has(source):
		holder.add_child(_scene(source).instantiate())
	else:
		var capsule := MeshInstance3D.new()
		capsule.mesh = CapsuleMesh.new()
		capsule.mesh.radius = 0.5
		capsule.mesh.height = 2.0
		capsule.material_override = _unit_material
		capsule.position.y = 0.7
		holder.add_child(capsule)
	var contact := MeshInstance3D.new()
	contact.mesh = _contact_mesh
	contact.position.y = 0.03
	holder.add_child(contact)
	return holder

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

## Every mesh in a staged scene with its transform relative to the scene root.
func _mesh_parts(relative: String) -> Array:
	var root := _scene(relative).instantiate()
	var parts := []
	_collect_meshes(root, Transform3D.IDENTITY, parts)
	root.free()
	return parts

static func _collect_meshes(node: Node, parent: Transform3D, parts: Array) -> void:
	var transform: Transform3D = parent * node.transform if node is Node3D else parent
	if node is MeshInstance3D and node.mesh != null:
		parts.append({"mesh": node.mesh, "transform": transform})
	for child in node.get_children():
		_collect_meshes(child, transform, parts)

## One MultiMesh per part, each instance placed at a placement.
static func _add_instances(holder: Node3D, parts: Array, placements: Array[Transform3D], material: Material = null) -> void:
	if placements.is_empty():
		return
	for part in parts:
		var multi := MultiMesh.new()
		multi.transform_format = MultiMesh.TRANSFORM_3D
		multi.mesh = part.mesh
		multi.instance_count = placements.size()
		var offset: Transform3D = part.get("transform", Transform3D.IDENTITY)
		for i in placements.size():
			multi.set_instance_transform(i, placements[i] * offset)
		var batch := MultiMeshInstance3D.new()
		batch.multimesh = multi
		batch.material_override = material
		holder.add_child(batch)

static func _box_part(color: Color, roughness: float) -> Dictionary:
	var box := BoxMesh.new()
	box.material = _material(color, roughness)
	return {"mesh": box}

static func _tree_part() -> Dictionary:
	var cone := CylinderMesh.new()
	cone.top_radius = 0.15
	cone.bottom_radius = 0.9
	cone.height = 5.0
	cone.material = _material(Color("385941"), 1.0)
	return {"mesh": cone}

static func _material(color: Color, roughness: float) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	material.roughness = roughness
	return material

func _holder(holder_name: String) -> Node3D:
	var holder := Node3D.new()
	holder.name = holder_name
	add_child(holder)
	return holder

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
	return typeof(building) == TYPE_DICTIONARY and typeof(building.get("template_id")) == TYPE_STRING and typeof(building.get("frame")) == TYPE_DICTIONARY and _is_numbers(building.frame.get("translation"), 3)

static func _valid_prop(prop: Variant) -> bool:
	return typeof(prop) == TYPE_DICTIONARY and typeof(prop.get("kind")) == TYPE_STRING and _is_numbers(prop.get("center"), 2) and _is_numbers(prop.get("half_extents"), 3)

static func _building_position(building: Dictionary) -> Vector2:
	return Vector2(float(building.frame.translation[0]), float(building.frame.translation[1]))

static func _prop_position(prop: Dictionary) -> Vector2:
	return Vector2(float(prop.center[0]), float(prop.center[1]))

static func _is_numbers(value: Variant, size: int) -> bool:
	if typeof(value) != TYPE_ARRAY or value.size() != size:
		return false
	return value.all(func(item): return typeof(item) == TYPE_INT or typeof(item) == TYPE_FLOAT)

static func _is_ring(value: Variant, minimum := 3) -> bool:
	return typeof(value) == TYPE_ARRAY and value.size() >= minimum and value.all(func(point): return _is_numbers(point, 2))

static func _polygon(ring: Array) -> PackedVector2Array:
	var polygon := PackedVector2Array()
	for point in ring:
		polygon.append(Vector2(float(point[0]), float(point[1])))
	return polygon

static func _ring_center(ring: Array) -> Vector2:
	var polygon := _polygon(ring)
	var bounds := Rect2(polygon[0], Vector2.ZERO)
	for point in polygon:
		bounds = bounds.expand(point)
	return bounds.get_center()

## A Godot point on flat ground at sim `(x, y)`, raised `height` metres.
static func _ground_point(position: Vector2, height: float) -> Vector3:
	return Coords.sim_to_godot(Vector3(position.x, position.y, height))

## One mesh of every ring's triangulation, flat at height zero and facing up
## whichever way the ring winds.
static func _polygon_mesh(rings: Array) -> ArrayMesh:
	var vertices := PackedVector3Array()
	var indices := PackedInt32Array()
	for ring in rings:
		var polygon := _polygon(ring)
		var triangles := Geometry2D.triangulate_polygon(polygon)
		var base := vertices.size()
		for point in polygon:
			vertices.append(_ground_point(point, 0.0))
		for i in range(0, triangles.size(), 3):
			var a := vertices[base + triangles[i]]
			var b := vertices[base + triangles[i + 1]]
			var c := vertices[base + triangles[i + 2]]
			# Godot's front faces wind clockwise seen from the front, so an
			# up-facing triangle's (b - a) x (c - a) points down.
			var order := [0, 1, 2] if (b - a).cross(c - a).y < 0.0 else [0, 2, 1]
			for corner in order:
				indices.append(base + triangles[i + corner])
	if indices.is_empty():
		return null
	var normals := PackedVector3Array()
	normals.resize(vertices.size())
	normals.fill(Vector3.UP)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh

const TERRAIN_SHADER := """
shader_type spatial;
render_mode diffuse_burley;

varying vec3 world_pos;

float hash21(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

float value_noise(vec2 p) {
    vec2 cell = floor(p);
    vec2 local = fract(p);
    local = local * local * (3.0 - 2.0 * local);
    float a = hash21(cell);
    float b = hash21(cell + vec2(1.0, 0.0));
    float c = hash21(cell + vec2(0.0, 1.0));
    float d = hash21(cell + vec2(1.0, 1.0));
    return mix(mix(a, b, local.x), mix(c, d, local.x), local.y);
}

void vertex() {
    world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
    // Sim (x, y) from Godot (x, -z), so the pattern lies in the sim plane.
    vec2 p = vec2(world_pos.x, -world_pos.z);
    float broad = value_noise(p * 0.018);
    float fine = value_noise(p * 0.35);
    float rows = 0.5 + 0.5 * sin(p.x * 0.17 + p.y * 0.043 + broad * 2.0);
    float furrows = smoothstep(0.40, 0.62, rows) * (0.35 + 0.65 * broad);
    vec3 dark = vec3(0.20, 0.24, 0.14);
    vec3 mid = vec3(0.31, 0.34, 0.18);
    vec3 light = vec3(0.43, 0.39, 0.22);
    vec3 field = mix(dark, mid, broad);
    field = mix(field, light, fine * 0.18 + furrows * 0.16);
    ALBEDO = field;
    ROUGHNESS = 0.96;
}
"""
