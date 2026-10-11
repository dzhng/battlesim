extends Node3D

## The parity run: renders one scene of a browser capture run, frame record by
## frame record, and writes a plate per record and a report.
##
##   godot --path native/godot --window-position 10000,10000 -- \
##     --run <dir> --scene <name> [--frames plates|all|<i,j,...>] [--maps <dir>] [--out <dir>]
##     [--bench <warm-up frames> [--pace uncapped|display]]
##
## `<dir>/<name>/` holds the browser's `capture.json` and `manifest.json`.
## Relative paths resolve from the repository root. Plates and `report.json` go
## to `--out`, by default `<dir>/<name>/native/`. `--bench` times instead of
## shooting: it draws every record (or `--frames`) once, uncapped or with
## `--pace display` at the display's rate, reads nothing back, and reports
## each frame's interval and CPU time after the warm-up
## (`report/frame_timing.gd`), by default in `<dir>/<name>/native-bench/`.
## Rendering needs a display:
## under `--headless` the run stops at a blocker. Any error Godot logs during
## the run is a blocker too. Exits 0 with no blocker, 1 with a blocker (named
## in the report), 2 on bad arguments.

const Capture = preload("res://capture/presentation_capture.gd")
const Manifest = preload("res://capture/visual_manifest.gd")
const Stepper = preload("res://reel/frame_stepper.gd")
const PlateWriter = preload("res://reel/plate_writer.gd")
const RenderReport = preload("res://report/render_report.gd")
const Staging = preload("res://assets/staging.gd")
const EnvironmentAdapter = preload("res://environment/environment_adapter.gd")
const StaticWorld = preload("res://world/static_world.gd")
const UnitLayer = preload("res://units/unit_layer.gd")
const TerrainLayer = preload("res://terrain/terrain_layer.gd")
const GrassLayer = preload("res://grass/grass_layer.gd")
const HeightField = preload("res://terrain/height_field.gd")
const FogLayer = preload("res://world/fog_layer.gd")
const CameraRig = preload("res://reel/camera_rig.gd")
const Light = preload("res://environment/light.gd")
const ErrorLog = preload("res://report/error_log.gd")
const LearnedGround = preload("res://capture/learned_ground.gd")
const ScarTexture = preload("res://terrain/scar_texture.gd")
const StrictJson = preload("res://core/strict_json.gd")
const FrameTiming = preload("res://report/frame_timing.gd")
const EffectLayer = preload("res://effects/effect_layer.gd")

const OWNER := "native/parity_run"
## The browser's presentation layers native can draw; the manifest decides
## which are drawn. `models` is the units (`units/unit_layer.gd`);
## `structures` the fitted props (`world/prop_layer.gd`); `effects` the
## combat effects (`effects/effect_layer.gd`).
const LAYERS := ["sky", "terrain", "water", "scars", "grass", "scenery", "structures", "buildings", "models", "effects", "fog"]
const USAGE := "usage: godot --path native/godot -- --run <dir> --scene <name> [--frames plates|all|<i,j,...>] [--maps <dir>] [--out <dir>] [--bench <warm-up frames> [--pace uncapped|display]]"
## A run that is still going after this long has stalled: it stops at a
## `timeout` blocker and frees the GPU lock. A whole scene benched at 80 ms a
## frame takes about 12 minutes.
const RUN_LIMIT_S := 1200.0

var _report := RenderReport.new()

func _ready() -> void:
	get_tree().quit(await _run(OS.get_cmdline_user_args()))

func _run(args: PackedStringArray) -> int:
	var options := parse_args(args)
	if options.has("error"):
		printerr("%s\n%s" % [options.error, USAGE])
		return 2
	get_tree().create_timer(RUN_LIMIT_S, true, false, true).timeout.connect(_out_of_time.bind(options.out.path_join("report.json")))
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
	var timed: int = summary.bench.intervalsMs.size() if summary.bench != null else 0
	print(JSON.stringify({"report": report_path, "plates": summary.plates.size(), "timedFrames": timed, "blockers": summary.blockers}))
	return 0 if _report.ok() else 1

