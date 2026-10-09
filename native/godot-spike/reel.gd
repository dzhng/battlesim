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
var authored_asset_loaded := false
var authored_building_count := 0
var authored_map_scene_count := 0
var authored_building_limit := 256
var authored_scene_nodes: Dictionary = {}
var authored_scene_by_family: Dictionary = {}
var cut_dir := ""
var cuts_saved := 0
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

func _ready() -> void:
	startup_started_usec = Time.get_ticks_usec()
	var configured := OS.get_environment("GODOT_REEL_SOURCE")
	source_path = configured if not configured.is_empty() else ProjectSettings.globalize_path(DEFAULT_SOURCE)
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
	cut_dir = OS.get_environment("GODOT_REEL_CUTS")
	_build_world()
	startup_ms = float(Time.get_ticks_usec() - startup_started_usec) / 1000.0
	started = true
	if OS.get_environment("GODOT_EXIT_AFTER_READY") == "1":
		call_deferred("_exit_after_ready")

func _exit_after_ready() -> void:
	print(JSON.stringify({"map_geometry": map_geometry_counts, "semantic_scenes": semantic_results.size()}))
	get_tree().quit()

func _load_presentation_captures() -> void:
	var configured := OS.get_environment("GODOT_PRESENTATION_CAPTURE")
	var directory := OS.get_environment("GODOT_PRESENTATION_CAPTURE_DIR")
	if not directory.is_empty():
		for scene in scenes:
			var path := directory.path_join(String(scene.map) + ".json")
			var result := _read_capture(path)
			if result.valid:
				capture_results[scene.map] = result
		return
	if not configured.is_empty():
		capture_result = _read_capture(configured)
		if capture_result.valid:
			capture_results[capture_result.capture.workload.scene] = capture_result

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
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color("101721")
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color("8aa4bc")
	env.ambient_light_energy = 0.8
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
	if not authored_paths.is_empty():
		for path in authored_paths.split(","):
			var clean_path := path.strip_edges()
			var scene_resource = load(clean_path)
			if scene_resource is PackedScene:
				var filename := clean_path.get_file().get_basename().to_lower()
				var family_key := "paris" if filename.contains("paris") else ("china" if filename.contains("china") else filename.split("-")[0].split("_")[0])
				authored_scene_by_family[family_key] = scene_resource
		if _build_authored_maps(null):
			authored_asset_loaded = true
			return
	if not authored_path.is_empty():
		var authored = load(authored_path)
		if authored is PackedScene:
			var limit_text := OS.get_environment("GODOT_AUTHORED_BUILDING_LIMIT")
			if not limit_text.is_empty():
				authored_building_limit = maxi(0, int(limit_text))
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
	var directory := OS.get_environment("GODOT_AUTHORED_MAP_DIR")
	if directory.is_empty():
		return
	var prop_limit := int(OS.get_environment("GODOT_MAP_PROP_LIMIT"))
	if prop_limit <= 0:
		prop_limit = 512
	var building_limit := int(OS.get_environment("GODOT_MAP_BUILDING_LIMIT"))
	if building_limit <= 0:
		building_limit = 512
	var road_limit := int(OS.get_environment("GODOT_MAP_ROAD_LIMIT"))
	if road_limit <= 0:
		road_limit = 1000
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
		var counts := {"terrain": 0, "roads": 0, "props": 0, "buildings": 0}
		var size: Array = map.get("size", [100.0, 100.0])
		var ground := MeshInstance3D.new()
		var ground_mesh := PlaneMesh.new()
		ground_mesh.size = Vector2(float(size[0]), float(size[1]))
		var ground_material := StandardMaterial3D.new()
		ground_material.albedo_color = Color("43564a") if String(map.get("regional_family", "")) == "china" else Color("55585b")
		ground_mesh.material = ground_material
		ground.mesh = ground_mesh
		ground.position = Vector3(float(size[0]) * 0.5, -0.08, float(size[1]) * 0.5)
		holder.add_child(ground)
		counts.terrain = 1
		for surface in map.get("surfaces", []):
			if typeof(surface) != TYPE_DICTIONARY or typeof(surface.get("shape")) != TYPE_DICTIONARY:
				continue
			var shape: Dictionary = surface.shape
			if shape.get("kind") != "stroke" or typeof(shape.get("points")) != TYPE_ARRAY:
				continue
			var points: Array = shape.points
			for i in range(min(max(0, points.size() - 1), road_limit - counts.roads)):
				var a: Array = points[i]
				var b: Array = points[i + 1]
				var start := Vector2(float(a[0]), float(a[1]))
				var end := Vector2(float(b[0]), float(b[1]))
				var length := start.distance_to(end)
				if length <= 0.01:
					continue
				var road := MeshInstance3D.new()
				var road_mesh := BoxMesh.new()
				road_mesh.size = Vector3(length, 0.035, float(shape.get("width_m", 8.0)))
				var road_material := StandardMaterial3D.new()
				road_material.albedo_color = Color("252a2d")
				road_mesh.material = road_material
				road.mesh = road_mesh
				road.position = Vector3((start.x + end.x) * 0.5, 0.0, (start.y + end.y) * 0.5)
				road.rotation.y = -atan2(end.y - start.y, end.x - start.x)
				holder.add_child(road)
				counts.roads += 1
		for building in map.get("buildings", []):
			if counts.buildings >= building_limit:
				break
			if typeof(building) != TYPE_DICTIONARY:
				continue
			var frame: Dictionary = building.get("frame", {})
			var building_node := MeshInstance3D.new()
			var building_mesh := BoxMesh.new()
			building_mesh.size = Vector3(18.0, 8.0, 18.0)
			var building_material := StandardMaterial3D.new()
			building_material.albedo_color = Color("8b8b83") if counts.buildings % 2 == 0 else Color("6f7377")
			building_mesh.material = building_material
			building_node.mesh = building_mesh
			var translation: Array = frame.get("translation", [0.0, 0.0, 0.0])
			building_node.position = Vector3(float(translation[0]), 4.0 + float(translation[2]), float(translation[1]))
			building_node.rotation.y = float(frame.get("yaw", 0.0))
			holder.add_child(building_node)
			counts.buildings += 1
		for prop in map.get("props", []):
			if counts.props >= prop_limit or typeof(prop) != TYPE_DICTIONARY:
				break
			var center: Array = prop.get("center", [0.0, 0.0])
			var half: Array = prop.get("half_extents", [1.0, 1.0, 0.5])
			var prop_node := MeshInstance3D.new()
			var prop_mesh := BoxMesh.new()
			prop_mesh.size = Vector3(max(0.2, float(half[0]) * 2.0), max(0.2, float(half[2]) * 2.0), max(0.2, float(half[1]) * 2.0))
			var prop_material := StandardMaterial3D.new()
			prop_material.albedo_color = Color("7d6a50")
			prop_mesh.material = prop_material
			prop_node.mesh = prop_mesh
			prop_node.position = Vector3(float(center[0]), float(half[2]), float(center[1]))
			prop_node.rotation.y = float(prop.get("yaw", 0.0))
			holder.add_child(prop_node)
			counts.props += 1
		counts["building_limit"] = building_limit
		counts["prop_limit"] = prop_limit
		counts["road_limit"] = road_limit
		map_geometry_counts[map_name] = counts

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
		material.albedo_color = Color(0.02, 0.04, 0.07, 0.42)
		material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		mesh.material = material
		var count := 0
		for y in ny:
			for x in nx:
				if count >= 4096:
					break
				var index := y * nx + x
				if index / 32 >= bits.size() or (int(bits[index / 32]) & (1 << (index % 32))) != 0:
					continue
				var cell := MeshInstance3D.new()
				cell.mesh = mesh
				cell.position = Vector3((float(x) + 0.5) * cell_m, 0.02, (float(y) + 0.5) * cell_m)
				holder.add_child(cell)
				count += 1
		fog_rendered_cells[scene_name] = count

