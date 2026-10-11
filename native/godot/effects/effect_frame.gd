extends RefCounted

## Combat effects, ported line for line from the browser's one owner
## (`packages/battle-renderer/src/effects/effectFrame.ts`). Each publication
## (`effect_feed.gd`) becomes short-lived effects, and each frame they become
## the effect pass's instances at the presentation clock, the frame record's
## `simTime`: tracers and their smoke trails, muzzle flashes on the drawn
## muzzles, impact puffs, ricochet and impact sparks, blasts with their dirt and
## smoke, dust behind moving hulls, smoke behind smoking hulls, flames and smoke
## over every known wreck and ruin, cook-offs and rotor blurs. Every effect has
## a published cause and a seed: per tick `mulberry32((tick·2654435761)>>>0)`,
## consumed in publication order, and the browser's `hashString` keys for
## dust, smoke trails and smoke sources, so native draws the same particles.
## Instances keep the browser's layout (16 floats, sim space) and the lights
## its cast-light candidates (8 floats).

const Mulberry32 = preload("res://core/mulberry32.gd")
const JsText = preload("res://core/js_text.gd")
const Launches = preload("res://effects/launches.gd")

## Floats per instance: a, b, colour, misc (vec4 each).
const FLOATS := 16
## Instance shapes (misc.x), in `SHAPE` order, and the kinds of effect.
const STREAK := 0
const GLOW := 1
const FLIPBOOK := 2
const ROTOR := 3
const SHAPES := ["streak", "glow", "flipbook", "rotor"]
const KINDS := ["tracer", "flash", "impact", "sparks", "blast", "puff", "flame", "trail"]
## Flipbook layers and their frames (`LAYER`, `LAYER_FRAMES`).
const FIRE := 0
const DUST := 1
const LAYER_FRAMES := [25, 64]
## Cast-light candidates a frame may offer, and floats per candidate:
## x, y, z, radius, r, g, b (colour × intensity), unused.
const LIGHT_CANDIDATES := 1024
const LIGHT_FLOATS := 8

const TRACER := 0
const FLASH := 1
const IMPACT := 2
const SPARKS := 3
const BLAST := 4
const PUFF := 5
const FLAME := 6
const TRAIL := 7

const SURFACE_LIFT_M := 0.08
const TRACER_TAIL_MAX_S := 1.0
const PUFF_TURN_FRAMES := 24.0
const PUFF_FADE_IN := 4.0
const RIBBON_WIDTH := 0.7
const GRAVITY := 9.81
const GOLDEN := 2654435761

## One effect: every field present from creation (one shape for all kinds).
class Effect:
	var type := TRACER
	var start := 0.0
	var end := 0.0
	var span := 1.0
	var p := [0.0, 0.0, 0.0]
	var n := [0.0, 0.0, 0.0]
	var size := 0.0
	var rotation := 0.0
	## A tracer's path flat (x, y, z), its cumulative lengths and total.
	var path := PackedFloat64Array()
	var cum := PackedFloat64Array()
	var length := 0.0
	var sparks := PackedFloat64Array()
	var tracer = null
	var tracer_visible := true
	var flash = null
	var shooter := 0
	var mount := 0
	var soldier = null
	var impact = null
	var puff = null
	var trail = null
	var flame = null
	var cause := ""
	var scale := 1.0
	var alpha := 1.0
	var frame := 0.0

## What a frame draws: `count` instances in `data`, those dropped past the
## capacity, and its cast-light candidates.
class Batch:
	var data := PackedFloat32Array()
	var count := 0
	var dropped := 0
	var lights := PackedFloat32Array()
	var light_count := 0
	var light_dropped := 0
	var causes: Array[String] = []
	var wrap := 0.0

	func _init(capacity: int) -> void:
		data.resize(capacity * FLOATS)
		lights.resize(LIGHT_CANDIDATES * LIGHT_FLOATS)
		causes.resize(LIGHT_CANDIDATES)

	func put(values: Array) -> void:
		if count * FLOATS >= data.size():
			dropped += 1
			return
		var o := count * FLOATS
		for k in FLOATS:
			data[o + k] = values[k]
		count += 1

	## `offerCastLight`: a light at (x, y, z) of `radius`, `color` × `intensity`.
	func offer(x: float, y: float, z: float, radius: float, color: Array, intensity: float, cause: String) -> void:
		if not (intensity > 0.0 and radius > 0.0):
			return
		if light_count * LIGHT_FLOATS >= lights.size():
			light_dropped += 1
			return
		var o := light_count * LIGHT_FLOATS
		var values := [x, y, z, radius, color[0] * intensity, color[1] * intensity, color[2] * intensity, 0.0]
		for k in LIGHT_FLOATS:
			lights[o + k] = values[k]
		causes[light_count] = cause
		light_count += 1

var _effects: Array[Effect] = []
## Smoke sources the side knows: `{key, cause, style, seed, start, x, y, top,
## yaw, hx, hy, scale, alpha, emitted, known}`.
var _sources: Array[Dictionary] = []
var _source_index := {}
var _noted := 0.0
var _keep := 1.0
var _movers := {}
var _smokers := {}
var _dt: float
var _p: Dictionary
var _last_tick := -1
var _launches := Launches.new()

## `presentation` is the manifest's `effects.presentation`.
func _init(tick_hz: float, presentation: Dictionary) -> void:
	_dt = 1.0 / tick_hz
	_p = presentation

func capacity() -> int:
	return int(_p.capacity)

## Forget everything (a new battle).
func reset() -> void:
	_effects.clear()
	_sources.clear()
	_source_index.clear()
	_movers.clear()
	_smokers.clear()
	_launches.reset()
	_last_tick = -1
	_noted = 0.0
	_keep = 1.0