func _out_of_time(report_path: String) -> void:
	_report.add_blocker("timeout", "the run was still going after %d s" % RUN_LIMIT_S)
	_report.write(report_path)
	printerr("timeout: the run was still going after %d s; see %s" % [RUN_LIMIT_S, report_path])
	get_tree().quit(1)

## Options from the user arguments, or `{error}`. `bench` is the warm-up
## frame count of a bench run, or -1 for a plate run.
static func parse_args(args: PackedStringArray) -> Dictionary:
	var values := {"--maps": "fixtures/maps"}
	var at := 0
	while at < args.size():
		if not args[at] in ["--run", "--scene", "--frames", "--maps", "--out", "--bench", "--pace"] or at + 1 >= args.size():
			return {"error": "unexpected argument %s" % args[at]}
		values[args[at]] = args[at + 1]
		at += 2
	for required in ["--run", "--scene"]:
		if not values.has(required):
			return {"error": "%s is required" % required}
	var bench := -1
	if values.has("--bench"):
		if not values["--bench"].is_valid_int() or int(values["--bench"]) < 0:
			return {"error": "--bench takes a warm-up frame count, not %s" % values["--bench"]}
		bench = int(values["--bench"])
	var pace: String = values.get("--pace", "uncapped")
	if not pace in ["uncapped", "display"] or (values.has("--pace") and bench < 0):
		return {"error": "--pace is uncapped or display, and only for --bench"}
	var scene_dir := Staging.repo_path(values["--run"]).path_join(values["--scene"])
	return {
		"scene_dir": scene_dir,
		"scene": values["--scene"],
		"frames": values.get("--frames", "all" if bench >= 0 else "plates"),
		"bench": bench,
		"pace": pace,
		"maps": Staging.repo_path(values["--maps"]),
		"out": Staging.repo_path(values["--out"]) if values.has("--out") else scene_dir.path_join("native-bench" if bench >= 0 else "native"),
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
	var ground := read_ground(options.scene_dir, manifest)
	if not ground.ok:
		_report.add_blocker(ground.blocker.name, ground.blocker.detail)
		return

	var stepper = selected.stepper
	var units := read_units(options.scene_dir, manifest)
	if not units.ok:
		_report.add_blocker(units.blocker.name, units.blocker.detail)
		return
	var world := compose(self, manifest, ground, decoded.samples, options.scene_dir, capture.side, _report, {"tables": units.tables, "tickHz": float(capture.tickHz)})
	if not _report.ok():
		return

	get_window().size = Vector2i(int(manifest.viewport.width), int(manifest.viewport.height))
	var camera := Camera3D.new()
	add_child(camera)
	camera.make_current()
	camera.far = CameraRig.far_plane(map.size, float(manifest.light.backdrop.reach_m))
	_report.add_budget(OWNER, "approximation: a finite far plane at %d m (the map's diagonal plus the backdrop reach); the browser's is infinite" % camera.far, "lens.far", 1, 1, 0)

	# The side's learned ground, replayed as the records pass its ticks: the
	# ground and the grass read its scars.
	var learned := LearnedGround.new(admitted.capture.layout.ground)
	var scars_shown: bool = manifest.layers.has("scars")
	var mismatch: String = world.scars.mismatch(learned) if scars_shown else ""
	if not mismatch.is_empty():
		_report.add_blocker("scars", mismatch)
		return
	var timing: FrameTiming = null
	var viewport_rid := get_viewport().get_viewport_rid()
	# The run draws each frame itself: Godot's main loop skips drawing, and
	# sleeps, while its window is minimised or covered. Each record still
	# passes one main-loop iteration, which pushes the moved nodes' transforms
	# to the renderer, with no sleep.
	RenderingServer.render_loop_enabled = false
	OS.low_processor_usage_mode_sleep_usec = 0
	RenderingServer.viewport_set_measure_render_time(viewport_rid, true)
	if options.bench >= 0:
		timing = FrameTiming.new(options.bench)
		_refuse_contention("start")
		DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_ENABLED if options.pace == "display" else DisplayServer.VSYNC_DISABLED)
		Engine.max_fps = 0
		# The headless browser presents nothing, so neither does a bench. The
		# project starts minimised, but macOS can bring the window back on
		# screen before this point; minimise it again before timing.
		DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_MINIMIZED)
	# Units are posed record by record: a record the stepper skips is posed
	# too, unseen, since poses carry from frame to frame.
	var posed := -1
	var record = stepper.step()
	while record != null:
		if world.units != null:
			for skipped in range(posed + 1, int(record.index)):
				world.units.advance(capture.frames[skipped])
			posed = int(record.index)
		var stepped := Time.get_ticks_usec()
		var sample := _sample_at(decoded.samples, int(record.tick))
		if sample.is_empty():
			_report.add_blocker("publication", "frame %d at tick %d precedes the first sample" % [record.index, record.tick])
			return
		CameraRig.apply(camera, record, manifest.lens)
		learned.advance_to(decoded.samples, int(record.tick))
		if scars_shown:
			world.scars.sync(learned)
		if world.grass != null:
			world.grass.grow(record)
		world.environment.prepare(camera, Vector2(get_window().size), StrictJson.vector3(record.target))
		var models: Dictionary = world.units.present(record, camera, Vector2(get_window().size)) if world.units != null else {}
		if world.effects != null:
			world.effects.present(record, camera, Vector2(get_window().size), world.units)
		world.static_world.present(sample, learned, float(record.simTime), camera, Vector2(get_window().size), _report)
		if world.fog != null:
			world.fog.show_fog(sample.fog)
		var step_ms := (Time.get_ticks_usec() - stepped) / 1000.0
		await get_tree().process_frame
		RenderingServer.force_draw()
		if timing != null:
			# CPU: this frame's step, Godot's frame setup and the viewport's
			# render submission, as the browser's is its frame callback.
			var cpu_ms := step_ms + RenderingServer.get_frame_setup_time_cpu() + RenderingServer.viewport_get_measured_render_time_cpu(viewport_rid)
			timing.drawn(Time.get_ticks_usec(), cpu_ms, _drawn_in_frame())
		else:
			var path: String = options.out.path_join("plate-%d.png" % record.index)
			var plate: Dictionary = PlateWriter.write(get_viewport(), record, path)
			if not plate.ok:
				_report.add_blocker(plate.blocker.name, plate.blocker.detail)
				return
			_report.add_plate(int(record.index), path, plate.gpu_ms)
			if world.static_world.listed():
				_count_instances(int(record.index), world.static_world, manifest, manifest.instanceCounts.filter(func(row): return row.plate == record.plate))
			if world.units != null:
				_count_models(int(record.index), models, manifest.modelCounts.filter(func(row): return row.plate == record.plate))
			if world.effects != null:
				_count_effects(int(record.index), world.effects.counts(record), manifest.effectCounts.filter(func(row): return row.plate == record.plate))
			if world.grass != null:
				var browser: Array = manifest.grassCounts.filter(func(row): return row.plate == record.plate)
				_report.add_grass(world.grass.counts(int(record.index), camera, Vector2(get_window().size), browser[0].cells if not browser.is_empty() else null))
		record = stepper.step()
	if timing != null:
		_refuse_contention("end")
		_report.bench = timing.to_dictionary()
		_report.bench.merge({"pace": options.pace, "vsync": DisplayServer.window_get_vsync_mode() != DisplayServer.VSYNC_DISABLED, "maxFps": Engine.max_fps, "conditions": _conditions()})
		if _report.bench.intervalsMs.is_empty():
			_report.add_blocker("bench", "no frame was drawn after the %d warm-up frames" % options.bench)
		return
	if world.grass != null:
		world.grass.report_counts(_report)
	if world.units != null:
		world.units.report_budgets(_report)
	if world.effects != null:
		world.effects.report_budgets(_report)