func _build_authored_maps(authored: PackedScene) -> bool:
	var directory := OS.get_environment("GODOT_AUTHORED_MAP_DIR")
	if directory.is_empty():
		return false
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
				var family := String(map.get("regional_family", "")).to_lower()
				building_scene = authored_scene_by_family.get(family)
			if building_scene == null:
				continue
			var instance = building_scene.instantiate()
			instance.position = Vector3(float(translation[0]), float(translation[2]), float(translation[1]))
			instance.rotation.y = float(frame.get("yaw", 0.0))
			holder.add_child(instance)
			count += 1
		authored_building_count += count
		authored_map_scene_count += 1
	return authored_map_scene_count > 0

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
	_update_observed_units()
	if not cut_dir.is_empty() and shot_elapsed <= advance:
		call_deferred("_save_cut")
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
				_write_report()
				get_tree().quit()
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
	while unit_nodes.size() < units.size():
		var mesh := MeshInstance3D.new()
		var capsule := CapsuleMesh.new()
		capsule.radius = 0.35
		capsule.height = 1.4
		mesh.mesh = capsule
		var material := StandardMaterial3D.new()
		material.albedo_color = Color("4d9be6") if unit_nodes.size() % 2 == 0 else Color("d95c5c")
		mesh.material_override = material
		get_node("ObservedUnits").add_child(mesh)
		unit_nodes.append(mesh)
	for i in unit_nodes.size():
		var node := unit_nodes[i]
		node.visible = i < units.size()
		if i >= units.size():
			continue
		var pose: Dictionary = units[i]
		var position: Array = pose.position
		node.position = Vector3(float(position[0]), max(0.7, float(position[2]) + 0.7), float(position[1]))
		node.rotation.y = float(pose.yaw)

