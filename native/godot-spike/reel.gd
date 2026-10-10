extends Node3D

const CaptureDecoder = preload("res://presentation_capture.gd")

## Camera/workload playback for the authored menu reel.
## This proves the browser-owned reel identity and camera contract while the
## presentation-capture decoder is still being built. Reports are explicit.
const DEFAULT_SOURCE := "res://../../fixtures/menu-backdrop.json"
const INSTANCE_COUNT := 4096
var scenes: Array = []
var scene_index := 0
var shot_index := 0
var elapsed := 0.0
var scene_elapsed := 0.0
var shot_elapsed := 0.0
var intervals: Array[float] = []
var camera: Camera3D
var started := false
var source_path := ""
var capture_result: Dictionary = {"valid": false, "comparison_ready": false, "comparison_blocker": "no presentation capture supplied"}
var capture_results: Dictionary = {}
var capture_errors: Dictionary = {}
var authored_asset_loaded := false
var authored_building_count := 0
var authored_map_scene_count := 0
## A non-positive limit means the authored catalog is complete.  A positive
## value is an explicit diagnostic cap, never an implicit production default.
var authored_building_limit := 0
var authored_cull_radius := 900.0
var map_render_radius := 1000.0
var authored_scene_nodes: Dictionary = {}
var authored_scene_by_family: Dictionary = {}
var authored_shell_prototypes: Dictionary = {}
var authored_unresolved_templates: Dictionary = {}
var cut_dir := ""
var cuts_saved := 0
var cut_frame_timeouts := 0
var saved_cut_paths: Dictionary = {}
var pending_cut_saves := 0
var reel_finished := false
var capture_word_count := 0
var capture_layout_valid := false
var map_geometry_nodes: Dictionary = {}
var map_geometry_counts: Dictionary = {}
var proxy_field_used := false
var startup_started_usec := 0
var startup_ms := 0.0
var semantic_decode_ms := 0.0
var semantic_fog_publications := 0
var semantic_results: Dictionary = {}
var unit_nodes: Array[Node3D] = []
var fog_rendered_cells: Dictionary = {}
var fog_nodes: Dictionary = {}
var fog_cell_nodes: Dictionary = {}

func _ready() -> void:
	startup_started_usec = Time.get_ticks_usec()
	var configured := OS.get_environment("GODOT_REEL_SOURCE")
	source_path = _resolve_path(configured) if not configured.is_empty() else ProjectSettings.globalize_path(DEFAULT_SOURCE)
	var file := FileAccess.open(source_path, FileAccess.READ)
	if file == null:
		push_error("unable to read menu reel source: " + source_path)
		get_tree().quit(2)
		return
	var parsed = JSON.parse_string(file.get_as_text())
	if typeof(parsed) != TYPE_DICTIONARY or not parsed.has("scenes") or parsed.scenes.is_empty():
		push_error("menu reel source has no scenes")
		get_tree().quit(2)
		return
	scenes = parsed.scenes
	_load_presentation_captures()
	_consume_capture_words()
	_decode_semantic_captures()
	cut_dir = _resolve_path(OS.get_environment("GODOT_REEL_CUTS"))
	_build_world()
	startup_ms = float(Time.get_ticks_usec() - startup_started_usec) / 1000.0
	started = true
	if OS.get_environment("GODOT_EXIT_AFTER_READY") == "1":
		call_deferred("_exit_after_ready")

func _exit_after_ready() -> void:
	print(JSON.stringify({
		"map_geometry": map_geometry_counts,
		"semantic_scenes": semantic_results.size(),
		"authored_asset_loaded": authored_asset_loaded,
		"authored_building_count": authored_building_count,
		"authored_map_scene_count": authored_map_scene_count,
		"authored_unresolved_templates": authored_unresolved_templates,
	}))
	get_tree().quit()

func _load_presentation_captures() -> void:
	var configured := OS.get_environment("GODOT_PRESENTATION_CAPTURE")
	var directory := OS.get_environment("GODOT_PRESENTATION_CAPTURE_DIR")
	if not directory.is_empty():
		directory = _resolve_path(directory)
		for scene in scenes:
			var path := directory.path_join(String(scene.map) + ".json")
			var result := _read_capture(path)
			if result.valid:
				capture_results[scene.map] = result
			else:
				capture_errors[scene.map] = result.get("comparison_blocker", "invalid capture")
		return
	if not configured.is_empty():
		capture_result = _read_capture(_resolve_path(configured))
		if capture_result.valid:
			capture_results[capture_result.capture.workload.scene] = capture_result
		else:
			capture_errors["configured"] = capture_result.get("comparison_blocker", "invalid capture")

func _read_capture(path: String) -> Dictionary:
	var capture_file := FileAccess.open(path, FileAccess.READ)
	if capture_file == null:
		return {"valid": false, "comparison_ready": false, "comparison_blocker": "unable to read presentation capture: " + path}
	return CaptureDecoder.decode_json(capture_file.get_as_text())

func _consume_capture_words() -> void:
	for result in capture_results.values():
		if not result.get("valid", false):
			continue
		capture_layout_valid = capture_layout_valid or typeof(result.get("layout")) == TYPE_DICTIONARY
		var capture: Dictionary = result.capture
		for sample in capture.samples:
			for word in sample.publication:
				capture_word_count += 1

