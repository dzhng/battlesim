extends RefCounted

## The browser's pose driver (`packages/battle-renderer/src/models/poseDriver.ts`),
## ported line for line: the one place a pose is derived from what the
## simulation published. Each `update` takes a feed frame (`pose_feed.gd`) and
## poses every soldier (clip, phase, crossfade, facing, lean), every vehicle
## (`Articulation`: turret and HMG bearings, eased elevations, recoil, track
## and wheel travel, rotors, a flying aircraft's lean), and lays the fallen as
## corpses, at most `corpses.max` of them.
##
## Everything is per soldier: his gait and facing come from his own velocity.
## Positions are sim space `[x, y, z]` in double precision, as the browser's
## arithmetic is (a float32 `Vector3` would move a walker's phase). The frame is rewritten in place
## by every `update`: poses are the driver's own dictionaries, valid until the
## next call.

const PoseMath = preload("res://units/pose_math.gd")
const Mulberry32 = preload("res://core/mulberry32.gd")

## The ways a soldier moves at their bearings from his facing (`WAY_BEARINGS`).
const WAYS := [["f", 0.0], ["l", PI / 2.0], ["b", PI], ["r", -PI / 2.0]]
const WAY_HOLD_RAD := 0.2
## `INFANTRY_MOVES`, `INFANTRY_DEATHS`, `INFANTRY_FALL_BACK`, `INFANTRY_FALLS`.
const MOVES := {"stand": "walk_aim", "kneel": "crouch", "prone": "crawl"}
const DEATHS := {"stand": "death", "kneel": "death_kneel", "prone": "death_prone"}
const FALL_BACK := "death_back"
const FALLS := {"death": 0, "death_kneel": 0, "death_prone": 0, "death_back": 1}
## `GRAVITY` (`effects/cookOff.ts`), m/s².
const GRAVITY := 9.81
const REST_ARTICULATION := {"turret_yaw": 0.0, "gun_pitch": 0.0, "recoil": 0.0, "hmg_yaw": 0.0, "hmg_pitch": 0.0, "travel_l": 0.0, "travel_r": 0.0, "deploy": 0.0, "rotor": 0.0, "rotor_blur": 0.0}

class SoldierState:
	var pose: Dictionary
	var at: Array
	var lean_t := 0.0
	var lean_at: Array
	var posture := "stand"
	var turn: float
	var tempo: float
	var watch: bool
	var stance: Dictionary
	var alert_at := -INF
	var fading := {"clip": "idle", "phase": 0.0, "weight": 0.0}
	var fade_left := 0.0
	var last_shots := 0.0
	var own_shots := 0.0
	var fired_at := -INF
	var fell_at = null
	var seen := 0

class VehicleState:
	var pose: Dictionary
	var gun_shots := 0.0
	var fired_at := -INF
	var seen := 0
	## A flying aircraft's eased velocity and acceleration, and its last lean.
	var flight = null

## `presentation.pose`.
var _feel: Dictionary
var _lean_hold: float
## Unit kind -> its `units.types` record.
var _types: Dictionary
## `(kind, clip, soldier pose) -> {duration, loop, stride} or null`.
var _clip: Callable
var _soldiers := {}
var _vehicles := {}
var _dying := {}
## The static corpses in the order they came to lie: soldier -> corpse.
var _corpse_map := {}
var _unshown := {}
var _gone := {}
var _last_fallen = null
var _time = null
var _generation := 0
var out := {"soldiers": [], "vehicles": [], "corpses": [], "corpsesVersion": 0, "fading": []}

func _init(feel: Dictionary, lean_hold: float, types: Dictionary, clip: Callable) -> void:
	_feel = feel
	_lean_hold = lean_hold
	_types = types
	_clip = clip

## `loopStart`: where a soldier starts a looping clip, his own offset.
static func loop_start(soldier: int) -> float:
	return fmod(soldier * 0.6180339887498949, 1.0)

## `restManner`: his gaze's stray, his idle's tempo, and whether he watches.
static func rest_manner(soldier: int, rest: Dictionary) -> Dictionary:
	var state := Mulberry32.create(Mulberry32.imul(soldier + 1, 0x9e3779b1))
	var turn := (Mulberry32.sample(state) * 2.0 - 1.0) * float(rest.turn_rad)
	var tempo := float(rest.tempo[0]) + Mulberry32.sample(state) * (float(rest.tempo[1]) - float(rest.tempo[0]))
	return {"turn": turn, "tempo": tempo, "watch": soldier % int(rest.watch_every) == 2}

