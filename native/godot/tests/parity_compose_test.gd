extends "res://tests/check.gd"

const ParityRun = preload("res://parity_run.gd")
const RenderReport = preload("res://report/render_report.gd")

const SAMPLES := []

func run() -> bool:
	var listed := _compose(["sky", "terrain"])
	check(listed.root.find_child("Terrain", true, false) != null, "a listed layer is drawn")
	check(listed.root.find_child("Grass", true, false) == null, "an unlisted grass layer is not drawn")
	check(listed.root.find_child("Fog", true, false) == null, "fog is decoded but not painted when the manifest does not list it")
	check(listed.report.ok(), "the layers compose without a blocker: %s" % JSON.stringify(listed.report.to_dictionary().blockers))
	check(listed.report.to_dictionary().budgets.any(func(row): return row.sourceId == "terrain.heights" and row.rendered > 0), "the ground is drawn from the run's terrain tables")
	listed.root.free()

	# A listed buildings layer resolves the capture's map from the
	# repository: a map that is not there stops the run by name.
	var built := _compose(["buildings"], "no-such-map")
	check(built.report.to_dictionary().blockers.any(func(row): return row.name == "buildings" and row.detail.contains("no-such-map")), "a listed buildings layer whose map does not resolve is a named blocker")
	built.root.free()

	var without_sky := _compose(["terrain"])
	var environment: WorldEnvironment = without_sky.root.find_child("Environment", true, false).get_child(0)
	check(environment.environment.background_mode != Environment.BG_SKY, "an unlisted sky is not drawn")
	check(without_sky.root.find_child("Buildings", true, false) == null, "an unlisted buildings layer is not drawn")
	check(without_sky.root.find_child("Units", true, false) == null, "an unlisted models layer draws no units")
	var sun: DirectionalLight3D = without_sky.root.find_child("Sun", true, false)
	# The manifest's sun is toward +x and up in sim space: light travels down and west.
	check((-sun.basis.z).is_equal_approx(Vector3(-1, -1, 0).normalized()), "the sun shines away from the manifest's sun direction: %s" % -sun.basis.z)
	check(environment.environment.ambient_light_source == Environment.AMBIENT_SOURCE_SKY, "the sky lights the scene even when it is not shown")
	without_sky.root.free()

	# The grass is grown on the GPU: without a display, a listed grass layer
	# stops the run by name rather than being left out.
	var grass := _compose(["terrain", "grass"])
	check(grass.report.to_dictionary().blockers.any(func(row): return row.name == "grass" and row.detail.contains("rendering device")), "a listed grass layer that cannot grow is a named blocker")
	grass.root.free()

	var fogged := _compose(["fog", "props"])
	var fog: MultiMeshInstance3D = fogged.root.find_child("Fog", true, false)
	check(fog != null, "a listed fog layer is drawn")
	if fog != null:
		fog.show_fog({"nx": 2, "ny": 2, "cellM": 8.0, "bits": [9]})
		check(fog.multimesh.instance_count == 2, "fog paints exactly the hidden cells (2 of 4)")
	check(fogged.report.to_dictionary().budgets.any(func(row): return row.sourceId == "layer:props" and row.dropped == 1), "a listed layer native cannot draw is reported as dropped")
	fogged.root.free()

	# The effects draw into the lit, hazed frame, before the post chain.
	var effected := _compose(["effects"])
	var world_environment: WorldEnvironment = effected.root.find_child("Environment", true, false).get_child(0)
	var passes: Array = world_environment.compositor.compositor_effects
	check(effected.world.effects != null and passes.size() == 3 and passes[1] == effected.world.effects.pass_, "a listed effects layer draws between the haze and the post chain")
	effected.root.free()
	var plain := _compose(["terrain"])
	check(plain.world.effects == null, "an unlisted effects layer draws nothing")
	plain.root.free()

	# Soldiers are posed on the GPU: without a display, a listed models layer
	# stops the run by name rather than being left out.
	var units := _compose(["models"])
	check(units.root.find_child("Units", true, false) != null, "a listed models layer is built")
	check(units.report.to_dictionary().blockers.any(func(row): return row.name == "units" and row.detail.contains("rendering device")), "a listed models layer that cannot pose is a named blocker")
	units.root.free()
	return true

## The fixture run's light (its sun turned), ground and world sections, its
## buildings those of saved map `map`.
func _compose(layers: Array, map := FIXTURE_SCENE) -> Dictionary:
	var root := Node3D.new()
	var report := RenderReport.new()
	var fixture: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("manifest.json"))
	var light: Dictionary = fixture.light
	light.sunDirection = [sqrt(0.5), 0.0, sqrt(0.5)]
	var identity: Dictionary = fixture.identity.merged({"map": map}, true)
	var manifest := {"identity": identity, "layers": layers, "light": light, "tuning": fixture.tuning, "grass": fixture.grass, "lens": fixture.lens, "units": fixture.units, "scenery": fixture.scenery, "props": fixture.props, "effects": fixture.effects}
	var ground := ParityRun.read_ground(FIXTURE_RUN.path_join(FIXTURE_SCENE), fixture)
	var world := ParityRun.compose(root, manifest, ground, SAMPLES, FIXTURE_RUN.path_join(FIXTURE_SCENE), "blue", report)
	return {"root": root, "report": report, "world": world}