func _decode_semantic_captures() -> void:
	var started_usec := Time.get_ticks_usec()
	for scene_name in capture_results:
		var capture_result: Dictionary = capture_results[scene_name]
		var baselines: Array = []
		var fog_baseline: Array = []
		var frames: Array = []
		var semantic_limit := int(OS.get_environment("GODOT_SEMANTIC_SAMPLE_LIMIT"))
		if semantic_limit <= 0:
			semantic_limit = capture_result.capture.samples.size()
		var semantic_index := 0
		for sample in capture_result.capture.samples:
			if semantic_index >= semantic_limit:
				break
			semantic_index += 1
			var decoded := CaptureDecoder.decode_publication(capture_result, sample.publication, baselines, fog_baseline)
			if not decoded.valid:
				continue
			baselines = decoded.baselines
			if decoded.has("fog"):
				fog_baseline = decoded.fog.bits
			if decoded.has("fog"):
				semantic_fog_publications += 1
			frames.append({"tick": sample.tick, "units": decoded.units, "fog": decoded.get("fog", {})})
		semantic_results[scene_name] = frames
	semantic_decode_ms = float(Time.get_ticks_usec() - started_usec) / 1000.0

func _build_world() -> void:
	var environment := WorldEnvironment.new()
	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	var sky := Sky.new()
	var sky_material := ProceduralSkyMaterial.new()
	sky_material.sky_top_color = Color("456e91")
	sky_material.sky_horizon_color = Color("b9d3d2")
	sky_material.ground_bottom_color = Color("4d5148")
	sky_material.ground_horizon_color = Color("8e927e")
	sky_material.sun_angle_max = 18.0
	sky_material.sun_curve = 0.08
	sky.sky_material = sky_material
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color("8aa4bc")
	env.ambient_light_energy = 0.8
	env.fog_enabled = true
	env.fog_light_color = Color("6f7e85")
	env.fog_light_energy = 0.35
	env.fog_density = 0.004
	env.fog_sky_affect = 0.25
	environment.environment = env
	add_child(environment)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-55.0, -30.0, 0.0)
	sun.light_energy = 1.5
	add_child(sun)
	camera = Camera3D.new()
	add_child(camera)
	_build_map_geometry()
	_build_fog_layers()
	var unit_holder := Node3D.new()
	unit_holder.name = "ObservedUnits"
	add_child(unit_holder)
	var authored_path := OS.get_environment("GODOT_AUTHORED_SCENE")
	var authored_paths := OS.get_environment("GODOT_AUTHORED_SCENES")
	var limit_text := OS.get_environment("GODOT_AUTHORED_BUILDING_LIMIT")
	if not limit_text.is_empty():
		authored_building_limit = maxi(0, int(limit_text))
	var cull_text := OS.get_environment("GODOT_AUTHORED_CULL_RADIUS")
	if not cull_text.is_empty() and float(cull_text) > 0.0:
		authored_cull_radius = float(cull_text)
	var map_cull_text := OS.get_environment("GODOT_MAP_RENDER_RADIUS")
	if not map_cull_text.is_empty() and float(map_cull_text) > 0.0:
		map_render_radius = float(map_cull_text)
	if not authored_paths.is_empty():
		for path in authored_paths.split(","):
			var clean_path := path.strip_edges()
			var scene_resource = load(clean_path)
			if scene_resource is PackedScene:
				var family_key := _authored_family_key(clean_path)
				authored_scene_by_family[family_key] = scene_resource
		if _build_authored_maps(null):
			authored_asset_loaded = true
			return
	if not authored_path.is_empty():
		var authored = load(authored_path)
		if authored is PackedScene:
			if _build_authored_maps(authored):
				authored_asset_loaded = true
				return
			var instance = authored.instantiate()
			add_child(instance)
			authored_asset_loaded = true
			return
	if map_geometry_nodes.is_empty():
		_build_proxy_field()

