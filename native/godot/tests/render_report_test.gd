extends "res://tests/check.gd"

const RenderReport = preload("res://report/render_report.gd")

func run() -> bool:
	var report := RenderReport.new()
	report.identity = {"scene": "test-town", "map": "test-town", "encounter": "menu", "seed": 1}
	report.frames = "plates"
	report.add_budget("native/legacy_map_layer", "buildings beyond 1000 m of every frame eye", "map:test-town/buildings", 3, 2, 1)
	report.add_blocker("asset", "assets/source/grass/grass_meadow.glb is a Git LFS pointer")
	report.add_plate(2, "/tmp/plate-2.png", 1.25)
	report.add_plate(3, "/tmp/plate-3.png", null)
	check(not report.ok(), "a report with a blocker is not ok")

	var path := scratch("render-report").path_join("report.json")
	check(report.write(path) == OK, "the report writes")
	var read: Dictionary = read_json(path)
	# JSON has one number type, so compare in its value domain.
	check(read == JSON.parse_string(JSON.stringify(report.to_dictionary())), "the written report reads back unchanged: %s" % JSON.stringify(read))
	check(read.schema == "godot-render-report", "the report names its schema without a version")
	check(read.budgets[0] == {"owner": "native/legacy_map_layer", "reason": "buildings beyond 1000 m of every frame eye", "sourceId": "map:test-town/buildings", "admitted": 3.0, "rendered": 2.0, "dropped": 1.0}, "a budget row keeps every field")
	check(read.plates[0] == {"index": 2.0, "path": "/tmp/plate-2.png", "gpuMs": 1.25}, "a plate row keeps its GPU time")
	check(read.plates[1].gpuMs == null, "a plate without a GPU measurement says so rather than 0")
	check(read.renderer.build == "editor", "a run from the editor binary says so, apart from a debug or release export: %s" % read.renderer)
	check(RenderReport.new().ok(), "a report without blockers is ok")
	return true
