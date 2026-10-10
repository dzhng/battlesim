extends "res://tests/check.gd"

## Display-backed backend probe: one real market-town building from its staged
## kit, the grass build's compute-written MultiMeshes, a shadowed sun and the sky, composed by the parity
## run and read back as a plate. Run it with a display (it is not a headless
## test):
##   godot --path native/godot --window-position 10000,10000 --script res://tests/forward_plus_probe.gd
## It needs the homes kit from Git LFS.

const ParityRun = preload("res://parity_run.gd")
const PlateWriter = preload("res://reel/plate_writer.gd")
const RenderReport = preload("res://report/render_report.gd")
const Staging = preload("res://assets/staging.gd")
const CameraRig = preload("res://reel/camera_rig.gd")

const TEMPLATE := "china-home-10x8-1f"
const OUT := "throwaway/native-runner/forward-plus-probe"

func run() -> bool:
	check(DisplayServer.get_name() != "headless", "the probe needs a display")
	check(ProjectSettings.get_setting("rendering/renderer/rendering_method") == "forward_plus", "the project renders with Forward+")
	var map: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(Staging.repo_path("fixtures/maps/market-town/map.json")))
	var building: Dictionary = map.buildings.filter(func(row): return row.template_id == TEMPLATE)[0]
	var at := Vector2(float(building.frame.translation[0]), float(building.frame.translation[1]))
	var one_building := {"size": map.size, "regional_family": map.regional_family, "surfaces": [], "forests": [], "props": [], "buildings": [building]}

	var report := RenderReport.new()
	var world := Node3D.new()
	root.add_child(world)
	# The fixture run's light and ground stand in for market-town's: the probe
	# proves the backend, not the scene.
	var fixture: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("manifest.json"))
	var ground := ParityRun.read_ground(FIXTURE_RUN.path_join(FIXTURE_SCENE), fixture)
	var manifest := {"layers": ["sky", "terrain", "grass", "buildings"], "light": fixture.light, "tuning": fixture.tuning, "grass": fixture.grass, "lens": fixture.lens}
	var composed := ParityRun.compose(world, manifest, ground, [], one_building, [at], "blue", false, report)
	var camera := Camera3D.new()
	world.add_child(camera)
	var record := {"index": 0, "canvas": [1280, 720], "eye": [at.x - 22.0, at.y - 22.0, 9.0], "target": [at.x, at.y, 2.0], "up": [0.0, 0.0, 1.0]}
	CameraRig.apply(camera, record, {"fovY": 0.8, "near": 1.0})
	camera.make_current()
	if composed.grass != null:
		composed.grass.grow(record)

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
	check(summary.budgets.any(func(row): return row.sourceId == "map.buildings" and row.rendered == 1), "the building is drawn")
	check(not summary.budgets.any(func(row): return row.sourceId == "map.buildings/boxes"), "the building is the kit's shell, not a box")
	check(composed.grass != null and composed.grass.counts(camera, Vector2(1280, 720)).cells.size() == 12, "the grass build runs and its clumps read back")
	check(world.find_child("Sun", true, false).shadow_enabled, "the sun casts shadows")
	check(summary.renderer.driver == "metal", "the probe renders on Metal")
	world.free()
	return true