func _build_map_geometry() -> void:
	var directory := _authored_map_directory()
	var prop_limit := _map_limit("GODOT_MAP_PROP_LIMIT")
	var building_limit := _map_limit("GODOT_MAP_BUILDING_LIMIT")
	var road_limit := _map_limit("GODOT_MAP_ROAD_LIMIT")
	for scene in scenes:
		var map_name := String(scene.map)
		var map_file := FileAccess.open(directory.path_join(map_name).path_join("map.json"), FileAccess.READ)
		if map_file == null:
			continue
		var map = JSON.parse_string(map_file.get_as_text())
		if typeof(map) != TYPE_DICTIONARY:
			continue
		var holder := Node3D.new()
		map_geometry_nodes[map_name] = holder
		holder.name = "MapGeometry_%s" % map_name
		holder.visible = map_name == String(scenes[0].map)
		add_child(holder)
		var counts := {"terrain": 0, "forests": 0, "roads": 0, "props": 0, "buildings": 0}
		var road_transforms: Array[Transform3D] = []
		var building_transforms: Array[Transform3D] = []
		var prop_transforms: Array[Transform3D] = []
		var tree_transforms: Array[Transform3D] = []
		var render_centers := _map_render_centers(map_name)
		var size: Array = map.get("size", [100.0, 100.0])
		var ground := MeshInstance3D.new()
		var ground_mesh := PlaneMesh.new()
		ground_mesh.size = Vector2(float(size[0]), float(size[1]))
		var ground_material := StandardMaterial3D.new()
		ground_material.albedo_color = Color("756c56") if String(map.get("regional_family", "")) == "china" else Color("62666b")
		ground_material.roughness = 0.92
		ground_mesh.material = ground_material
		ground.mesh = ground_mesh
		ground.position = Vector3(float(size[0]) * 0.5, -0.08, float(size[1]) * 0.5)
		holder.add_child(ground)
		if String(map.get("regional_family", "")) == "china":
			_add_field_strips(holder, Vector2(float(size[0]), float(size[1])))
		counts.terrain = 1
		for surface in map.get("surfaces", []):
			if typeof(surface) != TYPE_DICTIONARY or typeof(surface.get("shape")) != TYPE_DICTIONARY:
				continue
			var shape: Dictionary = surface.shape
			if shape.get("kind") != "stroke" or typeof(shape.get("points")) != TYPE_ARRAY:
				continue
			var points: Array = shape.points
			var remaining_roads := points.size() - 1 if road_limit <= 0 else mini(points.size() - 1, road_limit - counts.roads)
			for i in range(max(0, remaining_roads)):
				var a: Array = points[i]
				var b: Array = points[i + 1]
				var start := Vector2(float(a[0]), float(a[1]))
				var end := Vector2(float(b[0]), float(b[1]))
				var length := start.distance_to(end)
				if length <= 0.01:
					continue
				var midpoint := Vector2((start.x + end.x) * 0.5, (start.y + end.y) * 0.5)
				if _near_render_center(midpoint, render_centers):
					var basis := Basis(Vector3.UP, -atan2(end.y - start.y, end.x - start.x))
					basis = basis.scaled(Vector3(length, 0.035, float(shape.get("width_m", 8.0))))
					road_transforms.append(Transform3D(basis, Vector3(midpoint.x, 0.0, midpoint.y)))
				counts.roads += 1
		_add_box_batch(holder, road_transforms, Color("30383b"), 0.85)
		for forest in map.get("forests", []):
			if typeof(forest) != TYPE_DICTIONARY or typeof(forest.get("shape")) != TYPE_DICTIONARY:
				continue
			var forest_shape: Dictionary = forest.shape
			if forest_shape.get("kind") != "polygon" or typeof(forest_shape.get("ring")) != TYPE_ARRAY:
				continue
			var ring: Array = forest_shape.ring
			if ring.size() < 3:
				continue
			var min_x := INF
			var max_x := -INF
			var min_y := INF
			var max_y := -INF
			for point in ring:
				if typeof(point) != TYPE_ARRAY or point.size() < 2:
					continue
				min_x = minf(min_x, float(point[0]))
				max_x = maxf(max_x, float(point[0]))
				min_y = minf(min_y, float(point[1]))
				max_y = maxf(max_y, float(point[1]))
			if not is_finite(min_x) or max_x <= min_x or max_y <= min_y:
				continue
			var forest_node := MeshInstance3D.new()
			var forest_mesh := BoxMesh.new()
			forest_mesh.size = Vector3(max_x - min_x, 0.04, max_y - min_y)
			var forest_material := StandardMaterial3D.new()
			forest_material.albedo_color = Color("304b3b")
			forest_material.roughness = 1.0
			forest_mesh.material = forest_material
			forest_node.mesh = forest_mesh
			forest_node.position = Vector3((min_x + max_x) * 0.5, -0.01, (min_y + max_y) * 0.5)
			holder.add_child(forest_node)
			counts.forests += 1
		for building in map.get("buildings", []):
			if building_limit > 0 and counts.buildings >= building_limit:
				break
			if typeof(building) != TYPE_DICTIONARY:
				continue
			var frame: Dictionary = building.get("frame", {})
			var translation: Array = frame.get("translation", [0.0, 0.0, 0.0])
			var building_position := Vector2(float(translation[0]), float(translation[1]))
			if _near_render_center(building_position, render_centers):
				var building_basis := Basis(Vector3.UP, float(frame.get("yaw", 0.0))).scaled(Vector3(18.0, 8.0, 18.0))
				building_transforms.append(Transform3D(building_basis, Vector3(building_position.x, 4.0 + float(translation[2]), building_position.y)))
			counts.buildings += 1
		_add_box_batch(holder, building_transforms, Color("7b7d7b"), 0.8)
		for prop in map.get("props", []):
			if (prop_limit > 0 and counts.props >= prop_limit) or typeof(prop) != TYPE_DICTIONARY:
				break
			var center: Array = prop.get("center", [0.0, 0.0])
			var half: Array = prop.get("half_extents", [1.0, 1.0, 0.5])
			var prop_position := Vector2(float(center[0]), float(center[1]))
			if _near_render_center(prop_position, render_centers):
				if String(prop.get("kind", "")) == "street_tree":
					var tree_basis := Basis(Vector3.UP, float(prop.get("yaw", 0.0))).scaled(Vector3(1.4, 1.0, 1.4))
					tree_transforms.append(Transform3D(tree_basis, Vector3(prop_position.x, 2.5, prop_position.y)))
				else:
					var prop_basis := Basis(Vector3.UP, float(prop.get("yaw", 0.0))).scaled(Vector3(max(0.2, float(half[0]) * 2.0), max(0.2, float(half[2]) * 2.0), max(0.2, float(half[1]) * 2.0)))
					prop_transforms.append(Transform3D(prop_basis, Vector3(prop_position.x, float(half[2]), prop_position.y)))
			counts.props += 1
		_add_box_batch(holder, prop_transforms, Color("7d6a50"), 0.55)
		_add_tree_batch(holder, tree_transforms)
		counts["rendered_roads"] = road_transforms.size()
		counts["rendered_buildings"] = building_transforms.size()
		counts["rendered_props"] = prop_transforms.size()
		counts["rendered_trees"] = tree_transforms.size()
		counts["building_limit"] = building_limit
		counts["prop_limit"] = prop_limit
		counts["road_limit"] = road_limit
		map_geometry_counts[map_name] = counts