## `stanceManner`: his posture firing and pinned, and whether he falls back.
static func stance_manner(soldier: int, stance: Dictionary) -> Dictionary:
	var u := fmod(soldier * 0.41421356237309515, 1.0)
	var firing := "prone" if u < float(stance.fire_prone) else "stand" if u >= 1.0 - float(stance.fire_stand) else "kneel"
	var pinned := "kneel" if u >= 1.0 - float(stance.pinned_kneel) else "prone"
	var falls_back := fmod(soldier * 0.7548776662466927, 1.0) < float(stance.fall_back)
	return {"firing": firing, "pinned": pinned, "fallsBack": falls_back}

static func _fall_of(posture: String, stance: Dictionary) -> String:
	return FALL_BACK if posture == "stand" and stance.fallsBack else DEATHS[posture]

## `recoilAt`: the gun's run-back `since` seconds after a shot.
static func recoil_at(since: float, mount: Dictionary) -> float:
	if not (since >= 0.0) or since >= float(mount.recoil_return_s):
		return 0.0
	var left := 1.0 - since / float(mount.recoil_return_s)
	return float(mount.recoil_m) * left * left

static func _way_of(rel: float, current: String, gait: String) -> String:
	var held := current.substr(gait.length() + 1) if current.begins_with(gait + "_") else ""
	for way in WAYS:
		if way[0] == held and absf(PoseMath.delta_angle(way[1], rel)) <= PI / 4.0 + WAY_HOLD_RAD:
			return held
	var best: String = WAYS[0][0]
	var off := INF
	for way in WAYS:
		var d := absf(PoseMath.delta_angle(way[1], rel))
		if d < off:
			best = way[0]
			off = d
	return best

static func _gait_of_clip(clip: String) -> String:
	var cut := clip.rfind("_")
	var gait := "" if cut < 0 else clip.substr(0, cut)
	return gait if MOVES.values().has(gait) else ""

static func _approach(from: float, to: float, step: float) -> float:
	var d := to - from
	return to if absf(d) <= step else from + signf(d) * step

func reset() -> void:
	_soldiers.clear()
	_vehicles.clear()
	_dying.clear()
	_corpse_map.clear()
	_unshown.clear()
	_gone.clear()
	_last_fallen = null
	_time = null
	out.corpses.clear()
	out.corpsesVersion += 1
	out.fading.clear()

## Advances to `frame` (`pose_feed.gd`) and poses everything in it.
func update(frame: Dictionary) -> Dictionary:
	var time: float = frame.time
	if _time != null and time < _time:
		reset()
	var dt := 0.0 if _time == null else maxf(0.0, time - _time)
	_time = time
	_generation += 1
	var generation := _generation
	out.soldiers.clear()
	out.vehicles.clear()
	for unit in frame.units:
		if _types[unit.kind].hull != null:
			out.vehicles.append(_vehicle(unit, time, dt, generation))
		else:
			_squad(unit, time, dt, generation)
	var relisted := false
	if not is_same(frame.fallen, _last_fallen):
		_last_fallen = frame.fallen
		relisted = _reconcile_fallen(frame.fallen, time)
	if _advance_dying(time, dt, generation) or relisted:
		_publish_corpses(time)
	_advance_fading(time)
	for id in _soldiers.keys():
		if _soldiers[id].seen != generation:
			_soldiers.erase(id)
			_dying.erase(id)
	for key in _vehicles.keys():
		if _vehicles[key].seen != generation:
			_vehicles.erase(key)
	return out

