# Frozen: a port of the per-frame presentation `crates/presentation` now owns; no fixes here, deleted with Godot (slice 25).
extends RefCounted

## A side's decoded publication as the effects read it, ported from the
## browser's `effectPublication` (`apps/battle-lab/src/effectFeed.ts`): its
## visible flight and blasts, the shot counters and hulls of every unit the
## side sees (own units and identified enemies, kept apart by `sideKey`), the
## downed airframes still falling (which smoke), and every wreck and part of a
## destroyed building it knows (which smoke, each with its kind's look).
## Positions stay the decode's `[x, y, z]` arrays, read in double precision.

const PoseMath = preload("res://units/pose_math.gd")
const JsText = preload("res://core/js_text.gd")

var _unit_types: Dictionary
var _types: Dictionary
var _smokes: Dictionary
## Each smoke source's key, by `[kind, x, y]`: the text the browser hashes.
var _keys := {}

## `unit_types` are the manifest's `units.types` (a mount's round, whether a
## type flies), `effects` its `effects` (hulls, muzzles, smoke looks).
func _init(unit_types: Dictionary, effects: Dictionary) -> void:
	_unit_types = unit_types
	_types = effects.types
	_smokes = effects.smokes

## What `o`, seen as `side`, publishes for effects (`EffectPublication`).
func publication(o: Dictionary, side: String) -> Dictionary:
	var enemy := "red" if side == "blue" else "blue"
	var segments := []
	for p in o.projectiles:
		segments.append({"path": p.path, "ricochets": p.ricochets, "kind": p.kind, "shooter": p.shooterMember, "hit": p.hit, "normal": p.impactNormal})
	var shooters := []
	for u in o.own:
		shooters.append(_shooter(PoseMath.side_key(int(u.id), side, side), u.kind, u.position, u.yaw, u.memberIds, u.weaponPoses, u.smoking))
	for e in o.identified:
		shooters.append(_shooter(PoseMath.side_key(int(e.id), enemy, side), e.kind, e.position, e.yaw, e.memberIds, e.weaponPoses, e.smoking))
	# A downed airframe smokes all the way down, keyed as it was alive so its
	# trail runs on unbroken.
	for c in o.crashes:
		shooters.append(_shooter(PoseMath.side_key(int(c.id), side if c.own else enemy, side), c.kind, c.position, c.yaw, [], [], true))
	# Every wreck the side knows smokes where the side last saw it, and so does
	# every part of a building it knows destroyed.
	var smokes := []
	for p in o.knownProps:
		var look = null if p.destroyed else _smokes.get(p.kind)
		if look == null:
			continue
		smokes.append({"key": _key(p), "kind": look, "center": [p.center[0], p.center[1], p.baseZ], "yaw": p.yaw, "half": p.half})
	return {"tick": int(o.tick), "segments": segments, "blasts": o.blasts, "shooters": shooters, "smokes": smokes}

## A shooter (`EffectShooter`): each weapon pose's mount as published, the
## round its mount fires (its first weapon) and its muzzle model.
func _shooter(key: int, kind: String, position: Array, yaw: float, members: Array, poses: Array, smoking: bool) -> Dictionary:
	var unit: Dictionary = _unit_types[kind]
	var type: Dictionary = _types[kind]
	var mounts := []
	for p in poses:
		var m := int(p.mount)
		mounts.append({
			"bearing": float(p.bearing),
			"elevation": float(p.elevation),
			"shots": float(p.shots),
			"kind": unit.mounts[m].weapon if m < unit.mounts.size() else "default",
			"muzzle": type.muzzles[m] if m < type.muzzles.size() else null,
		})
	return {"key": key, "position": position, "half": type.half, "airborne": bool(unit.airborne), "smoking": smoking, "yaw": yaw, "members": members, "mounts": mounts}

## `${kind}:${x},${y}`, as the browser writes it.
func _key(p: Dictionary) -> String:
	var at := [p.kind, p.center[0], p.center[1]]
	var key = _keys.get(at)
	if key == null:
		key = "%s:%s,%s" % [p.kind, JsText.number_text(p.center[0]), JsText.number_text(p.center[1])]
		_keys[at] = key
	return key