func _map_limit(environment_name: String) -> int:
	var value := OS.get_environment(environment_name)
	return int(value) if not value.is_empty() else 0

func _authored_map_directory() -> String:
	var configured := OS.get_environment("GODOT_AUTHORED_MAP_DIR")
	return _resolve_path(configured) if not configured.is_empty() else ProjectSettings.globalize_path("res://../../fixtures/maps")

func _resolve_path(path: String) -> String:
	if path.is_empty():
		return ""
	if path.begins_with("res://") or path.is_absolute_path():
		return ProjectSettings.globalize_path(path)
	return ProjectSettings.globalize_path("res://../../" + path)

func _map_render_centers(map_name: String) -> Array:
	var result: Dictionary = capture_results.get(map_name, {})
	var frames: Array = result.get("capture", {}).get("frames", [])
	var centers: Array = []
	if frames.is_empty():
		centers.append(Vector2.ZERO)
		return centers
	var stride := maxi(1, int(ceil(float(frames.size()) / 64.0)))
	for index in range(0, frames.size(), stride):
		var target: Array = frames[index].get("camera", {}).get("target", [0.0, 0.0])
		centers.append(Vector2(float(target[0]), float(target[1])))
	var last_target: Array = frames.back().get("camera", {}).get("target", [0.0, 0.0])
	centers.append(Vector2(float(last_target[0]), float(last_target[1])))
	return centers

func _near_render_center(position: Vector2, centers: Array) -> bool:
	for center in centers:
		if position.distance_to(center) <= map_render_radius:
			return true
	return false

func _add_box_batch(holder: Node3D, transforms: Array[Transform3D], color: Color, roughness: float) -> void:
	if transforms.is_empty():
		return
	var mesh := BoxMesh.new()
	mesh.size = Vector3.ONE
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	material.roughness = roughness
	mesh.material = material
	var multi := MultiMesh.new()
	multi.transform_format = MultiMesh.TRANSFORM_3D
	multi.instance_count = transforms.size()
	multi.mesh = mesh
	for index in transforms.size():
		multi.set_instance_transform(index, transforms[index])
	var batch := MultiMeshInstance3D.new()
	batch.multimesh = multi
	holder.add_child(batch)

func _add_tree_batch(holder: Node3D, transforms: Array[Transform3D]) -> void:
	if transforms.is_empty():
		return
	var mesh := CylinderMesh.new()
	mesh.top_radius = 0.15
	mesh.bottom_radius = 0.9
	mesh.height = 5.0
	var material := StandardMaterial3D.new()
	material.albedo_color = Color("385941")
	material.roughness = 1.0
	mesh.material = material
	var multi := MultiMesh.new()
	multi.transform_format = MultiMesh.TRANSFORM_3D
	multi.instance_count = transforms.size()
	multi.mesh = mesh
	for index in transforms.size():
		multi.set_instance_transform(index, transforms[index])
	var batch := MultiMeshInstance3D.new()
	batch.multimesh = multi
	holder.add_child(batch)

func _add_field_strips(holder: Node3D, size: Vector2) -> void:
	var strips: Array[Transform3D] = []
	for index in range(0, int(size.y / 12.0) + 1):
		var position := Vector3(size.x * 0.5, -0.02, float(index) * 12.0)
		var basis := Basis.IDENTITY.scaled(Vector3(size.x, 0.025, 6.0))
		strips.append(Transform3D(basis, position))
	_add_box_batch(holder, strips, Color("5d6b4f"), 1.0)

func _build_fog_layers() -> void:
	for scene_name in semantic_results:
		var frames: Array = semantic_results[scene_name]
		if frames.is_empty() or not frames[0].fog.has("bits"):
			continue
		var fog: Dictionary = frames[0].fog
		var nx := int(fog.nx)
		var ny := int(fog.ny)
		var cell_m := float(fog.cellM)
		var bits: Array = fog.bits
		var holder := Node3D.new()
		holder.name = "Fog_%s" % scene_name
		holder.visible = scene_name == String(scenes[0].map)
		add_child(holder)
		fog_nodes[scene_name] = holder
		var mesh := BoxMesh.new()
		mesh.size = Vector3(cell_m, 0.03, cell_m)
		var material := StandardMaterial3D.new()
		material.albedo_color = Color(0.015, 0.03, 0.06, 0.62)
		material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		mesh.material = material
		var count := 0
		var cells: Array[Node3D] = []
		for y in ny:
			for x in nx:
				if count >= 4096:
					break
				var index := y * nx + x
				var cell := MeshInstance3D.new()
				cell.mesh = mesh
				cell.position = Vector3((float(x) + 0.5) * cell_m, 0.02, (float(y) + 0.5) * cell_m)
				cell.visible = _fog_cell_hidden(bits, index)
				holder.add_child(cell)
				cells.append(cell)
				count += 1
		fog_rendered_cells[scene_name] = count
		fog_cell_nodes[scene_name] = cells

