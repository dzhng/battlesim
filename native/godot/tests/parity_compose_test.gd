extends "res://tests/check.gd"

const ParityRun = preload("res://parity_run.gd")
const Manifest = preload("res://capture/visual_manifest.gd")
const Capture = preload("res://capture/presentation_capture.gd")
const RenderReport = preload("res://report/render_report.gd")

var _admitted: Dictionary
var _samples: Array

func run() -> bool:
	_admitted = Manifest.load(FIXTURE_RUN.path_join(FIXTURE_SCENE), FIXTURE_MAPS)
	_samples = Capture.decode_samples(_admitted.capture).samples
	check(_samples.size() == 2 and _samples[0].units[0].position == [10.0, 20.0, 0.0], "the fixture publications decode to their unit")

	# The fixture manifest lists sky, terrain, buildings, props and models: no grass and no fog.
	var listed := _compose(_admitted.manifest.layers)
	check(listed.root.find_child("Terrain", true, false) != null, "a listed layer is drawn")
	check(listed.root.find_child("Buildings", true, false) != null, "a listed buildings layer is drawn")
	check(listed.root.find_child("Grass", true, false) == null, "an unlisted grass layer is not drawn")
	check(listed.root.find_child("Fog", true, false) == null, "fog is decoded but not painted when the manifest does not list it")
	check(listed.report.ok(), "the fixture composes without a blocker: %s" % JSON.stringify(listed.report.to_dictionary().blockers))
	var budgets: Array = listed.report.to_dictionary().budgets
	check(budgets.any(func(row): return row.sourceId == "map.buildings/boxes" and row.reason.begins_with("approximation")), "a building without a kit shell is reported as an approximation")
	check(budgets.any(func(row): return row.sourceId == "unit:fixture_rifle" and row.admitted == 1), "each unit kind is reported with its unit count")
	listed.root.free()

	var without_sky := _compose(["terrain"])
	var environment: WorldEnvironment = without_sky.root.find_child("Environment", true, false).get_child(0)
	check(environment.environment.background_mode != Environment.BG_SKY, "an unlisted sky is not drawn")
	check(without_sky.root.find_child("Buildings", true, false) == null, "an unlisted buildings layer is not drawn")
	check(without_sky.root.find_child("ObservedUnits", true, false) == null, "an unlisted models layer draws no units")
	without_sky.root.free()

	var fogged := _compose(["fog", "scars"])
	var fog: MultiMeshInstance3D = fogged.root.find_child("Fog", true, false)
	check(fog != null, "a listed fog layer is drawn")
	if fog != null:
		fog.show_fog(_samples[0].fog)
		check(fog.multimesh.instance_count == 2, "fog paints exactly the hidden cells (2 of 4)")
	check(fogged.report.to_dictionary().budgets.any(func(row): return row.sourceId == "layer:scars" and row.dropped == 1), "a listed layer native cannot draw is reported as dropped")
	fogged.root.free()

	# Units keep their node by id, however the publication orders its rows.
	var units := _compose(["models"])
	var layer = units.world.layer
	var a := {"id": 1.0, "kindName": "fixture_rifle", "position": [0.0, 0.0, 0.0], "yaw": 0.0}
	var b := {"id": 2.0, "kindName": "fixture_rifle", "position": [5.0, 5.0, 0.0], "yaw": 0.0}
	var holder: Node = units.root.find_child("ObservedUnits", true, false)
	layer.show_units([a, b])
	var nodes: Array = holder.get_children()
	layer.show_units([b, a])
	check(holder.get_children() == nodes and nodes[0].position == Vector3.ZERO and nodes[1].position == Vector3(5, 0, 5), "a unit keeps its node when the rows reorder")
	layer.show_units([b])
	check(not nodes[0].visible and nodes[1].visible, "a unit absent from the publication is hidden")
	units.root.free()
	return true

func _compose(layers: Array) -> Dictionary:
	var root := Node3D.new()
	var report := RenderReport.new()
	var map = JSON.parse_string(FileAccess.get_file_as_string(_admitted.map_path))
	var world := ParityRun.compose(root, layers, _samples, map, [Vector2(24, 24)], "blue", report)
	return {"root": root, "report": report, "world": world}
