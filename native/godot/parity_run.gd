extends Node3D

## The parity run: renders one scene of a browser capture run, frame record by
## frame record, and writes a plate per record and a report.
##
##   godot --path native/godot --window-position 10000,10000 -- \
##     --run <dir> --scene <name> [--frames plates|all|<i,j,...>] [--maps <dir>] [--out <dir>]
##
## `<dir>/<name>/` holds the browser's `capture.json` and `manifest.json`.
## Relative paths resolve from the repository root. Plates and `report.json` go
## to `--out`, by default `<dir>/<name>/native/`. Rendering needs a display:
## under `--headless` the run stops at a blocker. Any error Godot logs during
## the run is a blocker too. Exits 0 with no blocker,
## 1 with a blocker (named in the report), 2 on bad arguments.

const Capture = preload("res://capture/presentation_capture.gd")
const Manifest = preload("res://capture/visual_manifest.gd")
const Stepper = preload("res://reel/frame_stepper.gd")
const PlateWriter = preload("res://reel/plate_writer.gd")
const RenderReport = preload("res://report/render_report.gd")
const Staging = preload("res://assets/staging.gd")
const EnvironmentAdapter = preload("res://environment/environment_adapter.gd")
const LegacyMapLayer = preload("res://world/legacy_map_layer.gd")
const FogLayer = preload("res://world/fog_layer.gd")
const CameraRig = preload("res://reel/camera_rig.gd")
const Light = preload("res://environment/light.gd")
const ErrorLog = preload("res://report/error_log.gd")

const OWNER := "native/parity_run"
## The browser's presentation layers native can draw; the manifest decides
## which are drawn. `models` is the observed units.
const LAYERS := ["sky", "terrain", "grass", "buildings", "props", "models", "fog"]
const USAGE := "usage: godot --path native/godot --window-position 10000,10000 -- --run <dir> --scene <name> [--frames plates|all|<i,j,...>] [--maps <dir>] [--out <dir>]"

var _report := RenderReport.new()

func _ready() -> void:
	get_tree().quit(await _run(OS.get_cmdline_user_args()))

func _run(args: PackedStringArray) -> int:
	var options := parse_args(args)
	if options.has("error"):
		printerr("%s\n%s" % [options.error, USAGE])
		return 2
	_report.frames = options.frames
	var errors := ErrorLog.new()
	OS.add_logger(errors)
	await _render(options)
	OS.remove_logger(errors)
	for error in errors.errors():
		_report.add_blocker("error", error)
	var report_path: String = options.out.path_join("report.json")
	if _report.write(report_path) != OK:
		printerr("could not write " + report_path)
		return 1
	var summary := _report.to_dictionary()
	print(JSON.stringify({"report": report_path, "plates": summary.plates.size(), "blockers": summary.blockers}))
	return 0 if _report.ok() else 1

## Options from the user arguments, or `{error}`.
static func parse_args(args: PackedStringArray) -> Dictionary:
	var values := {"--frames": "plates", "--maps": "fixtures/maps"}
	var at := 0
	while at < args.size():
		if not args[at] in ["--run", "--scene", "--frames", "--maps", "--out"] or at + 1 >= args.size():
			return {"error": "unexpected argument %s" % args[at]}
		values[args[at]] = args[at + 1]
		at += 2
	for required in ["--run", "--scene"]:
		if not values.has(required):
			return {"error": "%s is required" % required}
	var scene_dir := Staging.repo_path(values["--run"]).path_join(values["--scene"])
	return {
		"scene_dir": scene_dir,
		"scene": values["--scene"],
		"frames": values["--frames"],
		"maps": Staging.repo_path(values["--maps"]),
		"out": Staging.repo_path(values["--out"]) if values.has("--out") else scene_dir.path_join("native"),
	}