## The plate's building modules, scenery and known props against the
## browser's (`instanceCounts`): any count apart is an `instances` blocker.
func _count_instances(index: int, world: StaticWorld, manifest: Dictionary, browser: Array) -> void:
	var native := world.counts()
	_report.add_instances({"index": index, "native": native, "browser": null if browser.is_empty() else browser[0]})
	if browser.is_empty():
		_report.add_blocker("instances", "plate-%d: the manifest has no instance counts for it" % index)
		return
	for difference in StaticWorld.differences(native, browser[0], manifest.buildings.modules, manifest.scenery.kinds):
		_report.add_blocker("instances", "plate-%d: %s" % [index, difference])

## The plate's posed models by kind against the browser's (`modelCounts`):
## any kind apart is a `models` blocker.
func _count_models(index: int, native: Dictionary, browser: Array) -> void:
	var row := {"index": index, "native": native, "browser": null if browser.is_empty() else browser[0]}
	_report.add_models(row)
	if browser.is_empty():
		_report.add_blocker("models", "plate-%d: the manifest has no model counts for it" % index)
		return
	for kind in native:
		if int(browser[0][kind]) != int(native[kind]):
			_report.add_blocker("models", "plate-%d: native poses %d %s where the browser posed %d" % [index, native[kind], kind, browser[0][kind]])