func _fog_cell_hidden(bits: Array, index: int) -> bool:
	var word_index := index / 32
	return word_index >= bits.size() or (int(bits[word_index]) & (1 << (index % 32))) == 0

func _update_fog_layer(scene_name: String, fog: Dictionary) -> void:
	var cells: Array = fog_cell_nodes.get(scene_name, [])
	var bits: Array = fog.get("bits", [])
	for index in mini(cells.size(), bits.size() * 32):
		var cell: Node3D = cells[index]
		cell.visible = _fog_cell_hidden(bits, index)

func _build_authored_maps(authored: PackedScene) -> bool:
	var directory := _authored_map_directory()
	for scene in scenes:
		var map_path := directory.path_join(String(scene.map)).path_join("map.json")
		var map_file := FileAccess.open(map_path, FileAccess.READ)
		if map_file == null:
			continue
		var map = JSON.parse_string(map_file.get_as_text())
		if typeof(map) != TYPE_DICTIONARY or typeof(map.get("buildings")) != TYPE_ARRAY:
			continue
		var holder := Node3D.new()
		holder.name = "AuthoredMap_%s" % scene.map
		holder.visible = scene.map == scenes[0].map
		add_child(holder)
		authored_scene_nodes[scene.map] = holder
		var count := 0
		for building in map.buildings:
			if authored_building_limit > 0 and count >= authored_building_limit:
				break
			var frame: Dictionary = building.get("frame", {})
			var translation: Array = frame.get("translation", [0.0, 0.0, 0.0])
			var building_scene: PackedScene = authored
			if building_scene == null:
				building_scene = _authored_scene_for_template(String(building.get("template_id", "")), String(map.get("regional_family", "")).to_lower())
			if building_scene == null:
				var unresolved_id := String(building.get("template_id", ""))
				authored_unresolved_templates[unresolved_id] = int(authored_unresolved_templates.get(unresolved_id, 0)) + 1
				continue
			var template_id := String(building.get("template_id", ""))
			var instance := _authored_shell_instance(building_scene, template_id)
			if instance == null:
				authored_unresolved_templates[template_id] = int(authored_unresolved_templates.get(template_id, 0)) + 1
				continue
			instance.position = Vector3(float(translation[0]), float(translation[2]), float(translation[1]))
			instance.rotation.y = float(frame.get("yaw", 0.0))
			holder.add_child(instance)
			count += 1
		authored_building_count += count
		authored_map_scene_count += 1
	return authored_map_scene_count > 0

func _authored_family_key(path: String) -> String:
	var filename := path.get_file().get_basename().to_lower()
	var family := filename
	if family == "kit" or family.is_empty():
		family = path.get_base_dir().get_file().to_lower()
	if family.ends_with("-kit"):
		family = family.trim_suffix("-kit")
	elif family.ends_with("_kit"):
		family = family.trim_suffix("_kit")
	if family.contains("paris-apartment"):
		return "paris_apartments"
	if family.contains("china-apartment"):
		return "china_apartments"
	return family.replace("-", "_")

func _authored_scene_for_template(template_id: String, regional_family: String) -> PackedScene:
	var lower := template_id.to_lower()
	if lower.contains("apartment"):
		return authored_scene_by_family.get(regional_family + "_apartments", authored_scene_by_family.get(regional_family, null))
	if lower.contains("farmstead"):
		return authored_scene_by_family.get("farmsteads", null)
	if lower.contains("shed") or lower.contains("warehouse") or lower.contains("works") or lower.contains("depot"):
		return authored_scene_by_family.get("industry", null)
	if lower.contains("tower"):
		return authored_scene_by_family.get("towers", null)
	if lower.contains("home") or lower.contains("terrace") or lower.contains("townhouse") or lower.contains("shop"):
		return authored_scene_by_family.get("homes", null)
	return authored_scene_by_family.get(regional_family, null)