## Take one publication (`note`): every effect it causes starts between the
## previous tick's presentation time and its own. The same tick again is
## ignored; an earlier one starts over.
func note(pub: Dictionary) -> void:
	var tick: int = pub.tick
	if tick == _last_tick:
		return
	if tick < _last_tick:
		reset()
	var gap := _last_tick >= 0 and tick != _last_tick + 1
	_last_tick = tick
	var t0 := (tick - 1) * _dt
	var t1 := tick * _dt
	var rng := Mulberry32.create((tick * GOLDEN) & 0xffffffff)
	var tracers: Dictionary = _p.tracers
	for l in _launches.note(pub, gap, func(kind): return float(_pick(tracers, kind).get("chance", 1.0))):
		_add_flash(t0, l, rng)
	for index in pub.segments.size():
		var s: Dictionary = pub.segments[index]
		if s.path.size() < 2:
			continue
		var tracer: bool = _launches.tracers.has(index)
		var e := _add_tracer(t0, s, rng, tracer)
		# A ricochet always sparks; an impact sparks only off a tracer round.
		for r in s.ricochets:
			var point := int(r.point)
			var at: float = t0 + (_dt * e.cum[point]) / maxf(e.length, 1e-6)
			var next: Array = s.path[mini(point + 1, s.path.size() - 1)]
			var p: Array = s.path[point]
			var dir := _normalized([next[0] - p[0], next[1] - p[1], next[2] - p[2]])
			_add_sparks(at, p, r.normal, dir, int(_p.ricochet_sparks), rng)
		if s.hit != "none":
			_add_impact(t1, s.path[s.path.size() - 1], s.normal, s.hit, s.kind, tracer, rng)
	for b in pub.blasts:
		_add_blast(t1, b, rng)
	for c in pub.get("cookOffs", []):
		_add_cook_off(t0, c, rng)
	_note_movers(pub, gap, t0)
	_note_smokers(pub, gap, t0, t1)
	_note_sources(pub, t0, t1)

## Dust behind every hull the side sees move on the ground.
func _note_movers(pub: Dictionary, gap: bool, t0: float) -> void:
	var style: Dictionary = _p.dust
	for m in _movers.values():
		m.seen = false
	for u in pub.shooters:
		if u.half == null or u.airborne:
			continue
		var x: float = u.position[0]
		var y: float = u.position[1]
		var z: float = u.position[2]
		var m = _movers.get(u.key)
		if m == null:
			_movers[u.key] = {"x": x, "y": y, "z": z, "carry": 0.0, "seen": true}
			continue
		m.seen = true
		var dx: float = x - m.x
		var dy: float = y - m.y
		var dist := sqrt(dx * dx + dy * dy)
		var speed := dist / _dt
		if not gap and speed >= float(style.min_speed_mps) and dist > 1e-6:
			var fx := dx / dist
			var fy := dy / dist
			var hl: float = u.half[0]
			var hw: float = u.half[1]
			var thick := minf(1.0, speed / float(style.full_speed_mps))
			var along: float = float(style.spacing_m) - m.carry
			while along <= dist:
				var u01 := along / dist
				var px: float = m.x + dx * u01
				var py: float = m.y + dy * u01
				var pz: float = m.z + (z - m.z) * u01
				var at := t0 + _dt * u01
				var rng := Mulberry32.create(JsText.hash_string("%d:%d,%d" % [u.key, _js_round(px * 100.0), _js_round(py * 100.0)]))
				for side: float in [-1.0, 1.0]:
					var tx := px - fx * hl * 1.1 - fy * hw * 0.75 * side
					var ty := py - fy * hl * 1.1 + fx * hw * 0.75 * side
					_add_puff(at, tx, ty, pz + 0.3, style, 1.0, thick, rng)
				along += float(style.spacing_m)
			m.carry = dist - (along - float(style.spacing_m))
		else:
			m.carry = 0.0
		m.x = x
		m.y = y
		m.z = z
	for key in _movers.keys():
		if not _movers[key].seen:
			_movers.erase(key)

## Smoke behind every smoking hull the side sees: a puff every 1/`rate_hz` s,
## each seeded by its hull and index.
func _note_smokers(pub: Dictionary, gap: bool, t0: float, t1: float) -> void:
	var style: Dictionary = _p.damage_smoke
	var rate: float = style.rate_hz
	for m in _smokers.values():
		m.seen = false
	for u in pub.shooters:
		if not u.smoking:
			continue
		var top: float = u.position[2] + (u.half[2] * 2.0 if u.half != null else 0.0)
		var m = _smokers.get(u.key)
		if m == null or gap:
			m = {"x": u.position[0], "y": u.position[1], "z": top, "carry": 0.0, "seen": true}
			_smokers[u.key] = m
		m.seen = true
		var k := ceili(t0 * rate)
		while k / rate < t1:
			var at := k / rate
			var f := (at - t0) / (t1 - t0)
			var seed := (JsText.hash_string("smoke:%d" % u.key) ^ Mulberry32.imul(k + 1, GOLDEN)) & 0xffffffff
			_add_puff(at, m.x + (u.position[0] - m.x) * f, m.y + (u.position[1] - m.y) * f, m.z + (top - m.z) * f, style, 1.0, 1.0, Mulberry32.create(seed))
			k += 1
		m.x = u.position[0]
		m.y = u.position[1]
		m.z = top
	for key in _smokers.keys():
		if not _smokers[key].seen:
			_smokers.erase(key)