## The plate's effects by kind, its instances by shape and its lights against
## the browser's (`effectCounts`): any count apart is an `effects` blocker.
func _count_effects(index: int, native: Dictionary, browser: Array) -> void:
	_report.add_effects({"index": index, "native": native, "browser": null if browser.is_empty() else browser[0]})
	if browser.is_empty():
		_report.add_blocker("effects", "plate-%d: the manifest has no effect counts for it" % index)
		return
	for group in ["running", "instances"]:
		for kind in native[group]:
			if int(browser[0][group][kind]) != int(native[group][kind]):
				_report.add_blocker("effects", "plate-%d: native draws %d %s (%s) where the browser drew %d" % [index, native[group][kind], kind, group, browser[0][group][kind]])
	if int(browser[0].lights) != int(native.lights):
		_report.add_blocker("effects", "plate-%d: native casts %d lights where the browser cast %d" % [index, native.lights, browser[0].lights])

## A bench measures the GPU alone: a browser drawing at the same time (the
## scene runner's Chromium, which can outlive its locked command while it tears
## down) is a `contention` blocker, at the timing's `when`.
func _refuse_contention(when: String) -> void:
	var output: Array = []
	# Anchored on the executable's path, so a command that merely mentions it
	# (a shell, a queued lockf) doesn't count.
	OS.execute("pgrep", ["-fl", "^/[^ ]*Chrome for Testing\\.app/"], output)
	var found := str(output[0]).strip_edges() if not output.is_empty() else ""
	if not found.is_empty():
		_report.add_blocker("contention", "a browser was running at the timing's %s: %s" % [when, found.get_slice("\n", 0)])

## What a timing depends on besides the code: the power source (`pmset`),
## the adapter, the display's refresh rate and the window's mode.
static func _conditions() -> Dictionary:
	var output: Array = []
	OS.execute("pmset", ["-g", "batt"], output)
	var power := "unknown"
	var drawing := RegEx.create_from_string("drawing from '([^']+)'").search(str(output[0]) if not output.is_empty() else "")
	if drawing != null:
		power = drawing.get_string(1)
	return {
		"power": power,
		"adapter": RenderingServer.get_video_adapter_name(),
		"refreshHz": DisplayServer.screen_get_refresh_rate(),
		"window": "minimized" if DisplayServer.window_get_mode() == DisplayServer.WINDOW_MODE_MINIMIZED else "on screen",
	}

## What the last frame drew, by Godot's count over every viewport and pass.
static func _drawn_in_frame() -> Dictionary:
	return {
		"drawCalls": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
		"objects": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_OBJECTS_IN_FRAME),
		"primitives": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME),
	}

## The ground's tables beside the manifest, its decoded height field and the
## tables of the grass growing on it:
## `{ok: true, tables, heights, grass}` or `{ok: false, blocker}`.
static func read_ground(scene_dir: String, manifest: Dictionary) -> Dictionary:
	var read := Manifest.read_tables(scene_dir, manifest.terrain, "terrain")
	if not read.ok:
		return read
	var grass := Manifest.read_tables(scene_dir, manifest.grass.tables, "grass")
	if not grass.ok:
		return grass
	if not read.tables.has("heights"):
		return {"ok": false, "blocker": {"name": "terrain", "detail": "the manifest has no terrain heights"}}
	var heights := HeightField.decode(read.tables.heights.words, read.tables.heights.layout)
	if not heights.ok:
		return {"ok": false, "blocker": {"name": "terrain", "detail": heights.detail}}
	return {"ok": true, "tables": read.tables, "heights": heights.heights, "grass": grass.tables}

