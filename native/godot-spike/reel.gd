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
var authored_asset_loaded := false
var cut_dir := ""
var cuts_saved := 0

func _ready() -> void:
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
	_load_presentation_capture()
	cut_dir = OS.get_environment("GODOT_REEL_CUTS")
	_build_world()
	started = true

func _load_presentation_capture() -> void:
	var configured := OS.get_environment("GODOT_PRESENTATION_CAPTURE")
	if configured.is_empty():
		return
	var capture_file := FileAccess.open(configured, FileAccess.READ)
	if capture_file == null:
		capture_result = {"valid": false, "comparison_ready": false, "comparison_blocker": "unable to read presentation capture: " + configured}
		return
	capture_result = CaptureDecoder.decode_json(capture_file.get_as_text())

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
	var authored_path := OS.get_environment("GODOT_AUTHORED_SCENE")
	if not authored_path.is_empty():
		var authored = load(authored_path)
		if authored is PackedScene:
			var instance = authored.instantiate()
			add_child(instance)
			authored_asset_loaded = true
			return
	_build_proxy_field()

func _build_proxy_field() -> void:
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
	if not capture_result.get("valid", false):
		return {}
	var capture: Dictionary = capture_result.get("capture", {})
	var workload: Dictionary = capture.get("workload", {})
	var scene: Dictionary = scenes[scene_index]
	if workload.get("scene", "") != scene.get("map", ""):
		return {}
	var samples: Array = capture.get("samples", [])
	if samples.is_empty():
		return {}
	var target_tick := int(capture.get("warmTick", 0)) + int(scene_elapsed * int(capture.get("tickHz", 1)))
	var chosen: Dictionary = samples[0]
	for sample in samples:
		if int(sample.get("tick", 0)) > target_tick:
			break
		chosen = sample
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
	var report := {
		"schema": "godot-render-report/v1",
		"candidate": "godot-menu-reel-camera-probe",
		"comparison_ready": false,
		"comparison_blocker": "authored map assets are not loaded" if not authored_asset_loaded else "synthetic scene has no authored map publication",
		"authored_asset_loaded": authored_asset_loaded,
		"cuts_saved": cuts_saved,
		"capture_valid": capture_result.valid,
		"capture_comparison_ready": capture_result.comparison_ready,
		"capture_consumed": capture_result.valid and capture_result.sample_count > 0,
		"capture_blocker": capture_result.comparison_blocker,
		"source": source_path,
		"authored_scene": OS.get_environment("GODOT_AUTHORED_SCENE"),
		"scene_count": scenes.size(),
		"scene_ids": scenes.map(func(s): return {"map": s.map, "encounter": s.encounter, "seed": s.seed}),
		"instance_count": 0 if authored_asset_loaded else INSTANCE_COUNT,
		"average_fps": float(intervals.size()) / total,
		"minimum_fps": 1.0 / sorted[-1],
		"maximum_fps": 1.0 / sorted[0],
		"one_percent_low_fps": 1.0 / (slow_sum / slow_count),
		"frame_intervals_s": intervals,
	}
	var output_path := OS.get_environment("GODOT_REEL_REPORT")
	if output_path.is_empty():
		output_path = "user://godot-menu-reel-probe.json"
	var file := FileAccess.open(output_path, FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify(report, "  "))
	print(JSON.stringify(report))