## Smoke sources: each known one burns from the tick it is first known, then
## smoulders, then is out; every puff and flame has its own time and seed.
func _note_sources(pub: Dictionary, t0: float, t1: float) -> void:
	for src in _sources:
		src.known = false
	for k in pub.smokes:
		var src = _source_index.get(k.key)
		var style = _p.smoke.get(k.kind)
		if style == null:
			continue
		if src == null:
			var seed := JsText.hash_string(k.key)
			src = {
				"key": k.key, "cause": "fire:" + k.kind, "style": style, "seed": seed, "start": t0,
				"x": float(k.center[0]), "y": float(k.center[1]), "top": float(k.center[2]) + float(k.half[2]) * 2.0,
				"yaw": float(k.yaw), "hx": float(k.half[0]), "hy": float(k.half[1]),
				"scale": 0.75 + 0.5 * (float(seed & 0xffff) / 0xffff),
				"alpha": 0.75 + 0.25 * (float(seed >> 16) / 0xffff),
				"emitted": [0, 0, 0], "known": true,
			}
			_sources.append(src)
			_source_index[k.key] = src
		src.known = true
	_noted = t1
	var kept: Array[Dictionary] = []
	var demand := 0.0
	for src in _sources:
		if not src.known:
			_source_index.erase(src.key)
			continue
		kept.append(src)
		demand += _steady_instances(src.style, t1 - src.start)
	_sources = kept
	_keep = minf(1.0, float(_p.smoke_budget) / maxf(demand, 1.0))
	for src in _sources:
		var st: Dictionary = src.style
		var burn_end: float = src.start + float(st.burn_s)
		_emit(src, 0, src.start, burn_end, float(st.smoke_hz), float(st.smoke.life_s), t1)
		_emit(src, 1, burn_end, burn_end + float(st.smoulder_s), float(st.smoulder_hz), float(st.smoulder.life_s), t1)
		_emit(src, 2, src.start, burn_end, float(st.flame.rate_hz), float(st.flame.life_s), t1)

## Stream `stream` of `src`: a birth every 1/`hz` s from `from` to `to`, up
## to `t1`, skipping any already dead.
func _emit(src: Dictionary, stream: int, from: float, to: float, hz: float, life: float, t1: float) -> void:
	if hz <= 0.0:
		return
	var last := ceili((minf(to, t1) - from) * hz) - 1
	var first := maxi(int(src.emitted[stream]), ceili((t1 - life - from) * hz))
	var st: Dictionary = src.style
	var c := cos(src.yaw)
	var s := sin(src.yaw)
	for k in range(first, last + 1):
		var at := from + k / hz
		var rng := Mulberry32.create((int(src.seed) ^ Mulberry32.imul(k + 1, GOLDEN) ^ ((stream << 29) & 0xffffffff)) & 0xffffffff)
		if Mulberry32.sample(rng) >= _keep:
			continue
		var u := (Mulberry32.sample(rng) * 2.0 - 1.0) * 0.6
		var v := (Mulberry32.sample(rng) * 2.0 - 1.0) * 0.6
		var x: float = src.x + u * src.hx * c - v * src.hy * s
		var y: float = src.y + u * src.hx * s + v * src.hy * c
		if stream == 2:
			var left := (to - at) / (to - from)
			_add_flame(at, x, y, src.top - 0.3, st.flame, minf(1.0, left * 3.0), rng)
		else:
			_add_puff(at, x, y, src.top, st.smoke if stream == 0 else st.smoulder, src.scale, src.alpha, rng)
	src.emitted[stream] = maxi(int(src.emitted[stream]), last + 1)

## Every running effect at presentation time `clock` into `batch`; spent
## effects are dropped. `muzzle` (shooter, mount, soldier) answers where a
## shot's muzzle is drawn, or null; `rotors` lists the drawn rotors
## (`drawn_models.gd`). Either may be invalid: nothing drawn.
func build(clock: float, batch: Batch, muzzle := Callable(), rotors := Callable()) -> Batch:
	batch.count = 0
	batch.dropped = 0
	batch.light_count = 0
	batch.light_dropped = 0
	batch.wrap = float(_p.cast_wrap)
	if rotors.is_valid():
		_draw_rotors(batch, rotors.call())
	for e in _effects:
		if e.type == TRAIL and e.start <= clock and clock < e.end:
			_draw_trail(e, clock - e.start, batch)
	var kept: Array[Effect] = []
	for e in _effects:
		if e.end <= clock:
			continue
		kept.append(e)
		if clock < e.start:
			continue
		var age := clock - e.start
		match e.type:
			TRACER:
				_draw_tracer(e, age, batch)
			FLASH:
				_draw_flash(e, age, batch, muzzle)
			IMPACT:
				_draw_impact(e, age, batch)
			SPARKS:
				_draw_sparks(e, age, batch, _p.sparks.color, float(_p.sparks.intensity))
			BLAST:
				_draw_blast(e, age, batch)
			PUFF:
				_draw_puff(e, age, batch)
			FLAME:
				_draw_flame(e, age, batch)
	_effects = kept
	for src in _sources:
		_draw_fire_light(src, clock, batch)
	return batch

## What the last `build`, at `clock` into `batch`, drew (`EffectFrame.counts`):
## running effects by kind, instances by shape, and lights.
func counts(clock: float, batch: Batch) -> Dictionary:
	var running := {}
	for kind in KINDS:
		running[kind] = 0
	for e in _effects:
		if e.start <= clock:
			running[KINDS[e.type]] += 1
	var instances := {}
	for shape in SHAPES:
		instances[shape] = 0
	for i in batch.count:
		instances[SHAPES[int(batch.data[i * FLOATS + 12])]] += 1
	return {"running": running, "instances": instances, "lights": batch.light_count}