func _show_scene() -> void:
	for map_name in fog_nodes:
		fog_nodes[map_name].visible = map_name == scenes[scene_index].map
	for map_name in map_geometry_nodes:
		map_geometry_nodes[map_name].visible = map_name == scenes[scene_index].map
	for map_name in authored_scene_nodes:
		authored_scene_nodes[map_name].visible = map_name == scenes[scene_index].map

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

func _save_cut() -> void:
	DirAccess.make_dir_recursive_absolute(cut_dir)
	var texture := get_viewport().get_texture()
	if texture == null:
		return
	var image := texture.get_image()
	if image == null:
		return
	var path := cut_dir.path_join("scene-%02d-shot-%02d.png" % [scene_index, shot_index])
	if image.save_png(path) == OK:
		cuts_saved += 1

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
	var report := {
		"schema": "godot-render-report/v1",
		"identity": {"client": "godot", "renderer": ProjectSettings.get_setting("rendering/renderer/rendering_method", "unknown"), "viewport": [ProjectSettings.get_setting("display/window/size/viewport_width", 0), ProjectSettings.get_setting("display/window/size/viewport_height", 0)], "quality": "current-project-settings"},
		"candidate": "godot-menu-reel-camera-probe",
		"comparison_ready": false,
		"comparison_blocker": "authored model catalog is not loaded; map geometry and sampled units are rendered" if not authored_asset_loaded else "authored materials and full catalog are incomplete; map geometry and sampled unit publications are rendered",
		"authored_asset_loaded": authored_asset_loaded,
		"authored_building_count": authored_building_count,
		"authored_map_scene_count": authored_map_scene_count,
		"authored_building_limit": authored_building_limit,
		"map_geometry": map_geometry_counts,
		"fog_rendered_cells": fog_rendered_cells,
		"proxy_field_used": proxy_field_used,
		"cuts_saved": cuts_saved,
		"capture_valid": not capture_results.is_empty(),
		"capture_scene_count": capture_results.size(),
		"capture_comparison_ready": capture_results.size() == scenes.size(),
		"capture_consumed": capture_results.values().any(func(result): return result.sample_count > 0),
		"capture_layout_valid": capture_layout_valid,
		"capture_publication_words": capture_word_count,
		"semantic_publications": semantic_publications,
		"semantic_unit_samples": semantic_unit_samples,
		"semantic_fog_publications": semantic_fog_publications,
		"capture_blocker": "" if capture_results.size() == scenes.size() else "one or more scene captures are missing",
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
	var file := FileAccess.open(output_path, FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify(report, "  "))
	print(JSON.stringify(report))
