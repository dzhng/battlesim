extends RefCounted

## How a posed model's parts stand, ported from the browser's owners:
## `packages/scene-assets/src/articulation.ts` (`articulate`, `trackScroll`)
## for a vehicle's rig, and `packages/battle-renderer/src/models/clipTable.ts`
## with the pose kernel (`poseKernel.ts`) for a soldier's clips. Transforms
## are sim space, z up (the bundle's own space), as the manifest's records
## (`units.appearances`) give them: a local is `{t: Vector3, r: Quaternion,
## s: Vector3}`.

## Floats per joint sample in a clip table (`SAMPLE_FLOATS`).
const SAMPLE_FLOATS := 8
## The kernel's slerp turns linear past this cosine (`poseKernel.ts` `slerpQ`).
const LINEAR_COSINE := 0.9995

static func trs(record: Dictionary) -> Dictionary:
	return {
		"t": Vector3(record.t[0], record.t[1], record.t[2]),
		"r": Quaternion(record.r[0], record.r[1], record.r[2], record.r[3]),
		"s": Vector3(record.s[0], record.s[1], record.s[2]),
	}

## A local as a matrix. Quaternions are normalised first: quantised clip
## samples and float32 binds sit a few ulps off unit length.
static func matrix(local: Dictionary) -> Transform3D:
	return Transform3D(Basis(local.r.normalized()).scaled(local.s), local.t)

## Each node's or joint's bind as a local, from a manifest hierarchy.
static func binds(nodes: Array) -> Array:
	return nodes.map(func(node): return trs(node.bind))

## World transforms from `locals`; parents precede children (`worldTransforms`).
static func worlds(nodes: Array, locals: Array) -> Array[Transform3D]:
	var out: Array[Transform3D] = []
	for i in nodes.size():
		var m := matrix(locals[i])
		var parent := int(nodes[i].parent)
		out.append(m if parent < 0 else out[parent] * m)
	return out

## `quat.fromDegrees(x, y, z, "xyz")`.
static func from_degrees_xyz(turn: Vector3) -> Quaternion:
	var h := turn * (PI / 180.0) / 2.0
	var c1 := cos(h.x)
	var c2 := cos(h.y)
	var c3 := cos(h.z)
	var s1 := sin(h.x)
	var s2 := sin(h.y)
	var s3 := sin(h.z)
	return Quaternion(s1 * c2 * c3 + c1 * s2 * s3, c1 * s2 * c3 - s1 * c2 * s3, c1 * c2 * s3 + s1 * s2 * c3, c1 * c2 * c3 - s1 * s2 * s3)

## Every node's local for `input` (an `Articulation`), from the binds of
## `nodes` and `rig` (the manifest's), as `articulate` poses them.
static func articulate(nodes: Array, rig: Dictionary, input: Dictionary) -> Array:
	var out := binds(nodes)
	var turned := func(node: int, axis: Vector3, angle: float) -> void:
		out[node].r = out[node].r * Quaternion(axis, angle)
	if int(rig.turret) >= 0:
		turned.call(int(rig.turret), Vector3(0, 0, 1), float(input.turret_yaw))
	if int(rig.gun) >= 0:
		var gun := int(rig.gun)
		var pitch := clampf(float(input.gun_pitch), float(rig.pitch.gun[0]), float(rig.pitch.gun[1]))
		turned.call(gun, Vector3(0, 1, 0), -pitch)
		# Recoil runs the gun back along its own bore, pitched with it.
		var stroke: float = INF if rig.stroke == null else float(rig.stroke)
		out[gun].t += out[gun].r * Vector3(-minf(float(input.recoil), stroke), 0, 0)
	if int(rig.hmg) >= 0:
		turned.call(int(rig.hmg), Vector3(0, 0, 1), float(input.hmg_yaw))
	if int(rig.hmgGun) >= 0:
		turned.call(int(rig.hmgGun), Vector3(0, 1, 0), -clampf(float(input.hmg_pitch), float(rig.pitch.hmg[0]), float(rig.pitch.hmg[1])))
	for wheel in rig.wheels:
		var travel: float = input.travel_l if wheel.left else input.travel_r
		turned.call(int(wheel.node), Vector3(0, 1, 0), travel / float(wheel.radius))
	for rotor in rig.rotors:
		turned.call(int(rotor.node), Vector3(0, 0, 1), float(input.rotor) / float(rotor.radius))
		if float(input.rotor_blur) > 0.0:
			out[int(rotor.node)].s = Vector3.ZERO
	for motion in rig.deploy:
		var s := clampf((float(input.deploy) - float(motion.start)) / (float(motion.end) - float(motion.start)), 0.0, 1.0)
		if s == 0.0:
			continue
		var local: Dictionary = out[int(motion.node)]
		local.t += Vector3(motion.move[0], motion.move[1], motion.move[2]) * s
		local.r = local.r * from_degrees_xyz(Vector3(motion.turn[0], motion.turn[1], motion.turn[2]) * s)
	return out