func _aircraft_attitude(state: VehicleState, unit: Dictionary, dt: float) -> Variant:
	if state.flight == null:
		state.flight = {"velocity": Vector2.ZERO, "accel": Vector2.ZERO, "attitude": null}
	var f: Dictionary = state.flight
	if dt == 0.0:
		return f.attitude
	var feel: Dictionary = _feel.attitude
	var k := 1.0 - exp(-dt / float(feel.smoothing_s))
	var p: Array = state.pose.position
	var v := Vector2((unit.position[0] - p[0]) / dt, (unit.position[1] - p[1]) / dt)
	var o: Vector2 = f.velocity
	f.velocity = Vector2(o.x + (v.x - o.x) * k, o.y + (v.y - o.y) * k)
	var a := Vector2((f.velocity.x - o.x) / dt, (f.velocity.y - o.y) / dt)
	f.accel = Vector2(f.accel.x + (a.x - f.accel.x) * k, f.accel.y + (a.y - f.accel.y) * k)
	var most := float(feel.max_deg) * PI / 180.0
	var gain := float(feel.gain)
	var c := cos(float(unit.yaw))
	var s := sin(float(unit.yaw))
	var pitch := -clampf(gain * atan((f.accel.x * c + f.accel.y * s) / GRAVITY), -most, most)
	var roll := -clampf(gain * atan((-f.accel.x * s + f.accel.y * c) / GRAVITY), -most, most)
	f.attitude = {"pitch": pitch, "roll": roll} if absf(pitch) + absf(roll) > 1e-9 else null
	return f.attitude

func _squad(unit: Dictionary, time: float, dt: float, generation: int) -> void:
	var gait: Dictionary = _feel.gait
	var rest: Dictionary = _feel.rest
	var slide: Dictionary = _feel.lean
	var type: Dictionary = _types[unit.kind]
	for soldier in unit.soldiers:
		var selected = null if soldier.activeMount == null or int(soldier.activeMount) >= unit.mounts.size() else unit.mounts[int(soldier.activeMount)]
		var shots: float = selected.shots if selected != null else 0.0
		var aim: float = selected.bearing if selected != null else float(unit.yaw)
		var id := int(soldier.id)
		var state: SoldierState = _soldiers.get(id) if _soldiers.has(id) else _new_soldier(soldier, unit, shots, generation)
		state.seen = generation
		var pose: Dictionary = state.pose
		var carried = null
		for m in unit.mounts.size():
			if unit.mounts[m].operator != null and int(unit.mounts[m].operator) == id:
				carried = m
				break
		var position: Array = soldier.position
		var dx: float = position[0] - state.at[0]
		var dy: float = position[1] - state.at[1]
		var moved := sqrt(dx * dx + dy * dy)
		var speed := moved / dt if dt > 0.0 else 0.0
		var authored_hold = null
		if carried != null and carried < type.mounts.size():
			authored_hold = type.mounts[carried].activePose
		var active = null if soldier.activeMount == null else int(soldier.activeMount)
		var holding: bool = authored_hold != null and active == carried and speed < float(gait.walk_mps)
		# Presentation equipment only; the feed remains the observed authority.
		var active_mount = null if authored_hold != null and active == carried and not holding else active
		var equipment_changed: bool = pose.operatorMount != carried or (carried != null and (pose.activeMount == carried) != (active_mount == carried))
		if pose.activeMount != active_mount:
			state.fired_at = -INF
			state.last_shots = shots
			state.own_shots = soldier.shots if soldier.has("shots") else 0.0
		pose.operatorMount = carried
		pose.activeMount = active_mount
		var own = soldier.shots if soldier.has("shots") else null
		if (shots > state.last_shots) if own == null else (own > state.own_shots):
			state.fired_at = time
		if shots > state.last_shots:
			state.alert_at = time
		state.last_shots = shots
		state.own_shots = own if own != null else 0.0
		var firing := time - state.fired_at < _lean_hold

		var lean = soldier.get("lean")
		var held: String
		if soldier.get("posture") != null:
			held = soldier.posture
		elif lean != null:
			held = "kneel"
		elif unit.pinned:
			held = state.stance.pinned
		elif firing:
			held = state.stance.firing
		else:
			held = "stand"
		var posture := "kneel" if held == "prone" and speed >= float(gait.crawl_mps) else held
		state.posture = posture
		var running := speed >= float(gait.run_mps)
		var gait_of: String = "" if running or (posture == "stand" and not firing) else MOVES[posture]

		var settled := clampf((time - state.alert_at - float(rest.settle_s[0])) / float(rest.settle_s[1]), 0.0, 1.0)
		var travel := atan2(dy, dx)
		var target := travel if speed > float(gait.facing_mps) and gait_of.is_empty() else aim + state.turn * settled
		var turn := PoseMath.delta_angle(pose.facing, target)
		var step := float(gait.turn_rad_s) * dt
		if holding:
			pose.facing = aim
		else:
			pose.facing += turn if absf(turn) <= step else signf(turn) * step

		var clip: String
		if holding:
			clip = authored_hold.clip
		elif running:
			clip = "run"
		elif speed >= float(gait.walk_mps):
			clip = "%s_%s" % [gait_of, _way_of(PoseMath.delta_angle(pose.facing, travel), pose.clip, gait_of)] if not gait_of.is_empty() else "walk"
		elif posture == "prone":
			clip = "prone_pinned"
		elif posture == "kneel":
			clip = "kneel_fire"
		elif state.watch or firing:
			clip = "stand_aim"
		else:
			clip = "idle"
		if equipment_changed:
			_reset_clip(state, clip)
		if holding:
			pose.clip = authored_hold.clip
			pose.phase = float(authored_hold.phase)
			pose.blend = null
			state.fade_left = 0.0
		else:
			_advance(state, clip, moved, dt)
		state.at = position
		if lean != null:
			state.lean_at = [lean[0], lean[1], position[2]]
		state.lean_t = minf(1.0, state.lean_t + dt / float(slide.out_s)) if lean != null else maxf(0.0, state.lean_t - dt / float(slide.back_s))
		pose.position = PoseMath.lerp3(position, state.lean_at, PoseMath.sine_in_out(state.lean_t))
		pose.unit = unit.id
		out.soldiers.append(pose)

