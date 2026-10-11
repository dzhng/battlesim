extends "res://tests/check.gd"

## Native poses the fixture capture's units as the browser does: every frame
## record of paris-corner up to its first plate driven through the ported
## interpolation, feed and pose driver (`units/`), compared with the browser's
## own poses of the same records (gait clip and phase, facing, position,
## crossfade, turret bearing and travel, corpses) and model counts; the
## tank's articulation at fixed inputs; and a body's joint worlds sampled from
## the clip table at fixed clips. The vectors are written by
## `web/tests/nativePoseVectors.test.ts`.

const Capture = preload("res://capture/presentation_capture.gd")
const UnitManifest = preload("res://capture/unit_manifest.gd")
const UnitPoses = preload("res://units/unit_poses.gd")
const Articulation = preload("res://units/articulation.gd")

const POSES := "res://tests/fixtures/poses"
## Poses agree to float32 precision on the decoded positions they start from.
const TOLERANCE := 1e-4

func run() -> bool:
	var expected: Dictionary = read_json(POSES.path_join("paris-corner-poses.json"))
	var units: Dictionary = read_json(POSES.path_join("paris-corner-units.json"))
	check(UnitManifest.error(units).is_empty(), "the units section is accepted: %s" % UnitManifest.error(units))
	var run: Dictionary = read_json(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("manifest.json"))
	check(_same_json(run.get("units"), units), "the fixture run's manifest carries the units these poses were made with")
	var parsed := Capture.parse(FileAccess.get_file_as_string("res://tests/fixtures".path_join(expected.capture)))
	var decoded := Capture.decode_samples(parsed)
	check(decoded.ok, "the fixture capture decodes")
	if not decoded.ok:
		return true
	var capture: Dictionary = parsed.capture
	var poses := UnitPoses.new(units, run.effects, decoded.samples, capture.side, float(capture.tickHz))
	var frames := {}
	for frame in expected.frames:
		frames[int(frame.index)] = frame
	var compared := 0
	for record in capture.frames:
		var frame := poses.step(record)
		var want = frames.get(int(record.index))
		if want == null:
			continue
		compared += 1
		var at := "frame %d" % record.index
		check(poses.counts(frame) == _ints(want.counts), "%s: counts %s, not %s" % [at, poses.counts(frame), want.counts])
		check(frame.soldiers.size() == want.soldiers.size(), "%s: %d soldiers, not %d" % [at, frame.soldiers.size(), want.soldiers.size()])
		for i in mini(frame.soldiers.size(), want.soldiers.size()):
			var s: Dictionary = frame.soldiers[i]
			var w: Dictionary = want.soldiers[i]
			var who := "%s soldier %d" % [at, w.soldier]
			check(int(s.soldier) == int(w.soldier) and s.clip == w.clip, "%s plays %s, not %s" % [who, s.clip, w.clip])
			_near(s.phase, w.phase, who + " phase")
			_near(s.facing, w.facing, who + " facing")
			_near_all(s.position, w.position, who + " position")
			check((s.blend == null) == (w.blend == null), "%s blends as the browser" % who)
			if s.blend != null and w.blend != null:
				check(s.blend.clip == w.blend.clip, "%s fades from %s" % [who, w.blend.clip])
				_near(s.blend.weight, w.blend.weight, who + " blend weight")
			check(_same(s.operatorMount, w.operatorMount) and _same(s.activeMount, w.activeMount), "%s carries as the browser" % who)
		check(frame.vehicles.size() == want.vehicles.size(), "%s: vehicles" % at)
		for i in mini(frame.vehicles.size(), want.vehicles.size()):
			var v: Dictionary = frame.vehicles[i]
			var w: Dictionary = want.vehicles[i]
			_near_all(v.position, w.position, "%s vehicle position" % at)
			for key in w.articulation:
				_near(v.articulation[key], w.articulation[key], "%s vehicle %s" % [at, key])
		check(frame.corpses.size() == want.corpses.size(), "%s: corpses" % at)
		for i in mini(frame.corpses.size(), want.corpses.size()):
			check(int(frame.corpses[i].soldier) == int(want.corpses[i].soldier) and int(frame.corpses[i].lies) == int(want.corpses[i].lies), "%s corpse %d lies as the browser's" % [at, i])
	check(compared == expected.frames.size(), "every written frame was compared (%d of %d)" % [compared, expected.frames.size()])

	var vehicle: Dictionary = units.appearances[expected.vehicle]
	for case in expected.articulations:
		var locals := Articulation.articulate(vehicle.nodes, vehicle.rig, case.input)
		for n in locals.size():
			var w: Dictionary = case.locals[n]
			var l: Dictionary = locals[n]
			var q: Quaternion = l.r
			_near_all([l.t.x, l.t.y, l.t.z, q.x, q.y, q.z, q.w], w.t + w.r, "articulation %s node %s" % [case.input, vehicle.nodes[n].name])

	var body: Dictionary = units.appearances[expected.body]
	var table := FileAccess.get_file_as_bytes(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join("units-%s.u32" % body.table)).to_float32_array()
	for case in expected.joints:
		var worlds := Articulation.pose_worlds(body, table, case.clip, float(case.phase), null)
		for j in worlds.size():
			var o := worlds[j].origin
			_near_all([o.x, o.y, o.z], case.worlds[j], "%s at %s joint %s" % [case.clip, case.phase, body.joints[j].name])
	return true

func _near(actual: float, wanted: float, what: String) -> void:
	check(absf(actual - wanted) <= TOLERANCE * maxf(1.0, absf(wanted)), "%s is %s, not %s" % [what, actual, wanted])

func _near_all(actual: Array, wanted: Array, what: String) -> void:
	check(actual.size() == wanted.size(), what + " length")
	for i in mini(actual.size(), wanted.size()):
		if absf(float(actual[i]) - float(wanted[i])) > TOLERANCE * maxf(1.0, absf(float(wanted[i]))):
			check(false, "%s is %s, not %s" % [what, actual, wanted])
			return

## Whether two parsed JSON values are the same, numbers to a part in 10¹²:
## Godot's parser reads one number's different spellings a few ulps apart.
static func _same_json(a: Variant, b: Variant) -> bool:
	if typeof(a) in [TYPE_INT, TYPE_FLOAT] and typeof(b) in [TYPE_INT, TYPE_FLOAT]:
		return absf(float(a) - float(b)) <= 1e-12 * maxf(1.0, absf(float(a)))
	if typeof(a) != typeof(b):
		return false
	if a is Array:
		return a.size() == b.size() and range(a.size()).all(func(i): return _same_json(a[i], b[i]))
	if a is Dictionary:
		return a.size() == b.size() and a.keys().all(func(k): return b.has(k) and _same_json(a[k], b[k]))
	return a == b

static func _same(a: Variant, b: Variant) -> bool:
	return (a == null and b == null) or (a != null and b != null and int(a) == int(b))

static func _ints(counts: Dictionary) -> Dictionary:
	var out := {}
	for key in ["soldiers", "vehicles", "aircraft", "corpses", "fading"]:
		out[key] = int(counts[key])
	return out