func _authored_shell_token(template_id: String) -> String:
	var token := ""
	if template_id.contains("slab-35x11"):
		token = "slab_35x11_4f_shell"
	elif template_id.contains("slab-47x11"):
		token = "slab_47x11_5f_shell"
	elif template_id.contains("slab-59x14"):
		token = "slab_59x14_6f_shell"
	elif template_id.contains("slab-53x14"):
		token = "slab_53x14_8f_shell"
	elif template_id.contains("slab-47x14"):
		token = "slab_47x14_6f_shell"
	elif template_id.contains("slab-35x14"):
		token = "slab_35x14_8f_shell"
	elif template_id.contains("point-20x20"):
		token = "point_20x20_7f_shell"
	elif template_id.contains("block-u"):
		token = "block_u_5f_shell"
	elif template_id.contains("block-court"):
		token = "block_court_6f_shell"
	elif template_id.contains("corner-shop"):
		token = "corner_shop_3f_shell"
	elif template_id.contains("home-8x11"):
		token = "home_8x11_1f_shell"
	elif template_id.contains("home-9x9"):
		token = "home_9x9_2f_shell"
	elif template_id.contains("home-10x8"):
		token = "home_10x8_1f_shell"
	elif template_id.contains("home-12x9"):
		token = "home_12x9_2f_shell"
	elif template_id.contains("home-ell"):
		token = "home_ell_2f_shell"
	elif template_id.contains("townhouse"):
		token = "townhouse_2f_shell"
	elif template_id.contains("shops-4x3"):
		token = "shops_4x3f_unit"
	elif template_id.contains("terrace-3x2"):
		token = "terrace_3x2f_unit"
	elif template_id.contains("terrace-5x3"):
		token = "terrace_5x3f_unit"
	elif template_id.contains("farmstead-long"):
		token = "farm_long_shell"
	elif template_id.contains("farmstead-small"):
		token = "farm_small_shell"
	elif template_id.contains("farmstead-yard"):
		token = "farm_yard_shell"
	elif template_id.contains("depot"):
		token = "depot_shell"
	elif template_id.contains("shed"):
		token = "shed_shell"
	elif template_id.contains("warehouse"):
		token = "span_shell"
	elif template_id.contains("works"):
		token = "works_shell"
	return token

func _authored_shell_instance(scene: PackedScene, template_id: String) -> Node3D:
	var token := _authored_shell_token(template_id)
	if token.is_empty():
		return null
	var key := scene.resource_path + "|" + token
	if authored_shell_prototypes.has(key):
		return authored_shell_prototypes[key].duplicate() as Node3D
	var catalog := scene.instantiate()
	var shell := catalog.get_node_or_null(NodePath(token))
	if shell == null or not shell is Node3D:
		catalog.free()
		return null
	catalog.remove_child(shell)
	var prototype := shell.duplicate() as Node3D
	shell.free()
	catalog.free()
	authored_shell_prototypes[key] = prototype
	return prototype.duplicate() as Node3D

func _build_proxy_field() -> void:
	proxy_field_used = true
	var box := BoxMesh.new()
	box.size = Vector3(0.8, 0.8, 0.8)
	var material := StandardMaterial3D.new()
	material.albedo_color = Color("c28e55")
	box.material = material
	var multi := MultiMesh.new()
	multi.mesh = box
	multi.transform_format = MultiMesh.TRANSFORM_3D
	multi.instance_count = INSTANCE_COUNT
	for i in INSTANCE_COUNT:
		var x := float(i % 64) - 31.5
		var z := float(i / 64) - 31.5
		multi.set_instance_transform(i, Transform3D(Basis.IDENTITY, Vector3(x, 0.35, z)))
	var mesh_instance := MultiMeshInstance3D.new()
	mesh_instance.multimesh = multi
	add_child(mesh_instance)

func _process(delta: float) -> void:
	if not started:
		return
	intervals.append(max(delta, 0.000001))
	var scale := float(OS.get_environment("GODOT_REEL_TIME_SCALE"))
	if scale <= 0.0:
		scale = 1.0
	var advance := delta * scale
	_scene_pose()
	_update_authored_culling()
	_update_observed_units()
	if not cut_dir.is_empty() and shot_elapsed <= advance:
		pending_cut_saves += 1
		call_deferred("_save_cut_after_frame", scene_index, shot_index)
	elapsed += advance
	scene_elapsed += advance
	shot_elapsed += advance
	var scene: Dictionary = scenes[scene_index]
	var shots: Array = scene.reel.shots
	if shot_elapsed >= float(shots[shot_index].seconds):
		shot_elapsed -= float(shots[shot_index].seconds)
		shot_index += 1
		if shot_index >= shots.size():
			shot_index = 0
			scene_elapsed = 0.0
			scene_index += 1
			if scene_index >= scenes.size():
				started = false
				reel_finished = true
				call_deferred("_maybe_finish_reel")
			else:
				_show_scene()

func _update_observed_units() -> void:
	var scene: Dictionary = scenes[scene_index]
	var frames: Array = semantic_results.get(scene.map, [])
	if frames.is_empty():
		return
	var chosen: Dictionary = frames[0]
	var target_tick := int(capture_results.get(scene.map, {}).get("capture", {}).get("warmTick", 0)) + int(scene_elapsed * 30.0)
	for frame in frames:
		if int(frame.tick) > target_tick:
			break
		chosen = frame
	var units: Array = chosen.units
	var capture_side := String(capture_results.get(scene.map, {}).get("capture", {}).get("side", "blue"))
	var unit_color := Color("d95c5c") if capture_side == "red" else Color("4d9be6")
	_update_fog_layer(scene.map, chosen.get("fog", {}))
	while unit_nodes.size() < units.size():
		var holder := Node3D.new()
		var mesh := MeshInstance3D.new()
		var capsule := CapsuleMesh.new()
		capsule.radius = 0.5
		capsule.height = 2.0
		mesh.mesh = capsule
		var material := StandardMaterial3D.new()
		material.albedo_color = unit_color
		material.emission_enabled = true
		material.emission = unit_color
		material.emission_energy_multiplier = 0.25
		mesh.material_override = material
		mesh.position.y = 0.7
		holder.add_child(mesh)
		var contact := MeshInstance3D.new()
		var contact_mesh := CylinderMesh.new()
		contact_mesh.top_radius = 0.95
		contact_mesh.bottom_radius = 0.95
		contact_mesh.height = 0.05
		contact.mesh = contact_mesh
		var contact_material := StandardMaterial3D.new()
		contact_material.albedo_color = material.albedo_color
		contact_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		contact_material.albedo_color.a = 0.85
		contact.material_override = contact_material
		contact.position.y = 0.03
		holder.add_child(contact)
		get_node("ObservedUnits").add_child(holder)
		unit_nodes.append(holder)
	for i in unit_nodes.size():
		var node := unit_nodes[i]
		node.visible = i < units.size()
		if i >= units.size():
			continue
		var pose: Dictionary = units[i]
		var position: Array = pose.position
		node.position = Vector3(float(position[0]), max(0.0, float(position[2])), float(position[1]))
		node.rotation.y = float(pose.yaw)