func _render(options: Dictionary) -> void:
	if DisplayServer.get_name() == "headless":
		_report.add_blocker("display", "rendering needs a display; run without --headless and with --window-position 10000,10000")
		return
	var admitted := Manifest.load(options.scene_dir, options.maps)
	if not admitted.ok:
		_report.add_blocker(admitted.blocker.name, admitted.blocker.detail)
		return
	var manifest: Dictionary = admitted.manifest
	_report.identity = manifest.identity
	if manifest.identity.scene != options.scene:
		_report.add_blocker("identity", "the manifest is for scene %s, not %s" % [manifest.identity.scene, options.scene])
		return
	var capture: Dictionary = admitted.capture.capture
	var selected := Stepper.select(capture.frames, options.frames)
	var decoded := Capture.decode_samples(admitted.capture)
	var map = JSON.parse_string(FileAccess.get_file_as_string(admitted.map_path))
	for refused in [selected, decoded]:
		if not refused.ok:
			_report.add_blocker(refused.blocker.name, refused.blocker.detail)
			return
	if typeof(map) != TYPE_DICTIONARY:
		_report.add_blocker("map", "%s is not a JSON object" % admitted.map_path)
		return

	var stepper = selected.stepper
	var targets: Array = stepper.records().map(func(record): return Vector2(float(record.target[0]), float(record.target[1])))
	var world := compose(self, manifest, decoded.samples, map, targets, capture.side, _report)
	if not _report.ok():
		return

	get_window().size = Vector2i(int(manifest.viewport.width), int(manifest.viewport.height))
	var camera := Camera3D.new()
	add_child(camera)
	camera.make_current()
	camera.far = CameraRig.far_plane(map.size, float(manifest.light.backdrop.reach_m))
	_report.add_budget(OWNER, "approximation: a finite far plane at %d m (the map's diagonal plus the backdrop reach); the browser's is infinite" % camera.far, "lens.far", 1, 1, 0)

	var record = stepper.step()
	while record != null:
		var sample := _sample_at(decoded.samples, int(record.tick))
		if sample.is_empty():
			_report.add_blocker("publication", "frame %d at tick %d precedes the first sample" % [record.index, record.tick])
			return
		CameraRig.apply(camera, record, manifest.lens)
		world.environment.fit(camera, Vector2(get_window().size))
		world.layer.show_units(sample.units)
		if world.fog != null:
			world.fog.show_fog(sample.fog)
		var path: String = options.out.path_join("plate-%d.png" % record.index)
		var plate: Dictionary = await PlateWriter.write(get_viewport(), record, path)
		if not plate.ok:
			_report.add_blocker(plate.blocker.name, plate.blocker.detail)
			return
		_report.add_plate(int(record.index), path, plate.gpu_ms)
		record = stepper.step()

## Builds the light and every layer the manifest lists under `root`, staging the
## assets the layers draw. Returns `{environment, layer, fog}`; `fog` is null
## unless listed.
## Blockers and budgets go to `report`.
static func compose(root: Node3D, manifest: Dictionary, samples: Array, map: Dictionary, targets: Array, side: String, report) -> Dictionary:
	var layers: Array = manifest.layers
	for layer in layers:
		if not LAYERS.has(layer):
			report.add_budget(OWNER, "the browser drew this layer; native does not draw it", "layer:" + layer, 1, 0, 1)
	# Shadows fall on the map, flat until terrain heights are drawn (05a), and
	# on what stands on it.
	var size := Vector2(float(map.size[0]), float(map.size[1]))
	var receivers := AABB(Vector3(0.0, -1.0, -size.y), Vector3(size.x, 1.0 + Light.STANDING_M, size.y))
	var environment := EnvironmentAdapter.new(manifest.light, layers.has("sky"), receivers)
	root.add_child(environment)
	var unit_ids := {}
	for sample in samples:
		for unit in sample.units:
			if not unit_ids.has(unit.kindName):
				unit_ids[unit.kindName] = {}
			unit_ids[unit.kindName][unit.id] = true
	var unit_kinds := {}
	for kind in unit_ids:
		unit_kinds[kind] = unit_ids[kind].size()
	var layer := LegacyMapLayer.new(map, targets, unit_kinds)
	var staged := Staging.stage(layer.required_assets(layers), report)
	root.add_child(layer)
	var fog: Node3D = null
	if report.ok():
		layer.build(layers, staged, side, report)
		if layers.has("fog"):
			fog = FogLayer.new()
			root.add_child(fog)
	return {"environment": environment, "layer": layer, "fog": fog}

## The latest decoded sample at or before `tick`, or empty.
static func _sample_at(samples: Array, tick: int) -> Dictionary:
	var chosen := {}
	for sample in samples:
		if sample.tick > tick:
			break
		chosen = sample
	return chosen
