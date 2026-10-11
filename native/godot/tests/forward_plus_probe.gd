extends "res://tests/check.gd"

## Display-backed backend probe: one real kit module drawn by the building
## layer, the grass build's compute-written MultiMeshes, a shadowed sun and the
## sky, composed by the parity run and read back as a plate. Run it with a display (it is not a headless
## test):
##   godot --path native/godot --window-position 10000,10000 --script res://tests/forward_plus_probe.gd
## It needs the homes kit from Git LFS.

const ParityRun = preload("res://parity_run.gd")
const PlateWriter = preload("res://reel/plate_writer.gd")
const RenderReport = preload("res://report/render_report.gd")
const Staging = preload("res://assets/staging.gd")
const CameraRig = preload("res://reel/camera_rig.gd")
const ErrorLog = preload("res://report/error_log.gd")
const BuildingLayer = preload("res://world/building_layer.gd")
const AppearanceCatalog = preload("res://world/appearance_catalog.gd")
const DetailView = preload("res://world/detail_view.gd")
const Records = preload("res://tests/records.gd")
const BuildingLayerTest = preload("res://tests/building_layer_test.gd")

## A one-storey home's shell, whole at every tier.
const MODULE := {"kit": "city_kit_homes", "module": "home_10x8_1f_shell"}
const TIERS := {"lodPxPerM": [10.0, 4.0, 1.2], "chunkM": 64.0, "poolRecords": 1024, "expandRows": 40000, "casterCoarser": 1}
const OUT := "throwaway/native-runner/forward-plus-probe"

func run() -> bool:
	# As in the parity run, an error Godot logs fails the probe: a script
	# error returns null and lets the probe go on.
	var errors := ErrorLog.new()
	OS.add_logger(errors)
	check(DisplayServer.get_name() != "headless", "the probe needs a display")
	check(ProjectSettings.get_setting("rendering/renderer/rendering_method") == "forward_plus", "the project renders with Forward+")
	var at := Vector2(200.0, 200.0)

	var report := RenderReport.new()
	var world := Node3D.new()
	root.add_child(world)
	# The fixture run's light and ground stand in for market-town's: the probe
	# proves the backend, not the scene.
	var fixture: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("manifest.json"))
	var ground := ParityRun.read_ground(FIXTURE_RUN.path_join(FIXTURE_SCENE), fixture)
	var manifest := {"layers": ["sky", "terrain", "grass"], "light": fixture.light, "tuning": fixture.tuning, "grass": fixture.grass, "lens": fixture.lens}
	var composed := ParityRun.compose(world, manifest, ground, [], "", "blue", report)
	var building := BuildingLayer.new({
		"buildings": Records.table(BuildingLayerTest.BUILDING_FIELDS, [{"frame": [at.x, at.y, 0.0, 0.3], "min": [at.x - 6, at.y - 6, 0.0], "max": [at.x + 6, at.y + 6, 6.0], "owner": 0, "chunk": 0, "rows": [1, -1, -1]}]),
		"intact": Records.table(BuildingLayerTest.MODULE_FIELDS, [{"position": [at.x, at.y, 0.0], "yaw": 0.3, "scale": [1.0, 1.0, 1.0], "kitModule": 0, "tint": [1.0, 1.0, 1.0], "tiers": 15, "building": 0, "chunk": 0}]),
		"ruin": Records.table(BuildingLayerTest.MODULE_FIELDS, []),
		"gutted": Records.table(BuildingLayerTest.MODULE_FIELDS, []),
	}, {"tiers": TIERS, "modules": [MODULE]}, Vector3(sqrt(0.5), sqrt(0.5), 0.0), float(fixture.light.cascades.max_far_m))
	world.add_child(building)
	var catalog := AppearanceCatalog.new()
	catalog.stage(building.required_assets(catalog), report)
	building.build(catalog, report)
	catalog.free_scenes()
	var camera := Camera3D.new()
	world.add_child(camera)
	var record := {"index": 0, "simTime": 0.0, "canvas": [1280, 720], "eye": [at.x - 22.0, at.y - 22.0, 9.0], "target": [at.x, at.y, 2.0], "up": [0.0, 0.0, 1.0]}
	CameraRig.apply(camera, record, {"fovY": 0.8, "near": 1.0})
	camera.make_current()
	if composed.grass != null:
		composed.grass.grow(record)
	building.prepare(DetailView.of(camera, Vector2(1280, 720), Vector3(sqrt(0.5), sqrt(0.5), 0.0), float(fixture.light.cascades.max_far_m)), report)

	var plate: Dictionary = {"ok": false}
	if report.ok():
		plate = await PlateWriter.write(root, record, Staging.repo_path(OUT).path_join("plate-0.png"))
		if plate.ok:
			report.add_plate(0, Staging.repo_path(OUT).path_join("plate-0.png"), plate.gpu_ms)
		else:
			report.add_blocker(plate.blocker.name, plate.blocker.detail)
	report.write(Staging.repo_path(OUT).path_join("report.json"))

	var summary := report.to_dictionary()
	check(report.ok(), "the probe renders without a blocker: %s" % JSON.stringify(summary.blockers))
	check(Array(building.counts).reduce(func(sum, n): return sum + n, 0) == 1, "the kit module is drawn: %s" % building.counts)
	check(composed.grass != null and composed.grass.counts(0, camera, Vector2(1280, 720), null).cells.size() == 12, "the grass build runs and its clumps read back")
	check(world.find_child("Sun", true, false).shadow_enabled, "the sun casts shadows")
	check(summary.renderer.driver == "metal", "the probe renders on Metal")
	world.free()
	OS.remove_logger(errors)
	check(errors.errors().is_empty(), "the probe logs no error: %s" % [errors.errors()])
	return true