# ---- Creation (per publication). ----

func _push(type: int, start: float, end: float) -> Effect:
	var e := Effect.new()
	e.type = type
	e.start = start
	e.end = end
	e.span = end - start
	_effects.append(e)
	return e

func _add_tracer(t0: float, s: Dictionary, rng: PackedInt64Array, visible: bool) -> Effect:
	var style: Dictionary = _pick(_p.tracers, s.kind)
	var e := _push(TRACER, t0, t0)
	e.tracer = style
	e.tracer_visible = visible
	e.cause = "round:" + s.kind
	var total := 0.0
	for i in s.path.size():
		var q: Array = s.path[i]
		if i > 0:
			var r: Array = s.path[i - 1]
			total += _hypot3(q[0] - r[0], q[1] - r[1], q[2] - r[2])
		e.path.append_array([q[0], q[1], q[2]])
		e.cum.append(total)
	e.length = total
	e.span = _dt
	var tail: Array = style.tail_m
	e.size = minf(float(tail[1]), maxf(float(tail[0]), (total / _dt) * float(style.tail_s)))
	e.end = t0 + minf(_dt * (1.0 + e.size / total), _dt + TRACER_TAIL_MAX_S) if total > 1e-6 else t0
	if visible and style.has("smoke") and total > 1e-6:
		_add_trail(t0, e, style.smoke, rng)
	return e

## A round's smoke trail: its ribbon, and a puff every `spacing_m` on a
## lattice along the stretch's heading.
func _add_trail(t0: float, e: Effect, style: Dictionary, rng: PackedInt64Array) -> void:
	var ribbon := _push(TRAIL, t0, t0 + _dt + float(style.life_s))
	ribbon.trail = style
	ribbon.path = e.path.duplicate()
	ribbon.cum = e.cum.duplicate()
	ribbon.length = e.length
	var path := e.path
	var spacing: float = style.spacing_m
	var first := maxf(e.cum[1], 1e-6)
	var along := (path[0] * (path[3] - path[0]) + path[1] * (path[4] - path[1]) + path[2] * (path[5] - path[2])) / first
	var past := fposmod(along, spacing)
	var at := fmod(spacing - past, spacing)
	while at < e.length:
		var p := _point_along(e, at)
		_add_puff(t0 + (_dt * at) / e.length, p[0], p[1], p[2], style, 1.0, 1.0, rng)
		at += spacing

func _add_flash(t0: float, l: Dictionary, rng: PackedInt64Array) -> void:
	var style: Dictionary = _pick(_p.flashes, l.kind)
	var last := maxf(float(style.duration_s), maxf(float(style.fireball_s) if float(style.fireball_m) > 0.0 else 0.0, float(style.cast.duration_s) if style.has("cast") else 0.0))
	var e := _push(FLASH, t0, t0 + last)
	e.flash = style
	e.cause = "flash:" + l.kind
	e.shooter = l.shooter
	e.mount = l.mount
	e.soldier = l.soldier
	e.p = [l.x, l.y, l.z]
	e.n = [l.dx, l.dy, l.dz]
	e.rotation = Mulberry32.sample(rng) * PI * 2.0

func _add_impact(at: float, point: Array, normal, hit: String, kind: String, tracer: bool, rng: PackedInt64Array) -> void:
	var style: Dictionary = _pick(_p.impacts, hit)
	var scale = style.get("round_scale", {}).get(kind)
	var e := _push(IMPACT, at, at + float(style.duration_s) * (float(scale.duration) if scale != null else 1.0))
	e.impact = style
	e.tracer_visible = tracer
	e.cause = "impact:" + hit
	e.p = [point[0], point[1], point[2]]
	e.n = [normal[0], normal[1], normal[2]] if normal != null else [0.0, 0.0, 1.0]
	e.size = float(style.size_m) * (float(scale.size) if scale != null else float(_pick(_p.impact_scale, kind)))
	e.rotation = Mulberry32.sample(rng) * PI * 2.0
	e.path.append(Mulberry32.sample(rng))
	if tracer and float(style.sparks) > 0.0:
		_add_sparks(at, point, e.n, e.n, int(style.sparks), rng)

## Sparks from `point`, thrown along `dir` and off the face `normal`.
func _add_sparks(at: float, point: Array, normal: Array, dir: Array, count: int, rng: PackedInt64Array, scale := 1.0) -> Effect:
	var style: Dictionary = _p.sparks
	var e := _push(SPARKS, at, at + float(style.duration_s) * sqrt(scale))
	e.size = scale
	e.p = [point[0] + normal[0] * SURFACE_LIFT_M, point[1] + normal[1] * SURFACE_LIFT_M, point[2] + normal[2] * SURFACE_LIFT_M]
	for k in count:
		var r := [Mulberry32.sample(rng) * 2.0 - 1.0, Mulberry32.sample(rng) * 2.0 - 1.0, Mulberry32.sample(rng) * 2.0 - 1.0]
		var vx: float = dir[0] * 0.8 + normal[0] * 0.6 + r[0] * 0.6
		var vy: float = dir[1] * 0.8 + normal[1] * 0.6 + r[1] * 0.6
		var vz: float = dir[2] * 0.8 + normal[2] * 0.6 + r[2] * 0.6
		var length := _hypot3(vx, vy, vz)
		if length == 0.0:
			length = 1.0
		var speed := float(style.speed_mps) * scale * (0.4 + 0.6 * Mulberry32.sample(rng))
		e.sparks.append_array([(vx / length) * speed, (vy / length) * speed, (vz / length) * speed])
	return e