func _new_soldier(soldier: Dictionary, unit: Dictionary, shots: float, generation: int) -> SoldierState:
	var id := int(soldier.id)
	var manner := rest_manner(id, _feel.rest)
	var state := SoldierState.new()
	state.pose = {
		"soldier": id,
		"unit": unit.id,
		"kind": unit.kind,
		"slot": soldier.slot,
		"operatorMount": null,
		"activeMount": null,
		"side": unit.side,
		"position": soldier.position,
		"facing": float(unit.yaw) + manner.turn,
		"clip": "idle",
		"phase": loop_start(id),
		"blend": null,
	}
	state.at = soldier.position
	state.lean_at = soldier.position
	state.turn = manner.turn
	state.tempo = manner.tempo
	state.watch = manner.watch
	state.stance = stance_manner(id, _feel.stance)
	# Rounds fired before he was first seen are not his shot.
	state.last_shots = shots
	state.own_shots = soldier.shots if soldier.has("shots") else 0.0
	state.seen = generation
	_soldiers[id] = state
	return state

func _reconcile_fallen(fallen: Array, time: float) -> bool:
	var changed := false
	var listed := {}
	var fading := {}
	for f in out.fading:
		fading[f.corpse.soldier] = f.corpse
	for f in fallen:
		var id := int(f.soldier)
		listed[id] = true
		if _corpse_map.has(id):
			var corpse: Dictionary = _corpse_map[id]
			if corpse.position != f.position:
				corpse.position = f.position
				changed = true
			continue
		if _gone.has(id):
			if fading.has(id):
				fading[id].position = f.position
			continue
		var state: SoldierState = _soldiers.get(id)
		if state != null and state.fell_at == null:
			state.fell_at = time
			state.pose.position = f.position
			state.pose.unit = -1
			var changed_hold: bool = state.pose.operatorMount != null
			state.pose.activeMount = null
			# He falls from the posture he is in, never getting up to die.
			var fall := _fall_of(state.posture, state.stance)
			if changed_hold:
				_reset_clip(state, fall)
			else:
				_switch_to(state, fall)
			_dying[id] = true
		elif state != null:
			state.pose.position = f.position
		else:
			# Never seen fall: he lies as he would have fallen standing.
			_lay({
				"lies": FALLS[_fall_of("stand", stance_manner(id, _feel.stance))],
				"operatorMount": null,
				"soldier": id,
				"kind": f.kind,
				"slot": f.slot,
				"side": f.side,
				"position": f.position,
				"yaw": float(f.yaw),
			})
			changed = true
	for id in _corpse_map.keys():
		if not listed.has(id):
			_corpse_map.erase(id)
			_unshown.erase(id)
			changed = true
	for id in _gone.keys():
		if not listed.has(id):
			_gone.erase(id)
	out.fading = out.fading.filter(func(f): return listed.has(f.corpse.soldier))
	for id in _dying.keys():
		if not listed.has(id):
			_dying.erase(id)
			_soldiers.erase(id)
	return changed