func _show_scene() -> void:
	for map_name in fog_nodes:
		fog_nodes[map_name].visible = map_name == scenes[scene_index].map
	for map_name in map_geometry_nodes:
		map_geometry_nodes[map_name].visible = map_name == scenes[scene_index].map
	for map_name in authored_scene_nodes:
		authored_scene_nodes[map_name].visible = map_name == scenes[scene_index].map

func _update_authored_culling() -> void:
	var active_map := String(scenes[scene_index].map)
	var center := camera.position
	var target := Vector3.ZERO
	var captured := _captured_pose()
	if captured.has("target"):
		target = Vector3(float(captured.target[0]), 0.0, float(captured.target[1]))
	var radius := maxf(authored_cull_radius, camera.position.distance_to(target) * 20.0)
	for map_name in authored_scene_nodes:
		var holder: Node3D = authored_scene_nodes[map_name]
		var active: bool = map_name == active_map
		for child in holder.get_children():
			if child is Node3D:
				child.visible = active and child.position.distance_to(center) <= radius

func _scene_pose() -> void:
	var scene: Dictionary = scenes[scene_index]
	var captured := _captured_pose()
	if captured.size() > 0:
		_apply_pose(captured)
		return
	var shots: Array = scene.reel.shots
	var shot: Dictionary = shots[shot_index]
	var from: Dictionary = shot.from
	var to: Dictionary = shot.to
	var amount: float = clampf(shot_elapsed / float(shot.seconds), 0.0, 1.0)
	var target: Vector3 = Vector3(lerpf(float(from.target[0]), float(to.target[0]), amount), 0.0, lerpf(float(from.target[1]), float(to.target[1]), amount))
	var distance := lerpf(float(from.distance), float(to.distance), amount)
	var yaw := lerpf(float(from.yaw), float(to.yaw), amount)
	var pitch := lerpf(float(from.pitch), float(to.pitch), amount)
	var cp := cos(pitch)
	camera.position = target + Vector3(distance * cp * cos(yaw), distance * sin(pitch), distance * cp * sin(yaw))
	camera.look_at(target, Vector3.UP)

func _captured_pose() -> Dictionary:
	var scene: Dictionary = scenes[scene_index]
	var scene_capture: Dictionary = capture_results.get(scene.get("map", ""), capture_result)
	if not scene_capture.get("valid", false):
		return {}
	var capture: Dictionary = scene_capture.get("capture", {})
	var frames: Array = capture.get("frames", [])
	if frames.is_empty():
		return {}
	var chosen: Dictionary = frames[0]
	for frame in frames:
		if float(frame.get("elapsedMs", 0.0)) > scene_elapsed * 1000.0:
			break
		chosen = frame
	var pose = chosen.get("camera", {})
	return pose if typeof(pose) == TYPE_DICTIONARY else {}

func _apply_pose(pose: Dictionary) -> void:
	var target: Array = pose.get("target", [0.0, 0.0])
	var distance := float(pose.get("distance", 40.0))
	var yaw := float(pose.get("yaw", 0.0))
	var pitch := float(pose.get("pitch", 0.5))
	var cp := cos(pitch)
	var point := Vector3(float(target[0]), 0.0, float(target[1]))
	camera.position = point + Vector3(distance * cp * cos(yaw), distance * sin(pitch), distance * cp * sin(yaw))
	camera.look_at(point, Vector3.UP)

func _maybe_finish_reel() -> void:
	if not reel_finished or pending_cut_saves > 0:
		return
	_write_report()
	get_tree().quit()

func _save_cut_after_frame(scene_to_save: int, shot_to_save: int) -> void:
	var drawn := false
	var mark_drawn := func() -> void:
		drawn = true
	RenderingServer.frame_post_draw.connect(mark_drawn, CONNECT_ONE_SHOT)
	var deadline := Time.get_ticks_msec() + 2000
	while not drawn and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame
	if not drawn:
		cut_frame_timeouts += 1
	_save_cut(scene_to_save, shot_to_save)
	pending_cut_saves -= 1
	call_deferred("_maybe_finish_reel")