## A blast's fireball, sparks (a share `sparks` of its style's) and dirt and smoke.
func _add_blast(at: float, b: Dictionary, rng: PackedInt64Array, sparks := 1.0) -> void:
	var style: Dictionary = _p.blast
	var e := _push(BLAST, at, at + float(style.duration_s))
	e.p = [b.point[0], b.point[1], b.point[2]]
	e.size = float(b.radius) * float(style.size_per_radius) * float(_pick(_p.blast_scale, b.kind))
	var share := e.size / float(style.reference_size_m)
	e.rotation = Mulberry32.sample(rng) * PI * 2.0
	var up := [0.0, 0.0, 1.0]
	var fewer := minf(1.0, share)
	var count := _js_round(float(style.sparks) * sparks * fewer)
	if count != 0:
		_add_sparks(at, b.point, up, up, count, rng, float(style.spark_scale))
	var scale := sqrt(share)
	for burst in [style.plume, style.smoke]:
		var n := _js_round(float(burst.count) * fewer)
		for k in n:
			var r := e.size * 0.3 * sqrt(Mulberry32.sample(rng))
			var a := Mulberry32.sample(rng) * PI * 2.0
			_add_puff(at + float(burst.over_s) * (k / maxf(1.0, float(burst.count) - 1.0)), b.point[0] + cos(a) * r, b.point[1] + sin(a) * r, b.point[2] + e.size * 0.2, burst, scale, 1.0, rng)

## A hull cooking off from `at`: each fireball out of its ring, then the
## sparks and dust its thrown turret raises where it lands.
func _add_cook_off(at: float, c: Dictionary, rng: PackedInt64Array) -> void:
	var feel: Dictionary = _p.cook_off
	var top: float = c.center[2] + c.height
	for f in feel.fireballs:
		_add_blast(at + float(feel.delay_s) + float(f.after_s), {"point": [c.center[0], c.center[1], top + float(f.up_m)], "radius": f.radius_m, "kind": "cook_off"}, rng, float(f.sparks))
	if c.landing == null:
		return
	var plume: Dictionary = _p.blast.plume
	var landed := at + impact_after(feel)
	var up := [0.0, 0.0, 1.0]
	if int(feel.landing_sparks) != 0:
		_add_sparks(landed, c.landing, up, up, int(feel.landing_sparks), rng)
	var dust: float = feel.landing_dust
	var k := 0
	while dust > 0.0 and k < int(plume.count):
		var r := dust * sqrt(Mulberry32.sample(rng))
		var a := Mulberry32.sample(rng) * PI * 2.0
		_add_puff(landed + float(plume.over_s) * 0.5 * (k / maxf(1.0, float(plume.count) - 1.0)), c.landing[0] + cos(a) * r, c.landing[1] + sin(a) * r, c.landing[2], plume, dust, 1.0, rng)
		k += 1

## A puff born at `at` at (x, y, z): its own drift, spin and first frame.
func _add_puff(at: float, x: float, y: float, z: float, style: Dictionary, scale: float, alpha: float, rng: PackedInt64Array) -> void:
	var e := _push(PUFF, at, at + float(style.life_s))
	e.puff = style
	e.scale = scale
	e.alpha = alpha
	e.p = [x, y, z]
	var a := Mulberry32.sample(rng) * PI * 2.0
	var spread := float(style.spread_mps) * scale * Mulberry32.sample(rng)
	var rise := float(style.rise_mps) * scale * (0.7 + 0.6 * Mulberry32.sample(rng))
	e.n = [cos(a) * spread, sin(a) * spread, rise]
	e.rotation = Mulberry32.sample(rng) * PI * 2.0
	e.size = Mulberry32.sample(rng) - 0.5
	e.frame = Mulberry32.sample(rng) * (LAYER_FRAMES[DUST] - PUFF_TURN_FRAMES - 1.0)

## A flame born at `at`, at `strength` of its style (a fire dying down).
func _add_flame(at: float, x: float, y: float, z: float, style: Dictionary, strength: float, rng: PackedInt64Array) -> void:
	var e := _push(FLAME, at, at + float(style.life_s))
	e.flame = style
	e.scale = strength * (0.7 + 0.6 * Mulberry32.sample(rng))
	e.p = [x, y, z]
	e.rotation = Mulberry32.sample(rng) * PI * 2.0
	e.frame = 3.0 + Mulberry32.sample(rng) * 3.0

# ---- Drawing (per frame). ----

func _draw_tracer(e: Effect, age: float, batch: Batch) -> void:
	if not e.tracer_visible:
		return
	var style: Dictionary = e.tracer
	var length := e.length
	if length <= 1e-6:
		return
	var head := (length * age) / e.span
	var line = style.get("line_px")
	_draw_tail(e, head, e.size, style.glow, float(line) if line != null else float(style.glow.min_px), line == null, batch)
	if style.has("core"):
		_draw_tail(e, head, e.size * float(style.core.share), style.core, float(style.core.min_px), false, batch)
	if (not style.has("body") and not style.has("cast")) or age <= 0.0 or head > length * (1.0 + 1e-6):
		return
	var a := _point_along(e, minf(head, length))
	if style.has("body"):
		_disc(batch, a, float(style.body.size_m), float(_p.min_px) * 2.0, style.body.color, 1.0)
	if style.has("cast"):
		var c: Dictionary = style.cast
		batch.offer(a[0], a[1], a[2], float(c.radius_m), c.color, float(c.intensity), e.cause)

