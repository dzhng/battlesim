# Frozen: a port of the per-frame presentation `crates/presentation` now owns; no fixes here, deleted with Godot (slice 25).
extends RefCounted

## Where parts of each posed model are drawn, ported from the browser's
## `DrawnModels` (`packages/battle-renderer/src/models/drawnModels.ts`), read
## from the same pose frame the units layer drew (`units/unit_layer.gd`), so
## an effect put there sits on the drawn model:
## - its muzzles: a vehicle mount's muzzle node under its articulation, and a
##   soldier's weapon `muzzle` socket in his clip, computed only for the
##   muzzles a frame's flashes ask after and remembered for the frame;
## - its rotors: each rotor's hub, axis, blades and the angle it turned since
##   the last drawn frame.
## Sim space, positions as `[x, y, z]`.

const Articulation = preload("res://units/articulation.gd")
const PoseMath = preload("res://units/pose_math.gd")

var _units
var _side: String
var _vehicles := {}
var _soldiers := {}
var _memo := {}
var _rotor_at := {}
var _rotor_turned := {}

## `units` is the units layer, which posed the frame; `side` the capture's.
func _init(units, side: String) -> void:
	_units = units
	_side = side

## Takes the pose frame the models are drawn from this frame.
func update(frame: Dictionary) -> void:
	_vehicles.clear()
	_soldiers.clear()
	_memo.clear()
	_rotor_turned.clear()
	if frame.is_empty():
		_rotor_at.clear()
		return
	for v in frame.vehicles:
		var key := PoseMath.side_key(int(v.unit), v.side, "blue")
		_vehicles[key] = v
		var was = _rotor_at.get(key)
		_rotor_turned[key] = 0.0 if was == null else float(v.articulation.rotor) - was
	_rotor_at.clear()
	for key in _vehicles:
		_rotor_at[key] = float(_vehicles[key].articulation.rotor)
	for s in frame.soldiers:
		_soldiers[PoseMath.side_key(int(s.soldier), s.side, "blue")] = s

## The drawn muzzle of a shot by `shooter` (an effect shooter's key) from its
## mount `mount`, or its soldier `soldier` when set (`MuzzleSource`); null
## when nothing of it is drawn.
func muzzle(shooter: int, mount: int, soldier) -> Variant:
	var of := _side if shooter % 2 == 0 else ("red" if _side == "blue" else "blue")
	var key := [shooter, mount, soldier]
	if not _memo.has(key):
		_memo[key] = _vehicle(of, shooter >> 1, mount) if soldier == null else _soldier(of, int(soldier), mount)
	return _memo[key]

func _vehicle(side: String, unit: int, mount: int) -> Variant:
	var v = _vehicles.get(PoseMath.side_key(unit, side, "blue"))
	if v == null:
		return null
	var record = _units.appearance(_units.resolve(v.kind, v.side, int(v.unit), 0, null, null))
	var roles: Array = _units.manifest().types[v.kind].roles
	if record == null or record.kind != "articulated" or mount >= roles.size():
		return null
	var node = record.rig.muzzles.get(roles[mount])
	if node == null:
		return null
	var worlds := Articulation.worlds(record.nodes, Articulation.articulate(record.nodes, record.rig, v.articulation))
	return _placed(v.position, float(v.yaw), worlds[int(node)].origin)

func _soldier(side: String, soldier: int, mount: int) -> Variant:
	var s = _soldiers.get(PoseMath.side_key(soldier, side, "blue"))
	if s == null or s.activeMount == null or int(s.activeMount) != mount:
		return null
	var name: String = _units.resolve(s.kind, s.side, int(s.soldier), int(s.slot), s.operatorMount, s.activeMount)
	var record = _units.appearance(name)
	if record == null or record.kind != "skinned" or record.muzzle == null:
		return null
	# `poseWorlds`: the clip at its phase, the binds where the body lacks it.
	var worlds := Articulation.pose_worlds(record, _units.table(record.table), s.clip, float(s.phase), null) if record.clips.has(s.clip) else Articulation.worlds(record.joints, Articulation.binds(record.joints))
	var socket := worlds[int(record.muzzle.joint)] * Articulation.matrix(Articulation.trs(record.muzzle.offset))
	return _placed(s.position, float(s.facing), socket.origin)

## Each drawn rotor of this frame's aircraft, posed as the model is
## (`DrawnRotor`).
func rotors() -> Array:
	var out := []
	for key in _vehicles:
		var v: Dictionary = _vehicles[key]
		if not v.airborne:
			continue
		var record = _units.appearance(_units.resolve(v.kind, v.side, int(v.unit), 0, null, null))
		if record == null or record.kind != "articulated" or record.rig.rotors.is_empty():
			continue
		# Posed with its rotors shown: the blur hides them, not their place.
		var articulation: Dictionary = v.articulation.duplicate()
		articulation.rotor_blur = 0.0
		var worlds := Articulation.worlds(record.nodes, Articulation.articulate(record.nodes, record.rig, articulation))
		var yaw := float(v.yaw)
		for rotor in record.rig.rotors:
			var m: Transform3D = worlds[int(rotor.node)]
			# A falling airframe's tilt carries the whole model.
			if v.tilt != null:
				m = Transform3D(v.tilt, Vector3.ZERO) * m
			var c := cos(float(rotor.rest))
			var s := sin(float(rotor.rest))
			var axis := _placed([0.0, 0.0, 0.0], yaw, m.basis.z)
			var blade := _placed([0.0, 0.0, 0.0], yaw, m.basis.x * c + m.basis.y * s)
			var mast = null
			if float(rotor.mast_radius) > 0.0:
				mast = {"base": _placed(v.position, yaw, m.origin - m.basis.z * float(rotor.mast_below)), "top": _placed(v.position, yaw, m.origin + m.basis.z * float(rotor.mast_above))}
			out.append({
				"hub": _placed(v.position, yaw, m.origin),
				"mast": mast,
				"mastRadius": float(rotor.mast_radius),
				"axis": _unit(axis),
				"blade": _unit(blade),
				"radius": float(rotor.radius),
				"blades": int(rotor.blades),
				"chord": float(rotor.chord),
				"turned": _rotor_turned.get(key, 0.0) / float(rotor.radius),
			})
	return out

## A model-space point placed at `position`, turned by `yaw` about +z.
static func _placed(position: Array, yaw: float, p: Vector3) -> Array:
	var c := cos(yaw)
	var s := sin(yaw)
	return [position[0] + p.x * c - p.y * s, position[1] + p.x * s + p.y * c, position[2] + p.z]

static func _unit(v: Array) -> Array:
	var length := sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2])
	return v if length == 0.0 else [v[0] / length, v[1] / length, v[2] / length]
