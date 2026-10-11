extends RefCounted

## Presentation between completed ticks, as the browser's `TickInterpolator`
## (`web/src/battle/present/interpolate.ts`) blends it: units drawn a fraction
## of a tick behind the latest publication, blended from the previous one.
## Only units present in both blend, a soldier only with himself (by member
## id), and a falling airframe with itself; nothing is extrapolated.
##
## A frame record carries the interpolated sim time the browser drew with,
## and the latest tick: the publication at that tick is the one the poses
## blend toward, the one before it the one they blend from, and the
## fraction is where the record's time stands between their ticks.

const PoseMath = preload("res://units/pose_math.gd")

## The frame a record draws (`TickInterpolator.frame`): `{time, observation,
## own, identified, crashes}`, sim-space positions as `[x, y, z]` in double
## precision (as the browser's arithmetic is), or
## an empty dictionary before the first publication. `samples` are the
## capture's decoded observations in tick order, `at` the index of the one
## at or before the record's tick.
static func frame(samples: Array, at: int, sim_time: float, tick_hz: float) -> Dictionary:
	if at < 0:
		return {}
	var latest: Dictionary = samples[at]
	var previous: Dictionary = samples[at - 1] if at > 0 else {}
	var from: float = previous.tick if not previous.is_empty() else latest.tick
	var span: float = latest.tick - from
	var t := clampf((sim_time * tick_hz - from) / span, 0.0, 1.0) if span > 0.0 else 1.0
	var own_before := _by_id(previous.get("own", []))
	var identified_before := _by_id(previous.get("identified", []))
	var own := []
	for u in latest.own:
		var p = own_before.get(u.id)
		var deployment = u.deployment.progress if u.deployment != null else null
		if deployment != null and p != null and p.deployment != null:
			deployment = lerpf(p.deployment.progress, deployment, t)
		own.append(_blend_body(p, u, t, deployment))
	var identified := []
	for u in latest.identified:
		identified.append(_blend_body(identified_before.get(u.id), u, t, null))
	var crashes := []
	for c in latest.crashes:
		var p = null
		for q in previous.get("crashes", []):
			if q.id == c.id and q.own == c.own:
				p = q
				break
		var blended: Dictionary = c.duplicate()
		if p != null:
			blended.position = PoseMath.lerp3(p.position, c.position, t)
			blended.yaw = p.yaw + PoseMath.delta_angle(p.yaw, c.yaw) * t
			blended.pitch = lerpf(p.pitch, c.pitch, t)
			blended.roll = lerpf(p.roll, c.roll, t)
		crashes.append(blended)
	return {
		# The record's clock is the browser's presentation clock exactly.
		"time": sim_time,
		"observation": latest,
		"own": own,
		"identified": identified,
		"crashes": crashes,
	}

## The index of the latest of `samples` at or before `tick`, or -1.
static func latest_at(samples: Array, tick: int) -> int:
	var low := 0
	var high := samples.size() - 1
	var found := -1
	while low <= high:
		var mid := (low + high) / 2
		if int(samples[mid].tick) <= tick:
			found = mid
			low = mid + 1
		else:
			high = mid - 1
	return found

static func _by_id(units: Array) -> Dictionary:
	var out := {}
	for u in units:
		out[u.id] = u
	return out

## `u` blended from `p` (the previous tick's same unit, or null) by `t`.
static func _blend_body(p: Variant, u: Dictionary, t: float, deployment: Variant) -> Dictionary:
	if p == null:
		return {"id": u.id, "position": u.position, "yaw": u.yaw, "members": u.members, "memberIds": u.memberIds, "deployment": deployment}
	var before := {}
	for k in p.memberIds.size():
		before[p.memberIds[k]] = p.members[k]
	var members := []
	for k in u.members.size():
		var was = before.get(u.memberIds[k])
		members.append(u.members[k] if was == null else PoseMath.lerp3(was, u.members[k], t))
	return {
		"id": u.id,
		"position": PoseMath.lerp3(p.position, u.position, t),
		"yaw": p.yaw + PoseMath.delta_angle(p.yaw, u.yaw) * t,
		"members": members,
		"memberIds": u.memberIds,
		"deployment": deployment,
	}