## The part of `e`'s stretch within `length` behind `head`, as streaks fading
## out toward the tail's end.
func _draw_tail(e: Effect, head: float, length: float, light: Dictionary, min_px: float, soft: bool, batch: Batch) -> void:
	var tail := head - length
	var path := e.path
	var cum := e.cum
	var retained := float(e.tracer.get("tail_brightness", 0.0))
	var line = e.tracer.get("line_px")
	var boost := float(e.tracer.get("zoom_boost", 0.0))
	for i in range(0, cum.size() - 1):
		var c0 := cum[i]
		var c1 := cum[i + 1]
		var lo := maxf(c0, tail)
		var hi := minf(c1, head)
		if hi <= lo or c1 <= c0:
			continue
		var u0 := (lo - c0) / (c1 - c0)
		var u1 := (hi - c0) / (c1 - c0)
		var o := i * 3
		var a := [0.0, 0.0, 0.0]
		var b := [0.0, 0.0, 0.0]
		for k in 3:
			a[k] = path[o + k] + (path[o + 3 + k] - path[o + k]) * u0
			b[k] = path[o + k] + (path[o + 3 + k] - path[o + k]) * u1
		_streak(batch, a, b, -float(line) if line != null else float(light.width_m), min_px, light.color, float(light.intensity), _tracer_amplitude((lo - tail) / length, retained), _tracer_amplitude((hi - tail) / length, retained), soft, boost)

func _draw_rotors(batch: Batch, rotors: Array) -> void:
	var blur: Dictionary = _p.rotor_blur
	var albedo: float = blur.albedo
	var grey := [albedo, albedo, albedo]
	for r in rotors:
		var sweep := minf(2.0 * PI, absf(r.turned) * float(blur.shutter))
		var h: Array = r.hub
		var x: Array = r.axis
		var bl: Array = r.blade
		batch.put([h[0], h[1], h[2], r.radius, x[0], x[1], x[2], sweep, bl[0], bl[1], bl[2], albedo, ROTOR, r.chord / r.radius, r.blades, blur.min_chord_px])
		if sweep > 0.0 and float(blur.disc) > 0.0:
			batch.put([h[0], h[1], h[2], r.radius, x[0], x[1], x[2], 0.0, bl[0], bl[1], bl[2], albedo, ROTOR, 0.0, 0.0, blur.disc])
		batch.put([h[0], h[1], h[2], r.radius * float(blur.hub_share), x[0], x[1], x[2], 0.0, bl[0], bl[1], bl[2], albedo, ROTOR, 0.0, 0.0, 1.0])
		if r.mast != null:
			_ribbon(batch, r.mast.top, r.mast.base, 2.0 * r.mastRadius, 1.0, grey, 1.0)

func _draw_flash(e: Effect, age: float, batch: Batch, muzzle: Callable) -> void:
	var s: Dictionary = e.flash
	# On the drawn muzzle, pointing where the round went; the published launch
	# point where nothing of the shooter is drawn.
	var drawn = muzzle.call(e.shooter, e.mount, e.soldier) if muzzle.is_valid() else null
	var at: Array = drawn if drawn != null else e.p
	if s.has("cast") and age < float(s.cast.duration_s):
		var c: Dictionary = s.cast
		var forward := float(c.get("forward_m", 0.0))
		_offer_burning_out(batch, c, age, _along(at, e.n, forward), 1.0, e.cause)
	var x := age / float(s.duration_s)
	if x < 1.0:
		var k := (1.0 - x) * (1.0 - x)
		_glow(batch, at, float(s.size_m) * (0.7 + 0.5 * x), float(_p.min_px), s.color, float(s.intensity) * k, e.rotation, 0.0)
		var reach := float(s.tongue_m) * (0.6 + 0.4 * x)
		_streak(batch, at, _along(at, e.n, reach), float(s.size_m) * 0.9, float(_p.min_px), s.color, float(s.intensity) * k, 1.0, 0.15)
	if float(s.fireball_m) > 0.0 and age < float(s.fireball_s):
		var f := age / float(s.fireball_s)
		_flipbook(batch, _along(at, e.n, float(s.fireball_m) * (0.2 + 0.6 * f)), float(s.fireball_m) * (0.6 + 0.6 * sqrt(f)), e.rotation, FIRE, f * 10.0, s.color, (1.0 - f) * (1.0 - f), float(s.intensity) * 0.25 * (1.0 - f), 0.5)

func _draw_impact(e: Effect, age: float, batch: Batch) -> void:
	var s: Dictionary = e.impact
	var x := age / e.span
	var size := e.size * (0.7 + 0.6 * sqrt(x))
	var a := _along(e.p, e.n, size * 0.45)
	a[2] += e.size * 0.3 * x
	var opacity := float(s.opacity) * minf(1.0, x * 12.0) * (1.0 - x) * (1.0 - x)
	_flipbook(batch, a, size, e.rotation, DUST, (e.path[0] + x * 0.4) * (LAYER_FRAMES[DUST] - 1), s.color, opacity, 0.0, size * 0.6)
	if not e.tracer_visible:
		return
	if s.has("cast") and age < float(s.cast.duration_s):
		_offer_burning_out(batch, s.cast, age, _along(e.p, e.n, 0.5), sqrt(e.size / float(s.size_m)), e.cause)
	if float(s.flash) > 0.0 and x < 0.12:
		var k := 1.0 - x / 0.12
		var r := e.size * 0.35
		_glow(batch, _along(e.p, e.n, r), r, float(_p.min_px), _p.sparks.color, float(s.flash) * k * k, e.rotation)