## Each track's link offset, in links (`trackScroll`): `[left, right]`.
static func track_scroll(rig: Dictionary, input: Dictionary) -> Vector2:
	var scroll := Vector2.ZERO
	for left in [true, false]:
		for track in rig.tracks:
			if track.left == left:
				var travel: float = input.travel_l if left else input.travel_r
				scroll[0 if left else 1] = travel / float(track.linkPitch)
				break
	return scroll

## The two frames either side of `phase` and the weight of the second
## (`clipFrames`): `[f0, f1, w]`.
static func clip_frames(clip: Dictionary, phase: float) -> Array:
	var last := int(clip.frames) - 1
	if last <= 0:
		return [0, 0, 0.0]
	var p := phase - floorf(phase) if clip.loop else clampf(phase, 0.0, 1.0)
	var frame := minf(last, p * float(clip.duration) * float(clip.rate))
	var f0 := mini(last, floori(frame))
	return [f0, mini(last, f0 + 1), frame - f0]

## The kernel's shortest-arc slerp (`slerpQ`), on (x, y, z, w) vectors.
static func slerp_q(a: Vector4, b: Vector4, weight: float) -> Vector4:
	if weight <= 0.0:
		return a
	if weight >= 1.0:
		return b
	var cosine := a.dot(b)
	if cosine < 0.0:
		b = -b
		cosine = -cosine
	var result: Vector4
	if cosine > LINEAR_COSINE:
		result = a * (1.0 - weight) + b * weight
	else:
		var theta := atan2(sqrt(maxf(0.0, 1.0 - cosine * cosine)), cosine)
		result = a * sin((1.0 - weight) * theta) + b * sin(weight * theta)
	var magnitude := result.length()
	return result / (magnitude if magnitude > 0.0 else 1.0)

## Joint `joint`'s local at frames `frames` of a clip starting at float
## `offset` of `table`, for a body of `joints` joints, as the kernel samples
## it: `[rotation, translation and scale]`.
static func sample(table: PackedFloat32Array, offset: int, frames: Array, joint: int, joints: int) -> Array:
	var a := offset + (int(frames[0]) * joints + joint) * SAMPLE_FLOATS
	var b := offset + (int(frames[1]) * joints + joint) * SAMPLE_FLOATS
	var w: float = frames[2]
	var q := slerp_q(Vector4(table[a], table[a + 1], table[a + 2], table[a + 3]), Vector4(table[b], table[b + 1], table[b + 2], table[b + 3]), w)
	var ts := Vector4(table[a + 4], table[a + 5], table[a + 6], table[a + 7]).lerp(Vector4(table[b + 4], table[b + 5], table[b + 6], table[b + 7]), w)
	return [q, ts]

## A body's joint worlds at `clip` and `phase`, crossfaded over by `blend`
## (`{clip, phase, weight}`, or null), as the pose kernel composes them.
static func pose_worlds(body: Dictionary, table: PackedFloat32Array, clip: String, phase: float, blend: Variant) -> Array[Transform3D]:
	var joints: Array = body.joints
	var n := joints.size()
	var a: Dictionary = body.clips[clip]
	var fa := clip_frames(a, phase)
	var b: Dictionary = body.clips[blend.clip] if blend != null and body.clips.has(blend.clip) else a
	var fb := clip_frames(b, blend.phase) if blend != null else fa
	var weight: float = float(blend.weight) if blend != null and body.clips.has(blend.clip) else 0.0
	var out: Array[Transform3D] = []
	for j in n:
		var local := sample(table, int(a.offset), fa, j, n)
		if weight > 0.0:
			var other := sample(table, int(b.offset), fb, j, n)
			local = [slerp_q(local[0], other[0], weight), local[1].lerp(other[1], weight)]
		var q: Vector4 = local[0]
		var ts: Vector4 = local[1]
		var m := Transform3D(Basis(Quaternion(q.x, q.y, q.z, q.w).normalized()).scaled(Vector3(ts.w, ts.w, ts.w)), Vector3(ts.x, ts.y, ts.z))
		var parent := int(joints[j].parent)
		out.append(m if parent < 0 else out[parent] * m)
	return out
