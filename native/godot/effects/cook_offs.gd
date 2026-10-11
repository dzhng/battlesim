extends RefCounted

## Brew-ups a side watched, ported from the browser's `CookOffWatch`
## (`apps/battle-lab/src/cookOffs.ts`): a hull it saw that is gone, and a wreck
## of its type near where it stood. A side that saw the hull alive within a
## second of learning its wreck saw it die; a wreck found later was no one's
## to watch. An airframe shot down falls first and never brews up. Each
## brew-up becomes the effects' cook-off (`effectCookOff`): the hull's base,
## its height, and where its thrown turret lands, read from the wreck the
## props fit (`world/prop_layer.gd` `fit`) and what that wreck throws
## (`effects.wrecks`).

const Coords = preload("res://core/coords.gd")
const PropLayer = preload("res://world/prop_layer.gd")

## How far beyond its own half length a wreck may lie from where its hull was
## last seen: a vehicle killed at speed rolls a few metres on (`ROLL_REACH_M`).
const ROLL_REACH_M := 6.0

var _unit_types: Dictionary
var _types: Dictionary
var _wrecks: Dictionary
var _props: Dictionary
var _window: int
## Every hull seen within the window, where it was last seen, by `who:id`.
var _hulls := {}
var _known := {}
var _last_tick := -1

## `unit_types` the manifest's `units.types`, `effects` its effects, `props`
## its props (the fit and its looks); `window` in ticks (the tick rate: a
## second).
func _init(unit_types: Dictionary, effects: Dictionary, props: Dictionary, window: int) -> void:
	_unit_types = unit_types
	_types = effects.types
	_wrecks = effects.wrecks
	_props = props
	_window = window

## The cook-offs (`EffectCookOff`) the side's next publication shows: hulls
## it watched brew up since the last. An earlier tick starts over.
func note(o: Dictionary) -> Array:
	var tick := int(o.tick)
	if tick < _last_tick:
		_hulls.clear()
		_known.clear()
		_last_tick = -1
	var first := _last_tick < 0
	var seen := {}
	for group in [["own", o.own], ["enemy", o.identified]]:
		for u in group[1]:
			var half = _types[u.kind].half
			if _unit_types[u.kind].airborne or half == null:
				continue
			seen["%s:%d" % [group[0], int(u.id)]] = {"kind": u.kind, "tick": tick, "x": float(u.position[0]), "y": float(u.position[1]), "reach": float(half[0])}
	for key in _hulls.keys():
		if seen.has(key) or tick - int(_hulls[key].tick) > _window:
			_hulls.erase(key)
	var out := []
	for p in o.knownProps:
		var id := int(p.id)
		if _known.has(id):
			continue
		_known[id] = true
		if not first and not p.destroyed and p.wreckOf != null and _watched(p):
			out.append(_cook_off(p))
	_hulls.merge(seen, true)
	_last_tick = tick
	return out

## Whether a hull the side lost from sight lately, of the type wreck `p` was,
## stood where it lies.
func _watched(p: Dictionary) -> bool:
	for h in _hulls.values():
		var dx: float = float(p.center[0]) - h.x
		var dy: float = float(p.center[1]) - h.y
		if h.kind == p.wreckOf and sqrt(dx * dx + dy * dy) <= h.reach + ROLL_REACH_M:
			return true
	return false

## Wreck `p` as the effects take it: its hull, and where its turret lands
## if the wreck the props fit throws one.
func _cook_off(p: Dictionary) -> Dictionary:
	var landing = null
	var fitted := PropLayer.fit(_props.fit, p)
	if not fitted.is_empty():
		var wreck = _wrecks.get(_props.looks[int(fitted[0][0])].appearance)
		if wreck != null and wreck.lies != null:
			# The piece's underside under its centre, placed as the wreck is.
			var local := Coords.sim_to_godot(Vector3(wreck.lies[0], wreck.lies[1], wreck.underside))
			var at := Coords.godot_to_sim((fitted[0][1] as Transform3D) * local)
			landing = [at.x, at.y, at.z]
	return {"center": [p.center[0], p.center[1], p.baseZ], "height": 2.0 * float(p.half[2]), "landing": landing}
