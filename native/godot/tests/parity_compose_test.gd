extends "res://tests/check.gd"

const ParityRun = preload("res://parity_run.gd")
const RenderReport = preload("res://report/render_report.gd")

## A map whose rows need no assets: a template with no kit shell and a prop
## kind with no catalog model.
const MAP := {
	"size": [64.0, 64.0],
	"regional_family": "test",
	"surfaces": [{"kind": "paving", "shape": {"kind": "polygon", "ring": [[20, 20], [44, 20], [44, 44], [20, 44]]}}],
	"forests": [],
	# The parked car has a catalog model, but lies beyond reach, so nothing stages it.
	"props": [{"kind": "test_post", "center": [18.0, 24.0], "yaw": 0.0, "half_extents": [0.3, 0.3, 1.0], "base_z": null}, {"kind": "parked_car", "center": [5000.0, 5000.0], "yaw": 0.0, "half_extents": [2.0, 1.0, 0.7], "base_z": null}],
	"buildings": [{"template_id": "test-block", "frame": {"translation": [32.0, 32.0, 0.0], "yaw": 0.3}}],
}
const SAMPLES := [{"tick": 3, "units": [{"id": 7.0, "kindName": "test_unit", "position": [10.0, 20.0, 0.0], "yaw": 0.5}], "fog": {}}]

func run() -> bool:
	var listed := _compose(["sky", "terrain", "buildings", "props", "models"])
	check(listed.root.find_child("Terrain", true, false) != null, "a listed layer is drawn")
	check(listed.root.find_child("Buildings", true, false) != null, "a listed buildings layer is drawn")
	check(listed.root.find_child("Grass", true, false) == null, "an unlisted grass layer is not drawn")
	check(listed.root.find_child("Fog", true, false) == null, "fog is decoded but not painted when the manifest does not list it")
	check(listed.report.ok(), "the map composes without a blocker: %s" % JSON.stringify(listed.report.to_dictionary().blockers))
	var budgets: Array = listed.report.to_dictionary().budgets
	check(budgets.any(func(row): return row.sourceId == "map.buildings/boxes" and row.reason.begins_with("approximation")), "a building without a kit shell is reported as an approximation")
	check(budgets.any(func(row): return row.sourceId == "unit:test_unit" and row.admitted == 1), "each unit kind is reported with its unit count")
	check(budgets.any(func(row): return row.sourceId == "prop:parked_car" and row.rendered == 0 and row.dropped == 1), "a prop beyond reach is reported dropped, and its model is never loaded")
	listed.root.free()

	var without_sky := _compose(["terrain"])
	var environment: WorldEnvironment = without_sky.root.find_child("Environment", true, false).get_child(0)
	check(environment.environment.background_mode != Environment.BG_SKY, "an unlisted sky is not drawn")
	check(without_sky.root.find_child("Buildings", true, false) == null, "an unlisted buildings layer is not drawn")
	check(without_sky.root.find_child("ObservedUnits", true, false) == null, "an unlisted models layer draws no units")
	var sun: DirectionalLight3D = without_sky.root.find_child("Sun", true, false)
	# The manifest's sun is toward +x and up in sim space: light travels down and west.
	check((-sun.basis.z).is_equal_approx(Vector3(-1, -1, 0).normalized()), "the sun shines away from the manifest's sun direction: %s" % -sun.basis.z)
	check(environment.environment.ambient_light_source == Environment.AMBIENT_SOURCE_SKY, "the sky lights the scene even when it is not shown")
	without_sky.root.free()

	var bare := _compose(["grass"])
	check(bare.report.to_dictionary().budgets.any(func(row): return row.sourceId == "grass" and row.admitted == 0), "a listed grass layer with no tufts near a frame target draws nothing and says so")
	bare.root.free()

	var fogged := _compose(["fog", "scars"])
	var fog: MultiMeshInstance3D = fogged.root.find_child("Fog", true, false)
	check(fog != null, "a listed fog layer is drawn")
	if fog != null:
		fog.show_fog({"nx": 2, "ny": 2, "cellM": 8.0, "bits": [9]})
		check(fog.multimesh.instance_count == 2, "fog paints exactly the hidden cells (2 of 4)")
	check(fogged.report.to_dictionary().budgets.any(func(row): return row.sourceId == "layer:scars" and row.dropped == 1), "a listed layer native cannot draw is reported as dropped")
	fogged.root.free()

	# Units keep their node by id, however the publication orders its rows.
	var units := _compose(["models"])
	var layer = units.world.layer
	var a := {"id": 1.0, "kindName": "test_unit", "position": [0.0, 0.0, 0.0], "yaw": 0.0}
	var b := {"id": 2.0, "kindName": "test_unit", "position": [5.0, 5.0, 0.0], "yaw": 0.0}
	var holder: Node = units.root.find_child("ObservedUnits", true, false)
	layer.show_units([a, b])
	var nodes: Array = holder.get_children()
	layer.show_units([b, a])
	check(holder.get_children() == nodes and nodes[0].position == Vector3.ZERO and nodes[1].position == Vector3(5, 0, -5), "a unit keeps its node when the rows reorder")
	layer.show_units([b])
	check(not nodes[0].visible and nodes[1].visible, "a unit absent from the publication is hidden")
	units.root.free()
	return true

func _compose(layers: Array) -> Dictionary:
	var root := Node3D.new()
	var report := RenderReport.new()
	var light: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("manifest.json")).light
	light.sunDirection = [1.0, 0.0, 1.0]
	var manifest := {"layers": layers, "light": light}
	var world := ParityRun.compose(root, manifest, SAMPLES, MAP, [Vector2(24, 24)], "blue", report)
	return {"root": root, "report": report, "world": world}
