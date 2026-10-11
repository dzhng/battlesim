extends RefCounted

## Which soldier fired, from one side's publications: the soldier half of the
## browser's `LaunchTracker` (`packages/battle-renderer/src/effects/launches.ts`)
## over `effectPublication` (`apps/battle-lab/src/effectFeed.ts`). A squad's
## shot counter rising at tick T names the rounds whose stretches start at
## their muzzles in the next publication, one round a soldier, matched by
## kind; a round still flying is no new launch. A unit first seen shows no
## shot until its counter rises once more. The pose feed counts each
## soldier's launches, so the soldier who kneels to fire is the one who fired.
## Hull launches and tracers are the effects' (slice 17) and are not ported.

const PoseMath = preload("res://units/pose_math.gd")

## Shot counters by shooter key, as last published.
var _counters := {}
## Where the last publication's still-flying stretches ended: `[kind,
## shooter, x, y, z]` -> true.
var _flying := {}
## Squads' rises in the last publication, by shooter key and mount.
var _owed := {}

func reset() -> void:
	_counters.clear()
	_flying.clear()
	_owed.clear()

## The soldier launches `observation`, seen as `side`, shows, `[{soldier,
## mount}]`. Call once per publication, in order; `gap` says ticks were
## skipped since the last. `types` are the manifest's unit types.
func note(observation: Dictionary, side: String, types: Dictionary, gap: bool) -> Array:
	var out := []
	var starts := {}
	for p in observation.projectiles:
		if p.shooterMember == null or p.path.size() < 2:
			continue
		if not gap and _flying.has(_flight_key(p, p.path[0])):
			continue
		var shooter := int(p.shooterMember)
		if not starts.has(shooter):
			starts[shooter] = []
		starts[shooter].append(p)
	var enemy := "red" if side == "blue" else "blue"
	var shooters := []
	for u in observation.own:
		shooters.append([PoseMath.side_key(int(u.id), side, side), u])
	for e in observation.identified:
		shooters.append([PoseMath.side_key(int(e.id), enemy, side), e])
	for c in observation.crashes:
		shooters.append([PoseMath.side_key(int(c.id), side if c.own else enemy, side), c])
	var seen := {}
	var owed := {}
	for entry in shooters:
		var key: int = entry[0]
		var u: Dictionary = entry[1]
		var type: Dictionary = types[u.kind]
		var poses: Array = u.get("weaponPoses", [])
		var members: Array = u.get("memberIds", [])
		seen[key] = true
		var before = _counters.get(key)
		_counters[key] = poses.map(func(p): return p.shots)
		var due = null if gap else _owed.get(key)
		for m in poses.size():
			var shots: float = poses[m].shots
			var rose := 0.0
			if before != null:
				rose = shots - (before[m] if m < before.size() else shots)
			if type.hull != null:
				continue
			if rose > 0.0:
				if not owed.has(key):
					owed[key] = {}
				owed[key][m] = rose
			var left: float = due.get(m, 0.0) if due != null else 0.0
			var mount_index := int(poses[m].mount)
			var kind: String = type.mounts[mount_index].weapon if mount_index < type.mounts.size() else "default"
			for id in members:
				for s in starts.get(int(id), []):
					if left <= 0.0 or s.kind != kind:
						continue
					left -= 1.0
					out.append({"soldier": int(id), "mount": m})
	_owed = owed
	for key in _counters.keys():
		if not seen.has(key):
			_counters.erase(key)
	var flying := {}
	for p in observation.projectiles:
		if p.path.size() >= 2 and p.hit == "none":
			flying[_flight_key(p, p.path[p.path.size() - 1])] = true
	_flying = flying
	return out

static func _flight_key(p: Dictionary, at: Array) -> Array:
	return [p.kind, p.shooterMember, at[0], at[1], at[2]]