func _advance_dying(time: float, dt: float, generation: int) -> bool:
	var laid := false
	for id in _dying.keys():
		var state: SoldierState = _soldiers[id]
		state.seen = generation
		var pose: Dictionary = state.pose
		pose.activeMount = null
		var facts = _clip.call(pose.kind, pose.clip, pose)
		pose.phase = minf(1.0, (time - state.fell_at) / float(facts.duration)) if facts != null else 1.0
		_fade(state, dt)
		if pose.phase >= 1.0 and pose.blend == null:
			_dying.erase(id)
			_soldiers.erase(id)
			_lay({
				"lies": FALLS[pose.clip],
				"soldier": id,
				"kind": pose.kind,
				"operatorMount": pose.operatorMount,
				"slot": pose.slot,
				"side": pose.side,
				"position": pose.position,
				"yaw": pose.facing,
			})
			laid = true
			continue
		out.soldiers.append(pose)
	return laid

func _lay(corpse: Dictionary) -> void:
	_corpse_map[corpse.soldier] = corpse
	_unshown[corpse.soldier] = true

func _publish_corpses(time: float) -> void:
	var excess: int = _corpse_map.size() - int(_feel.corpses.max)
	if excess > 0:
		for id in _corpse_map.keys().slice(0, excess):
			var corpse: Dictionary = _corpse_map[id]
			_corpse_map.erase(id)
			_gone[id] = true
			if not _unshown.has(id):
				out.fading.append({"corpse": corpse, "sink": 0.0, "since": time})
	_unshown.clear()
	out.corpses = _corpse_map.values()
	out.corpsesVersion += 1

func _advance_fading(time: float) -> void:
	var fade_s := float(_feel.corpses.fade_s)
	var sink_m := float(_feel.corpses.sink_m)
	var kept := []
	for f in out.fading:
		var t := (time - float(f.since)) / fade_s
		if t >= 1.0:
			continue
		f.sink = sink_m * PoseMath.sine_in(t)
		kept.append(f)
	out.fading = kept

func _advance(state: SoldierState, clip: String, moved: float, dt: float) -> void:
	var pose: Dictionary = state.pose
	if clip != pose.clip:
		# Turning his way within one gait keeps his step: its ways share a phase.
		var gait := _gait_of_clip(clip)
		var phase: float = pose.phase
		_switch_to(state, clip)
		if not gait.is_empty() and gait == _gait_of_clip(state.fading.clip):
			pose.phase = phase
	var facts = _clip.call(pose.kind, pose.clip, pose)
	if facts != null:
		# Locomotion advances with ground covered, so feet do not slide;
		# everything else with time, the idle at his own tempo.
		var step: float = moved / float(facts.stride) if facts.stride != null and float(facts.stride) != 0.0 else (dt / float(facts.duration)) * (state.tempo if pose.clip == "idle" else 1.0)
		pose.phase = fmod(pose.phase + step, 1.0) if facts.loop else minf(1.0, pose.phase + step)
	_fade(state, dt)

func _reset_clip(state: SoldierState, clip: String) -> void:
	var pose: Dictionary = state.pose
	pose.clip = clip
	var facts = _clip.call(pose.kind, clip, pose)
	pose.phase = loop_start(pose.soldier) if facts != null and facts.loop else 0.0
	pose.blend = null
	state.fade_left = 0.0

func _switch_to(state: SoldierState, clip: String) -> void:
	var pose: Dictionary = state.pose
	state.fading.clip = pose.clip
	state.fading.phase = pose.phase
	state.fading.weight = 1.0
	pose.blend = state.fading
	state.fade_left = float(_feel.gait.fade_s)
	pose.clip = clip
	var facts = _clip.call(pose.kind, clip, pose)
	pose.phase = loop_start(pose.soldier) if facts != null and facts.loop else 0.0