func _save_cut(scene_to_save: int, shot_to_save: int) -> void:
	DirAccess.make_dir_recursive_absolute(cut_dir)
	var image := _viewport_image()
	if image == null:
		return
	var path := cut_dir.path_join("scene-%02d-shot-%02d.png" % [scene_to_save, shot_to_save])
	if image.save_png(path) == OK:
		if not saved_cut_paths.has(path):
			saved_cut_paths[path] = true
			cuts_saved += 1

func _viewport_image() -> Image:
	if not _display_backed():
		return null
	var texture := get_viewport().get_texture()
	if texture == null:
		return null
	return texture.get_image()

func _display_backed() -> bool:
	return DisplayServer.get_name() != "headless"

func _write_report() -> void:
	if intervals.is_empty():
		return
	var sorted := intervals.duplicate()
	sorted.sort()
	var total := 0.0
	for value in intervals:
		total += value
	var slow_count := maxi(1, int(ceil(intervals.size() * 0.01)))
	var slow_sum := 0.0
	for i in slow_count:
		slow_sum += sorted[sorted.size() - 1 - i]
	var semantic_publications := 0
	var semantic_unit_samples := 0
	for frames in semantic_results.values():
		semantic_publications += frames.size()
		for frame in frames:
			semantic_unit_samples += frame.units.size()
	var expected_cuts := 0
	for scene in scenes:
		expected_cuts += scene.reel.shots.size()
	var report := {
		"schema": "godot-render-report/v1",
		"identity": {"client": "godot", "renderer": ProjectSettings.get_setting("rendering/renderer/rendering_method", "unknown"), "display_server": DisplayServer.get_name(), "display_backed": _display_backed(), "viewport": [ProjectSettings.get_setting("display/window/size/viewport_width", 0), ProjectSettings.get_setting("display/window/size/viewport_height", 0)], "quality": "current-project-settings"},
		"candidate": "godot-menu-reel-camera-probe",
		"comparison_ready": false,
		"comparison_blocker": _comparison_blocker(),
		"authored_asset_loaded": authored_asset_loaded,
		"authored_catalog_complete": authored_asset_loaded and authored_unresolved_templates.is_empty(),
		"authored_building_count": authored_building_count,
		"authored_map_scene_count": authored_map_scene_count,
		"authored_building_limit": authored_building_limit,
		"authored_cull_radius": authored_cull_radius,
		"map_render_radius": map_render_radius,
		"authored_unresolved_templates": authored_unresolved_templates,
		"map_geometry": map_geometry_counts,
		"fog_rendered_cells": fog_rendered_cells,
		"proxy_field_used": proxy_field_used,
		"cuts_saved": cuts_saved,
		"cut_frame_timeouts": cut_frame_timeouts,
		"expected_cuts": expected_cuts,
		"named_cuts_complete": cuts_saved == expected_cuts,
		"capture_valid": not capture_results.is_empty(),
		"capture_scene_count": capture_results.size(),
		"capture_comparison_ready": capture_results.size() == scenes.size(),
		"capture_consumed": capture_results.values().any(func(result): return result.sample_count > 0),
		"capture_layout_valid": capture_layout_valid,
		"capture_publication_words": capture_word_count,
		"semantic_publications": semantic_publications,
		"semantic_unit_samples": semantic_unit_samples,
		"semantic_fog_publications": semantic_fog_publications,
		"capture_errors": capture_errors,
		"capture_blocker": "" if capture_results.size() == scenes.size() else ("one or more scene captures were rejected: %s" % JSON.stringify(capture_errors) if not capture_errors.is_empty() else "one or more scene captures are missing"),
		"source": source_path,
		"authored_scene": OS.get_environment("GODOT_AUTHORED_SCENE"),
		"authored_scenes": OS.get_environment("GODOT_AUTHORED_SCENES"),
		"scene_count": scenes.size(),
		"scene_ids": scenes.map(func(s): return {"map": s.map, "encounter": s.encounter, "seed": s.seed}),
		"instance_count": 0 if not proxy_field_used else INSTANCE_COUNT,
		"average_fps": float(intervals.size()) / total,
		"minimum_fps": 1.0 / sorted[-1],
		"maximum_fps": 1.0 / sorted[0],
		"one_percent_low_fps": 1.0 / (slow_sum / slow_count),
		"frame_intervals_s": intervals,
		"timings_ms": {"startup": startup_ms, "capture_decode": semantic_decode_ms, "simulation": null, "transfer": null, "cpu_submission": null, "gpu": null, "presentation": total * 1000.0 / intervals.size(), "shader_compile": null, "memory": null},
		"measurement": "Displayed-frame intervals are measured by Godot process frames. Simulation, GPU, transfer, shader and memory timings are null until the native extension and a real display-backed run provide them.",
	}
	var output_path := OS.get_environment("GODOT_REEL_REPORT")
	if output_path.is_empty():
		output_path = "user://godot-menu-reel-probe.json"
	elif not output_path.begins_with("user://"):
		output_path = _resolve_path(output_path)
	var file := FileAccess.open(output_path, FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify(report, "  "))
	print(JSON.stringify(report))

func _comparison_blocker() -> String:
	if not authored_asset_loaded:
		return "authored model catalog is not loaded; map geometry and sampled units are rendered"
	if not authored_unresolved_templates.is_empty():
		return "authored template modules are unresolved; map geometry and sampled units are rendered"
	return "authored materials and display-backed visual equivalence remain unproven"
