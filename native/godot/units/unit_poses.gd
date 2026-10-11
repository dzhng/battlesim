extends RefCounted

## A battle's units posed frame record by frame record, as the browser's
## battle session poses them: the record's publication blended from the one
## before (`tick_interpolator.gd`), read as a feed (`pose_feed.gd`) and posed
## (`pose_driver.gd`). Records must be stepped in order: poses carry state
## from frame to frame (a walker's phase, a gun's eased elevation, a fall).
##
## It also resolves which appearance draws each posed unit, as the browser's
## `AppearanceCatalog.resolve` picks from the sets the manifest lists, and
## counts what a frame poses by kind (`modelCounts`).

const TickInterpolator = preload("res://units/tick_interpolator.gd")
const PoseFeed = preload("res://units/pose_feed.gd")
const PoseDriver = preload("res://units/pose_driver.gd")

## The manifest's `units`.
var units: Dictionary
var _samples: Array
var _tick_hz: float
var _feed: PoseFeed
var _driver: PoseDriver

func _init(manifest_units: Dictionary, samples: Array, side: String, tick_hz: float) -> void:
	units = manifest_units
	_samples = samples
	_tick_hz = tick_hz
	_feed = PoseFeed.new(side, units.types)
	_driver = PoseDriver.new(units.pose, float(units.leanHold), units.types, clip_facts)

## The pose frame (`PoseFrame`) for a record, or an empty dictionary before
## the first publication.
func step(record: Dictionary) -> Dictionary:
	var sample := TickInterpolator.frame(_samples, TickInterpolator.latest_at(_samples, int(record.tick)), float(record.simTime), _tick_hz)
	if sample.is_empty():
		return {}
	return _driver.update(_feed.frame(sample))

## The appearance drawing soldier `id` in slot `slot` of a unit of type
## `kind` on `side` (a hull's model whatever the id), or "" when none is
## installed: `AppearanceCatalog.resolve` over the manifest's sets.
func resolve(kind: String, side: String, id: int, slot: int, operator_mount: Variant, active_mount: Variant) -> String:
	var draws: Dictionary = units.types[kind].draws[side]
	var names: Array = draws.hull
	if names.is_empty():
		var equipment = draws.operated[int(operator_mount)] if operator_mount != null and int(operator_mount) < draws.operated.size() else null
		var operated: Array = [] if equipment == null else (equipment.active if active_mount != null and int(active_mount) == int(operator_mount) else equipment.carried)
		names = operated if not operated.is_empty() else (draws.slots[slot] if slot < draws.slots.size() else [])
	if names.is_empty():
		return ""
	return names[posmod(id, names.size())]

## What the driver needs of a clip a soldier's appearance plays
## (`ClipFacts`), or null where his rig lacks it.
func clip_facts(kind: String, clip: String, soldier: Dictionary) -> Variant:
	var name := resolve(kind, "blue", int(soldier.get("soldier", 0)), int(soldier.get("slot", 0)), soldier.get("operatorMount"), soldier.get("activeMount"))
	var appearance = units.appearances.get(name)
	if appearance == null or appearance.kind != "skinned" or not appearance.clips.has(clip):
		return null
	return appearance.clips[clip]

## How many models of each kind `frame` poses, counting only those with an
## appearance (`modelCounts`).
func counts(frame: Dictionary) -> Dictionary:
	var out := {"soldiers": 0, "vehicles": 0, "aircraft": 0, "corpses": 0, "fading": 0}
	if frame.is_empty():
		return out
	for s in frame.soldiers:
		if not resolve(s.kind, s.side, int(s.soldier), int(s.slot), s.operatorMount, s.activeMount).is_empty():
			out.soldiers += 1
	for v in frame.vehicles:
		if not resolve(v.kind, v.side, int(v.unit), 0, null, null).is_empty():
			out["aircraft" if v.airborne else "vehicles"] += 1
	for c in frame.corpses:
		if not resolve(c.kind, c.side, int(c.soldier), int(c.slot), c.operatorMount, null).is_empty():
			out.corpses += 1
	for f in frame.fading:
		var c: Dictionary = f.corpse
		if not resolve(c.kind, c.side, int(c.soldier), int(c.slot), c.operatorMount, null).is_empty():
			out.fading += 1
	return out