func _fade(state: SoldierState, dt: float) -> void:
	var pose: Dictionary = state.pose
	if pose.blend == null:
		return
	state.fade_left = maxf(0.0, state.fade_left - dt)
	state.fading.weight = state.fade_left / float(_feel.gait.fade_s)
	if state.fading.weight <= 0.0:
		pose.blend = null

func _vehicle(unit: Dictionary, time: float, dt: float, generation: int) -> Dictionary:
	var type: Dictionary = _types[unit.kind]
	var roles: Array = type.roles
	var gun := roles.find("gun")
	var hmg := roles.find("hmg")
	var gun_mount = unit.mounts[gun] if gun >= 0 and gun < unit.mounts.size() else null
	var hmg_mount = unit.mounts[hmg] if hmg >= 0 and hmg < unit.mounts.size() else null
	var gun_target: float = gun_mount.elevation if gun_mount != null else 0.0
	var hmg_target: float = hmg_mount.elevation if hmg_mount != null else 0.0
	var key := PoseMath.side_key(int(unit.id), unit.side, "blue")
	var state: VehicleState = _vehicles.get(key)
	if state == null:
		var flies: bool = type.airborne
		state = VehicleState.new()
		state.gun_shots = gun_mount.shots if gun_mount != null else 0.0
		var articulation := REST_ARTICULATION.duplicate()
		articulation.gun_pitch = gun_target
		articulation.hmg_pitch = hmg_target
		articulation.rotor_blur = 1.0 if flies else 0.0
		state.pose = {
			"unit": unit.id,
			"kind": unit.kind,
			"side": unit.side,
			"position": unit.position,
			"yaw": float(unit.yaw),
			"airborne": flies,
			"articulation": articulation,
			"tilt": null,
		}
		state.seen = generation
		_vehicles[key] = state
	state.seen = generation
	var pose: Dictionary = state.pose
	var a: Dictionary = pose.articulation
	var position: Array = unit.position
	if pose.airborne:
		a.rotor += float(_feel.rotor.tip_mps) * dt
	else:
		var forward: float = (position[0] - pose.position[0]) * cos(float(unit.yaw)) + (position[1] - pose.position[1]) * sin(float(unit.yaw))
		var turned := PoseMath.delta_angle(pose.yaw, unit.yaw)
		var half: float = type.hull.halfTrack
		a.travel_l += forward - turned * half
		a.travel_r += forward + turned * half
	var flying = _aircraft_attitude(state, unit, dt) if pose.airborne and unit.get("attitude") == null else null
	pose.position = position
	pose.yaw = float(unit.yaw)
	var attitude = unit.get("attitude") if unit.get("attitude") != null else flying
	# Nose up about its +Y, then right side down about its +X (sim axes).
	pose.tilt = null if attitude == null else Basis(Vector3(0, 1, 0), -float(attitude.pitch)) * Basis(Vector3(1, 0, 0), float(attitude.roll))
	a.deploy = unit.deployment if unit.deployment != null else 0.0

	# The turret on the cannon's bearing, the HMG relative to what it rides;
	# elevations eased; a new cannon round recoils the gun.
	var on = type.mounts[hmg].on if hmg >= 0 else null
	var carrier = unit.mounts[on] if on != null and on < unit.mounts.size() else null
	var carrier_bearing: float = carrier.bearing if carrier != null else float(unit.yaw)
	a.turret_yaw = PoseMath.delta_angle(unit.yaw, gun_mount.bearing) if gun_mount != null else 0.0
	var feel: Dictionary = _feel.mount
	a.gun_pitch = _approach(a.gun_pitch, gun_target, float(feel.gun_elevation_rad_s) * dt)
	a.hmg_yaw = PoseMath.delta_angle(carrier_bearing, hmg_mount.bearing) if hmg_mount != null else 0.0
	a.hmg_pitch = _approach(a.hmg_pitch, hmg_target, float(feel.hmg_elevation_rad_s) * dt)
	if gun_mount != null and gun_mount.shots > state.gun_shots:
		state.fired_at = time
	if gun_mount != null:
		state.gun_shots = gun_mount.shots
	a.recoil = recoil_at(time - state.fired_at, feel)
	return pose
