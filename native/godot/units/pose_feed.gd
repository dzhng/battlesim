extends RefCounted

## The observation as the pose driver reads it, ported from the browser's
## `ObservationFeed` (`apps/battle-lab/src/poseFeed.ts`): a side's decoded
## publication and its interpolated poses (`tick_interpolator.gd`) become a
## feed frame. Every own unit and identified enemy, its soldiers by member id
## with their slot, selected weapon, lean and own launches; each unit's
## weapon poses; every downed airframe still falling; and the fallen the side
## knows of, kept as one list until the corpse view changes, so the driver
## reconciles only those changes.

const PoseMath = preload("res://units/pose_math.gd")
const Launches = preload("res://units/launches.gd")

var _side: String
var _types: Dictionary
var _observation = null
var _fallen: Array = []
var _corpses = null
## Each soldier's launches so far, by soldier id: per mount, a count.
var _shots := {}
var _launches := Launches.new()
var _last_tick := -1
## Each unit's mount records, by `sideKey`.
var _mounts := {}

## `side` is the capture's; `types` the manifest's unit types.
func _init(side: String, types: Dictionary) -> void:
	_side = side
	_types = types

## The feed frame for one drawn sample (`tick_interpolator.gd` `frame`).
func frame(sample: Dictionary) -> Dictionary:
	var observation: Dictionary = sample.observation
	var enemy := "red" if _side == "blue" else "blue"
	if not is_same(observation, _observation):
		_observation = observation
		if not is_same(observation.corpses, _corpses):
			_corpses = observation.corpses
			_fallen = observation.corpses.map(func(c): return {
				"soldier": c.soldier,
				"position": c.position,
				"yaw": c.yaw,
				"kind": c.kind,
				"slot": c.slot,
				"side": _side if c.own else enemy,
			})
		var tick := int(observation.tick)
		if tick != _last_tick:
			if tick < _last_tick:
				_launches.reset()
				_shots.clear()
			var gap := _last_tick >= 0 and tick != _last_tick + 1
			_last_tick = tick
			for launch in _launches.note(observation, _side, _types, gap):
				if not _shots.has(launch.soldier):
					_shots[launch.soldier] = {}
				var counts: Dictionary = _shots[launch.soldier]
				counts[launch.mount] = counts.get(launch.mount, 0) + 1
	var units := []
	var own := {}
	for u in observation.own:
		own[u.id] = u
	for pose in sample.own:
		var u = own.get(pose.id)
		if u != null:
			units.append(_unit(pose, u.kind, _side, u, u.suppression == "pinned"))
	var enemies := {}
	for e in observation.identified:
		enemies[e.id] = e
	for pose in sample.identified:
		var e = enemies.get(pose.id)
		if e != null:
			# The side cannot know an enemy's suppression.
			units.append(_unit(pose, e.kind, enemy, e, false))
	# A downed airframe falls on as the vehicle it was, crewless, its turret
	# as last seen, tipping into the fall.
	for c in sample.crashes:
		var side := _side if c.own else enemy
		units.append({
			"id": c.id,
			"kind": c.kind,
			"side": side,
			"position": c.position,
			"yaw": c.yaw,
			"soldiers": [],
			"mounts": _mounts.get(PoseMath.side_key(int(c.id), side, "blue"), []),
			"deployment": null,
			"pinned": false,
			"attitude": {"pitch": c.pitch, "roll": c.roll},
		})
	return {"time": sample.time, "units": units, "fallen": _fallen}

func _unit(pose: Dictionary, kind: String, side: String, published: Dictionary, pinned: bool) -> Dictionary:
	var key := PoseMath.side_key(int(pose.id), side, "blue")
	var mounts := _mounts_of(published.weaponPoses)
	_mounts[key] = mounts
	var soldiers := []
	for k in pose.members.size():
		var id = pose.memberIds[k]
		var at: int = published.memberIds.find(id)
		var active = published.memberActiveMounts[at] if at >= 0 and at < published.memberActiveMounts.size() else null
		var lean = published.memberLeans[at] if at >= 0 and at < published.memberLeans.size() else null
		soldiers.append({
			"id": id,
			"slot": published.memberSlots[at] if at >= 0 and at < published.memberSlots.size() else 0,
			"activeMount": active,
			"position": pose.members[k],
			"shots": 0.0 if active == null else float(_shots.get(int(id), {}).get(int(active), 0)),
			# Leans are discrete: the driver eases the slide from this visible fact.
			"lean": lean.at if lean != null else null,
		})
	return {
		"id": pose.id,
		"kind": kind,
		"side": side,
		"position": pose.position,
		"yaw": pose.yaw,
		"soldiers": soldiers,
		"mounts": mounts,
		"deployment": pose.deployment,
		"pinned": pinned,
	}

## Mount records in mount order, from a unit's weapon poses (`mountsOf`).
static func _mounts_of(poses: Array) -> Array:
	var into := []
	for p in poses:
		var m := int(p.mount)
		while into.size() <= m:
			into.append(null)
		into[m] = {"operator": p.operator, "bearing": p.bearing, "elevation": p.elevation, "shots": p.shots}
	for i in into.size():
		if into[i] == null:
			into[i] = {"operator": null, "bearing": 0.0, "elevation": 0.0, "shots": 0.0}
	return into
