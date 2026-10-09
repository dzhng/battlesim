extends Node3D

const BENCHMARK_SECONDS := 10.0
const INSTANCE_COUNT := 4096

var elapsed := 0.0
var samples: Array[float] = []
var mesh_instance: MultiMeshInstance3D
var camera: Camera3D

func _ready() -> void:

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
	camera.position = Vector3(0.0, 18.0, 30.0)
	camera.look_at_from_position(camera.position, Vector3.ZERO)
	add_child(camera)

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
		var y := 0.35 + sin(float(i) * 0.19) * 0.25
		multi.set_instance_transform(i, Transform3D(Basis.IDENTITY, Vector3(x, y, z)))
	mesh_instance = MultiMeshInstance3D.new()
	mesh_instance.multimesh = multi
	add_child(mesh_instance)

func _process(delta: float) -> void:
	elapsed += delta
	camera.position = Vector3(sin(elapsed * 0.18) * 34.0, 18.0, cos(elapsed * 0.18) * 34.0)
	camera.look_at(Vector3.ZERO)
	if elapsed > 1.0:
		samples.append(1.0 / max(delta, 0.000001))
	if elapsed >= BENCHMARK_SECONDS + 1.0:
		_write_report()
		get_tree().quit()

func _write_report() -> void:
	if samples.is_empty():
		return
	var sorted := samples.duplicate()
	sorted.sort()
	var total := 0.0
	for value in samples:
		total += value
	var report := {
		"candidate": "godot-reference-scene",
		"renderer": ProjectSettings.get_setting("rendering/renderer/rendering_method", "unknown"),
		"instances": INSTANCE_COUNT,
		"duration_s": BENCHMARK_SECONDS,
		"average_fps": total / samples.size(),
		"minimum_fps": sorted[0],
		"maximum_fps": sorted[-1],
		"one_percent_low_fps": sorted[max(0, int(ceil(samples.size() * 0.01)) - 1)],
		"frame_samples": samples,
	}
	var file := FileAccess.open("user://godot-spike-report.json", FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify(report, "  "))
	print(JSON.stringify(report))