## The units' clip tables beside the manifest: `{ok: true, tables}` or
## `{ok: false, blocker}`.
static func read_units(scene_dir: String, manifest: Dictionary) -> Dictionary:
	return Manifest.read_tables(scene_dir, manifest.units.tables, "units")

## Builds the light and every layer the manifest lists under `root`, staging the
## assets the layers draw, everything seated on `ground.heights`. Returns
## `{environment, static_world, grass, fog, scars, units, effects}`; `grass`,
## `fog`, `units` and `effects` are null unless listed, and `scars` (`scar_texture.gd`) holds no scars
## until synced. `units` holds the manifest units' clip tables
## (`read_units`) and the capture's tick rate. Blockers and budgets go to
## `report`.
static func compose(root: Node3D, manifest: Dictionary, ground: Dictionary, samples: Array, scene_dir: String, side: String, report, units := {}) -> Dictionary:
	var layers: Array = manifest.layers
	for layer in layers:
		if not LAYERS.has(layer):
			report.add_budget(OWNER, "the browser drew this layer; native does not draw it", "layer:" + layer, 1, 0, 1)
	var environment := EnvironmentAdapter.new(manifest.light, layers.has("sky"), _receivers(ground.heights))
	root.add_child(environment)
	var scars := ScarTexture.new(ground.tables.get("scars"))
	if layers.has("terrain"):
		var terrain := TerrainLayer.new(ground.tables, ground.heights, manifest.tuning.terrain, float(manifest.light.backdrop.reach_m), scars.texture, layers.has("water"))
		root.add_child(terrain)
		terrain.build(report)
	var grass: Node3D = null
	if layers.has("grass") and report.ok():
		grass = GrassLayer.new(ground, manifest.grass, manifest.tuning, manifest.lens, scars.texture)
		root.add_child(grass)
		grass.build(report)
	var static_world := StaticWorld.new(scene_dir, manifest, float(units.get("tickHz", 30.0)), report)
	root.add_child(static_world)
	var unit_layer: Node3D = null
	var assets := []
	if layers.has("models"):
		unit_layer = UnitLayer.new(manifest.units, manifest.effects, units.get("tables", {}), samples, side, float(units.get("tickHz", 30.0)), manifest.light)
		root.add_child(unit_layer)
		assets.append_array(unit_layer.required_assets())
	var staged := Staging.stage(assets, report)
	var fog: Node3D = null
	if report.ok():
		static_world.build(report)
		if unit_layer != null:
			unit_layer.build(staged, report)
		if layers.has("fog"):
			fog = FogLayer.new()
			root.add_child(fog)
	var effects: EffectLayer = null
	if layers.has("effects") and report.ok():
		effects = EffectLayer.new(manifest, samples, side, float(units.get("tickHz", 30.0)), environment.sky_lut(), unit_layer)
		environment.add_effect(effects.pass_)
	return {"environment": environment, "static_world": static_world, "grass": grass, "fog": fog, "scars": scars, "units": unit_layer, "effects": effects}

## The box shadows fall in, in Godot space: the ground grid, from a metre
## below its lowest ground to `Light.STANDING_M` above its highest, for what
## stands on it (the browser's `mapBox`).
static func _receivers(heights) -> AABB:
	var size := Vector2((heights.nx - 1) * heights.spacing, (heights.ny - 1) * heights.spacing)
	var low: float = heights.lowest() - 1.0
	var high: float = heights.highest() + Light.STANDING_M
	return AABB(Vector3(0.0, low, -size.y), Vector3(size.x, high - low, size.y))

## The latest decoded sample at or before `tick`, or empty.
static func _sample_at(samples: Array, tick: int) -> Dictionary:
	var chosen := {}
	for sample in samples:
		if sample.tick > tick:
			break
		chosen = sample
	return chosen
