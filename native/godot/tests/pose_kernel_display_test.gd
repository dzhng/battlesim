extends "res://tests/check.gd"

## Display-backed: the GPU pose kernel (`units/pose_kernel.gd`) composes a
## body's joints as the CPU port does (`units/articulation.gd`, itself
## checked against the browser's vectors by `pose_vectors_test.gd`): a clip
## at a phase, and a crossfade of two, on the fixture run's clip table. Then
## the units layer builds every appearance of the fixture run with no
## blocker: each glTF's joints and nodes stand where the bundle's do.

const ParityRun = preload("res://parity_run.gd")
const PoseKernel = preload("res://units/pose_kernel.gd")
const UnitLayer = preload("res://units/unit_layer.gd")
const Articulation = preload("res://units/articulation.gd")
const Coords = preload("res://core/coords.gd")
const Staging = preload("res://assets/staging.gd")
const RenderReport = preload("res://report/render_report.gd")

const BODY := "rifle_squad_active_a"
const TOLERANCE := 1e-4

func run() -> bool:
	check(DisplayServer.get_name() != "headless", "the test needs a display")
	RenderingServer.render_loop_enabled = false
	var dir := FIXTURE_RUN.path_join(FIXTURE_SCENE)
	var fixture: Dictionary = read_json(dir.path_join("manifest.json"))
	var units := ParityRun.read_units(dir, fixture)
	var record: Dictionary = fixture.units.appearances[BODY]
	var table: PackedFloat32Array = units.tables[record.table].words.to_float32_array()
	var joints: int = record.joints.size()
	# Each joint its own bind, the identity: the palette is the joints' worlds.
	var parents := PackedInt32Array()
	var binds := PackedFloat32Array()
	for j in joints:
		parents.append(int(record.joints[j].parent))
		PoseKernel.bind(binds, Transform3D.IDENTITY, j)
	var kernel := PoseKernel.new(RenderingServer.get_rendering_device(), 4 * joints)
	check(kernel.load_tables(table, parents, binds), "the kernel compiles")
	var cases := [
		{"clip": "walk", "phase": 0.37, "blend": null},
		{"clip": "run", "phase": 0.9, "blend": {"clip": "walk_aim_f", "phase": 0.2, "weight": 0.35}},
	]
	for case in cases:
		var a := PoseKernel.clip(int(record.clips[case.clip].offset), Articulation.clip_frames(record.clips[case.clip], case.phase))
		var b := a
		var weight := 0.0
		if case.blend != null:
			b = PoseKernel.clip(int(record.clips[case.blend.clip].offset), Articulation.clip_frames(record.clips[case.blend.clip], case.blend.phase))
			weight = case.blend.weight
		check(kernel.queue(a, b, weight, joints, 0, joints, 0) >= 0, "a pose fits the palette")
	kernel.dispatch()
	RenderingServer.force_draw()
	var palette := kernel.read(joints * cases.size())
	for c in cases.size():
		var case: Dictionary = cases[c]
		var worlds := Articulation.pose_worlds(record, table, case.clip, case.phase, case.blend)
		var worst := 0.0
		for j in joints:
			var m := Transform3D(Coords.TO_GODOT, Vector3.ZERO) * worlds[j]
			for row in 3:
				var o := ((c * joints + j) * 3 + row) * 4
				var want := [m.basis.x[row], m.basis.y[row], m.basis.z[row], m.origin[row]]
				for k in 4:
					worst = maxf(worst, absf(palette[o + k] - want[k]))
		check(worst <= TOLERANCE, "%s: the GPU's joints are the CPU port's (worst %s)" % [case.clip, worst])
	kernel.free_device()

	var report := RenderReport.new()
	var layer := UnitLayer.new(fixture.units, fixture.effects, units.tables, [], "blue", 30.0, fixture.light)
	get_root().add_child(layer)
	layer.build(Staging.stage(layer.required_assets(), report), report)
	check(report.ok(), "every fixture appearance builds: %s" % JSON.stringify(report.to_dictionary().blockers))
	layer.free()
	return true
