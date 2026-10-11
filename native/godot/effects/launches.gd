extends RefCounted

## Which shots a side's publications show, ported from the browser's
## `LaunchTracker` (`packages/battle-renderer/src/effects/launches.ts`), the one
## derivation the muzzle flashes and the pose feed's firing soldiers read. A
## mount's shot counter rising is a shot. A hull's mount fires from its own
## muzzle along its bearing and elevation, on the tick its counter rises. A
## squad's rise goes to the soldiers who start new rounds, one round each,
## matched by kind, in the next publication; a round still flying is no new
## launch. A unit first seen shows no shot until its counter rises once more.
## It also chooses which rounds show as tracers, by their kind's chance.

const JsText = preload("res://core/js_text.gd")
const Mulberry32 = preload("res://core/mulberry32.gd")

## Shot counters by shooter key, as last published.
var _counters := {}
## Where the last publication's still-flying stretches ended,
## `[kind, shooter, x, y, z]`, and whether each was chosen as a tracer.
var _flying := {}
## Squads' rises in the last publication, by shooter key and mount.
var _owed := {}
## This publication's stretches chosen as tracers, by index.
var tracers := {}

func reset() -> void:
	_counters.clear()
	_flying.clear()
	_owed.clear()
	tracers.clear()

## The launches `pub` (an effect publication, `effect_feed.gd`) shows, each
## `{shooter, mount, soldier, x, y, z, dx, dy, dz, kind, hull}`. Call once per
## publication, in order; `gap` says ticks were skipped since the last.
## `chance` gives a round kind's chance to show as a tracer (none: always).
func note(pub: Dictionary, gap: bool, chance := Callable()) -> Array:
	var out := []
	var starts := {}
	for s in pub.segments:
		if s.shooter == null or s.path.size() < 2:
			continue
		if not gap and _flying.has(_flight_key(s, s.path[0])):
			continue
		var shooter := int(s.shooter)
		if not starts.has(shooter):
			starts[shooter] = []
		starts[shooter].append(s)
	var seen := {}
	var owed := {}
	for u in pub.shooters:
		var key: int = u.key
		seen[key] = true
		var before = _counters.get(key)
		_counters[key] = u.mounts.map(func(m): return m.shots)
		var due = null if gap else _owed.get(key)
		for m in u.mounts.size():
			var mount: Dictionary = u.mounts[m]
			var rose := 0.0
			if before != null:
				rose = mount.shots - (before[m] if m < before.size() else mount.shots)
			if u.half != null:
				if rose > 0.0:
					out.append(_hull_launch(u, m))
				continue
			if rose > 0.0:
				if not owed.has(key):
					owed[key] = {}
				owed[key][m] = rose
			var left: float = due.get(m, 0.0) if due != null else 0.0
			for id in u.members:
				for s in starts.get(int(id), []):
					if left <= 0.0 or s.kind != mount.kind:
						continue
					left -= 1.0
					var a: Array = s.path[0]
					var b: Array = s.path[1]
					var length := sqrt((b[0] - a[0]) * (b[0] - a[0]) + (b[1] - a[1]) * (b[1] - a[1]) + (b[2] - a[2]) * (b[2] - a[2]))
					if length == 0.0:
						length = 1.0
					out.append({"shooter": key, "mount": m, "soldier": int(id), "x": a[0], "y": a[1], "z": a[2], "dx": (b[0] - a[0]) / length, "dy": (b[1] - a[1]) / length, "dz": (b[2] - a[2]) / length, "kind": s.kind, "hull": false})
	_owed = owed
	for key in _counters.keys():
		if not seen.has(key):
			_counters.erase(key)
	var flying := {}
	tracers.clear()
	for index in pub.segments.size():
		var s: Dictionary = pub.segments[index]
		if s.path.size() < 2:
			continue
		var probability: float = chance.call(s.kind) if chance.is_valid() else 1.0
		# The authority publishes in round order: a queue keeps apart the
		# choices of rounds whose paths share an end.
		var previous = null
		if not gap:
			var queue = _flying.get(_flight_key(s, s.path[0]))
			if queue != null and not queue.is_empty():
				previous = queue.pop_front()
		var visible: bool
		if previous != null:
			visible = previous
		elif probability >= 1.0:
			visible = true
		else:
			var start: Array = s.path[0]
			var text := "%d:%d:%s:%s:%s,%s,%s" % [int(pub.tick), index, s.kind, JsText.number_text(s.shooter), JsText.number_text(start[0]), JsText.number_text(start[1]), JsText.number_text(start[2])]
			visible = Mulberry32.sample(Mulberry32.create(JsText.hash_string(text))) < probability
		if visible:
			tracers[index] = true
		if s.hit == "none":
			var end_key := _flight_key(s, s.path[s.path.size() - 1])
			if not flying.has(end_key):
				flying[end_key] = []
			flying[end_key].append(visible)
	_flying = flying
	return out

## A hull's shot from mount `m`: from its own muzzle, along its aim. Its pivot
## swings with its carrier round the carrier's ring (`muzzleOffset`).
static func _hull_launch(u: Dictionary, m: int) -> Dictionary:
	var mount: Dictionary = u.mounts[m]
	var at := [0.0, 0.0, 0.0]
	if mount.muzzle != null:
		var carried: float = u.yaw if mount.muzzle.on == null else u.mounts[int(mount.muzzle.on)].bearing
		at = muzzle_offset(mount.muzzle, u.yaw, carried, mount.bearing)
	var ce := cos(mount.elevation)
	return {
		"shooter": u.key, "mount": m, "soldier": null,
		"x": u.position[0] + at[0], "y": u.position[1] + at[1], "z": u.position[2] + at[2],
		"dx": ce * cos(mount.bearing), "dy": ce * sin(mount.bearing), "dz": sin(mount.elevation),
		"kind": mount.kind, "hull": true,
	}

## A mount's muzzle from the unit's position (`muzzleOffset`), for a hull
## heading `yaw` with the mount's carrier along `carried` and the mount along
## `bearing` (world radians).
static func muzzle_offset(m: Dictionary, yaw: float, carried: float, bearing: float) -> Array:
	var muzzle := _turned(m.muzzle, [0.0, 0.0, 0.0], bearing)
	var pivot := _turned(m.pivot, m.ring, carried)
	var ring := _turned(m.ring, [0.0, 0.0, 0.0], yaw)
	return [pivot[0] - m.ring[0] + ring[0] + muzzle[0], pivot[1] - m.ring[1] + ring[1] + muzzle[1], pivot[2] - m.ring[2] + ring[2] + muzzle[2]]

## `a` turned by `angle` about +z through `about` (`vec3.rotateZ`).
static func _turned(a: Array, about: Array, angle: float) -> Array:
	var x: float = a[0] - about[0]
	var y: float = a[1] - about[1]
	return [x * cos(angle) - y * sin(angle) + about[0], x * sin(angle) + y * cos(angle) + about[1], a[2]]

static func _flight_key(s: Dictionary, at: Array) -> Array:
	return [s.kind, s.shooter, at[0], at[1], at[2]]
