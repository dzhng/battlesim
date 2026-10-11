extends RefCounted

## Reader for the manifest's `effects` and `effectCounts` (the browser's
## `web/src/battle/benchmark/effectManifest.ts`): the effects' tuning as the
## browser read it (`presentation.effects`, each weapon's rows resolved), each
## fielded type's hull and mount muzzles, the prop kinds that smoke, what each
## wreck throws when it cooks off, and what each plate drew. Strict: no
## defaults, no unknown fields where native reads them.

const StrictJson = preload("res://core/strict_json.gd")

const EFFECTS_KEYS := ["presentation", "types", "smokes", "wrecks"]
## Every part of `presentation.effects` the effects read (`EffectPresentation`).
const PRESENTATION_KEYS := ["capacity", "min_px", "tracers", "flashes", "impacts", "impact_scale", "blast_scale", "sparks", "ricochet_sparks", "blast", "cook_off", "dust", "rotor_blur", "damage_smoke", "smoke", "smoke_budget", "cast_wrap", "wind_mps"]
## The tables keyed by round or hit kind, each with its `default` row.
const DEFAULTED := ["tracers", "flashes", "impacts", "impact_scale", "blast_scale"]
const COUNT_KEYS := ["plate", "running", "instances", "lights"]
## The kinds and shapes a plate's counts name (`EFFECT_KINDS`, `SHAPE_NAMES`).
const KINDS := ["tracer", "flash", "impact", "sparks", "blast", "puff", "flame", "trail"]
const SHAPES := ["streak", "glow", "flipbook", "rotor"]

## An empty string when `effects` is a section the effects can follow, or
## the first problem.
static func error(effects: Variant) -> String:
	var shape := StrictJson.exact_keys(effects, EFFECTS_KEYS, "effects")
	if not shape.is_empty():
		return shape
	var p = effects.presentation
	shape = StrictJson.exact_keys(p, PRESENTATION_KEYS, "effects.presentation")
	if not shape.is_empty():
		return shape
	for table in DEFAULTED:
		if typeof(p[table]) != TYPE_DICTIONARY or not p[table].has("default"):
			return "effects.presentation.%s needs a default row" % table
	if not StrictJson.is_integer(p.capacity) or int(p.capacity) <= 0 or not StrictJson.is_vector(p.wind_mps, 2):
		return "effects.presentation needs a capacity and the wind"
	if typeof(p.smoke) != TYPE_DICTIONARY or typeof(p.cook_off) != TYPE_DICTIONARY or typeof(p.cook_off.get("fireballs")) != TYPE_ARRAY:
		return "effects.presentation needs smoke looks and the cook-off's fireballs"
	if typeof(effects.types) != TYPE_DICTIONARY:
		return "effects.types must be an object"
	for kind in effects.types:
		var type = effects.types[kind]
		shape = StrictJson.exact_keys(type, ["half", "muzzles"], "effects type " + kind)
		if not shape.is_empty():
			return shape
		if not (type.half == null or StrictJson.is_vector(type.half, 3)) or typeof(type.muzzles) != TYPE_ARRAY:
			return "effects type %s needs a hull's half extents or null and its muzzles" % kind
		for muzzle in type.muzzles:
			if muzzle == null:
				continue
			shape = StrictJson.exact_keys(muzzle, ["on", "pivot", "ring", "muzzle"], "muzzle of " + kind)
			if not shape.is_empty():
				return shape
			if not ((muzzle.on == null or StrictJson.is_integer(muzzle.on)) and StrictJson.is_vector(muzzle.pivot, 3) and StrictJson.is_vector(muzzle.ring, 3) and StrictJson.is_vector(muzzle.muzzle, 3)):
				return "a muzzle of %s needs its carrier, pivot, ring and muzzle" % kind
	if typeof(effects.smokes) != TYPE_DICTIONARY or not effects.smokes.values().all(func(look): return p.smoke.has(look)):
		return "effects.smokes must name presentation.smoke looks"
	if typeof(effects.wrecks) != TYPE_DICTIONARY:
		return "effects.wrecks must be an object"
	for name in effects.wrecks:
		var wreck = effects.wrecks[name]
		shape = StrictJson.exact_keys(wreck, ["lies", "underside", "debrisTop"], "wreck " + name)
		if not shape.is_empty():
			return shape
		if not ((wreck.lies == null or StrictJson.is_vector(wreck.lies, 3)) and StrictJson.is_number(wreck.underside) and (wreck.debrisTop == null or StrictJson.is_number(wreck.debrisTop))):
			return "wreck %s needs its turret's centre or null, its underside and its debris's top or null" % name
	return ""

## An empty string when `counts` holds one row per plate of whole effects by
## kind, instances by shape and lights.
static func counts_error(counts: Variant) -> String:
	if typeof(counts) != TYPE_ARRAY:
		return "effectCounts must be an array"
	for row in counts:
		var shape := StrictJson.exact_keys(row, COUNT_KEYS, "effect count")
		if shape.is_empty():
			shape = StrictJson.exact_keys(row.running, KINDS, "effect count running")
		if shape.is_empty():
			shape = StrictJson.exact_keys(row.instances, SHAPES, "effect count instances")
		if not shape.is_empty():
			return shape
		var whole := func(v): return StrictJson.is_integer(v) and int(v) >= 0
		if not StrictJson.is_name(row.plate) or not row.running.values().all(whole) or not row.instances.values().all(whole) or not whole.call(row.lights):
			return "effect count %s needs a plate and whole counts" % row.plate
	return ""