func _draw_sparks(e: Effect, age: float, batch: Batch, color: Array, intensity: float) -> void:
	var s: Dictionary = _p.sparks
	var fade := 1.0 - age / e.span
	var v := e.sparks
	var k := 0
	while k + 2 < v.size():
		var speed := _hypot3(v[k], v[k + 1], v[k + 2])
		if speed == 0.0:
			speed = 1.0
		var lag := minf(age, (float(s.length_m) * e.size) / speed)
		var t1 := age
		var t0 := age - lag
		var a := [0.0, 0.0, 0.0]
		var b := [0.0, 0.0, 0.0]
		for j in 3:
			var g := -0.5 * GRAVITY if j == 2 else 0.0
			a[j] = e.p[j] + v[k + j] * t0 + g * t0 * t0
			b[j] = e.p[j] + v[k + j] * t1 + g * t1 * t1
		_streak(batch, a, b, float(s.width_m) * e.size, float(_p.min_px) * 0.5, color, intensity * fade, 0.0, 1.0)
		k += 3

## Where smoke born at `p`, moving at `n`, is `age` seconds into a life of `life`.
func _smoke_at(p: Array, n: Array, life: float, age: float) -> Array:
	var x := age / life
	var wind: Array = _p.wind_mps
	var climb := life * 0.5 * (1.0 - (1.0 - x) * (1.0 - x))
	var drift := age * (0.3 + 0.7 * x)
	return [p[0] + n[0] * climb + float(wind[0]) * drift, p[1] + n[1] * climb + float(wind[1]) * drift, p[2] + n[2] * climb]

func _draw_trail(e: Effect, age: float, batch: Batch) -> void:
	var s: Dictionary = e.trail
	var length := e.length
	if length <= 1e-6:
		return
	var flown := minf(length, (length * age) / _dt)
	var rise := [0.0, 0.0, float(s.rise_mps)]
	for i in range(0, e.cum.size() - 1):
		var lo := e.cum[i]
		var hi := minf(e.cum[i + 1], flown)
		if hi <= lo:
			continue
		var per_m := _dt / length
		var age_lo := age - lo * per_m
		var age_hi := maxf(0.0, age - hi * per_m)
		var a := _smoke_at(_point_along(e, lo), rise, float(s.life_s), age_lo)
		var b := _smoke_at(_point_along(e, hi), rise, float(s.life_s), age_hi)
		var x := minf(1.0, (age_lo + age_hi) / 2.0 / float(s.life_s))
		_ribbon(batch, a, b, 2.0 * RIBBON_WIDTH * _puff_radius(s, x), float(_p.min_px), s.albedo, float(s.ribbon) * pow(1.0 - x, 2))

func _draw_puff(e: Effect, age: float, batch: Batch) -> void:
	var s: Dictionary = e.puff
	var x := age / e.span
	var a := _smoke_at(e.p, e.n, float(s.life_s), age)
	var radius := e.scale * _puff_radius(s, x)
	var fade := minf(1.0, age * PUFF_FADE_IN) * pow(1.0 - x, 1.2)
	_flipbook(batch, a, radius, e.rotation + e.size * x * 1.5, DUST, e.frame + x * PUFF_TURN_FRAMES, s.albedo, float(s.opacity) * e.alpha * fade, 0.0, radius * 0.8, true)

func _draw_flame(e: Effect, age: float, batch: Batch) -> void:
	var s: Dictionary = e.flame
	var x := age / e.span
	var a := [e.p[0], e.p[1], e.p[2] + float(s.size_m) * e.scale * (0.3 + 1.2 * x)]
	_flipbook(batch, a, float(s.size_m) * e.scale * (0.8 + 0.4 * sin(PI * x)), e.rotation, FIRE, e.frame + x * 4.0, s.tint, minf(1.0, x * 6.0) * (1.0 - x), float(s.emissive), 0.3)

## A burning source's fire light, flickering on the clock, dying down over the
## burn's last third.
func _draw_fire_light(src: Dictionary, clock: float, batch: Batch) -> void:
	var f: Dictionary = src.style.flame
	var age: float = clock - src.start
	var burn: float = src.style.burn_s
	if age < 0.0 or age >= burn or clock > _noted + float(f.life_s) or float(f.light_intensity) <= 0.0:
		return
	if float(src.seed) / 4294967296.0 >= _keep:
		return
	var phase := (int(src.seed) % 1000) * 0.01
	var flicker := 0.7 + 0.3 * sin(clock * 11.3 + phase) * sin(clock * 5.7 + phase * 1.7)
	var strength := minf(minf(1.0, ((burn - age) / burn) * 3.0), age * 4.0)
	_glow(batch, [src.x, src.y, src.top + 0.5], float(f.light_m), 0.0, f.light, float(f.light_intensity) * flicker * strength, phase, 0.0)
	if f.has("cast"):
		var c: Dictionary = f.cast
		batch.offer(src.x, src.y, src.top + 0.5, float(c.radius_m), c.color, float(c.intensity) * flicker * strength, src.cause)

func _draw_blast(e: Effect, age: float, batch: Batch) -> void:
	var s: Dictionary = _p.blast
	var x := age / e.span
	var a := [e.p[0], e.p[1], e.p[2] + e.size * (0.55 + 0.5 * x)]
	var heat := maxf(0.0, 1.0 - x * 2.2)
	var c: Dictionary = s.cast
	if age < float(c.duration_s):
		var share := e.size / float(s.reference_size_m)
		_offer_burning_out(batch, c, age, [e.p[0], e.p[1], e.p[2] + e.size * 0.5], sqrt(share), "blast", minf(1.0, share))
	# The glow in the air round the burst; the fireball then sits where the
	# glow does, as the browser's shared scratch point has it.
	if heat > 0.0:
		a = [e.p[0], e.p[1], e.p[2] + e.size * 0.5]
		_glow(batch, a, e.size * 1.2, 0.0, s.flash, float(s.spill) * heat * heat, e.rotation, 0.0)
	_flipbook(batch, a, e.size * (0.75 + 0.5 * sqrt(x)), e.rotation, FIRE, minf(x * 1.15, 1.0) * (LAYER_FRAMES[FIRE] - 1), s.tint, float(s.opacity) * minf(1.0, (1.0 - x) * 3.0), float(s.emissive) * heat * heat, e.size * 0.5)
	if age < float(s.flash_duration_s):
		var k := 1.0 - age / float(s.flash_duration_s)
		_glow(batch, [e.p[0], e.p[1], e.p[2] + e.size * 0.4], e.size * 0.3, float(_p.min_px), s.flash, float(s.flash_intensity) * k * k, e.rotation, 0.5)

# ---- Instance writers, in the shader's layout. ----

func _streak(batch: Batch, a: Array, b: Array, width: float, min_px: float, color: Array, intensity: float, along_a: float, along_b: float, soft := false, boost := 0.0) -> void:
	var w := (-boost if soft else boost) if boost != 0.0 else (1.0 if soft else 0.0)
	batch.put([a[0], a[1], a[2], width, b[0], b[1], b[2], min_px, color[0] * intensity, color[1] * intensity, color[2] * intensity, 0.0, STREAK, along_a, along_b, w])

func _glow(batch: Batch, p: Array, radius: float, min_px: float, color: Array, intensity: float, rotation: float, rays := 1.0) -> void:
	batch.put([p[0], p[1], p[2], radius, rotation, min_px, rays, 0.0, color[0] * intensity, color[1] * intensity, color[2] * intensity, 0.0, GLOW, 0.0, 0.0, 0.0])

func _disc(batch: Batch, p: Array, radius: float, min_px: float, color: Array, opacity: float) -> void:
	batch.put([p[0], p[1], p[2], radius, 0.0, min_px, 0.0, 0.0, color[0] * opacity, color[1] * opacity, color[2] * opacity, opacity, GLOW, 0.0, 0.0, 0.0])

func _ribbon(batch: Batch, a: Array, b: Array, width: float, min_px: float, albedo: Array, opacity: float) -> void:
	batch.put([a[0], a[1], a[2], width, b[0], b[1], b[2], min_px, albedo[0], albedo[1], albedo[2], opacity, STREAK, 1.0, 1.0, 0.0])

func _flipbook(batch: Batch, p: Array, radius: float, rotation: float, layer: int, frame: float, tint: Array, opacity: float, emissive: float, soft_m: float, lit := false) -> void:
	batch.put([p[0], p[1], p[2], radius, rotation, frame, layer, soft_m, tint[0], tint[1], tint[2], opacity, FLIPBOOK, emissive, 1.0 if lit else 0.0, 0.0])

## A burning-out cast light `age` seconds in, its radius scaled by `reach`.
func _offer_burning_out(batch: Batch, c: Dictionary, age: float, at: Array, reach: float, cause: String, strength := 1.0) -> void:
	var f := age / float(c.duration_s)
	batch.offer(at[0], at[1], at[2], float(c.radius_m) * reach, c.color, float(c.intensity) * strength * (1.0 - f * f), cause)

# ---- Helpers. ----

static func _tracer_amplitude(u: float, retained: float) -> float:
	return sqrt(retained + (1.0 - retained) * u * u) if retained != 0.0 else u

static func _puff_radius(s: Dictionary, x: float) -> float:
	return float(s.size_m[0]) + (float(s.size_m[1]) - float(s.size_m[0])) * sqrt(x)

## The point `at` metres along a tracer's path.
static func _point_along(e: Effect, at: float) -> Array:
	var cum := e.cum
	var path := e.path
	var i := 0
	while i + 2 < cum.size() and cum[i + 1] < at:
		i += 1
	var u := (at - cum[i]) / maxf(cum[i + 1] - cum[i], 1e-6)
	var o := i * 3
	return [path[o] + (path[o + 3] - path[o]) * u, path[o + 1] + (path[o + 4] - path[o + 1]) * u, path[o + 2] + (path[o + 5] - path[o + 2]) * u]

## `steadyInstances`: what a source of this look holds `age` seconds in.
static func _steady_instances(s: Dictionary, age: float) -> float:
	if age < float(s.burn_s):
		return float(s.smoke_hz) * float(s.smoke.life_s) + float(s.flame.rate_hz) * float(s.flame.life_s) + 1.0
	if age < float(s.burn_s) + float(s.smoulder_s):
		return float(s.smoulder_hz) * float(s.smoulder.life_s)
	return 0.0

## Seconds after the killing hit that a cook-off's turret strikes the deck
## (`impactAfter`): its delay, then a body thrown `rise_m` up falls back.
static func impact_after(feel: Dictionary) -> float:
	return float(feel.delay_s) + sqrt((8.0 * float(feel.rise_m)) / GRAVITY)

## `table`'s row for `key`, or its `default` (`pick`).
static func _pick(table: Dictionary, key: String) -> Variant:
	var row = table.get(key)
	return row if row != null else table.default

static func _along(p: Array, n: Array, d: float) -> Array:
	return [p[0] + n[0] * d, p[1] + n[1] * d, p[2] + n[2] * d]

static func _normalized(v: Array) -> Array:
	var length := _hypot3(v[0], v[1], v[2])
	return v if length == 0.0 else [v[0] / length, v[1] / length, v[2] / length]

## `Math.hypot` of three, for lengths far from over- or underflow.
static func _hypot3(x: float, y: float, z: float) -> float:
	return sqrt(x * x + y * y + z * z)

## JavaScript's `Math.round`: halves round up.
static func _js_round(x: float) -> int:
	return floori(x + 0.5)
