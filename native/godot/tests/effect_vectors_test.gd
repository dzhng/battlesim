extends "res://tests/check.gd"

## The effects (`effects/effect_frame.gd`) draw what the browser's
## `EffectFrame` draws for the same causes, seeded as it seeds them: every
## instance's sixteen floats and every cast light at fixed clocks after a few
## publications that exercise every kind of effect (`nativeEffectVectors.test.ts`
## wrote them from the TypeScript). The flashes sit on the drawn muzzles the
## vectors name; a rotor is drawn throughout.

const EffectFrame = preload("res://effects/effect_frame.gd")

const VECTORS := "res://tests/fixtures/effects/effect-vectors.json"
## Instances agree to float32 rounding of double sums (positions in hundreds
## of metres) and to the last digits the browser's transcendentals may differ in.
const TOLERANCE := 2e-4

func run() -> bool:
	var vectors: Dictionary = read_json(VECTORS)
	var frame := EffectFrame.new(float(vectors.tickHz), vectors.presentation)
	var batch := EffectFrame.Batch.new(frame.capacity())
	var drawn := {}
	for m in vectors.muzzles:
		drawn[[int(m[0]), int(m[1]), m[2] if m[2] == null else int(m[2])]] = m[3]
	var muzzle := func(shooter: int, mount: int, soldier) -> Variant:
		return drawn.get([shooter, mount, soldier if soldier == null else int(soldier)])
	var rotor: Dictionary = vectors.rotor
	var rotors := func() -> Array: return [rotor]
	var noted := 0
	var publications: Array = vectors.publications.map(_ints)
	for want in vectors.frames:
		var clock := float(want.clock)
		while noted < int(want.noted):
			frame.note(publications[noted])
			noted += 1
		frame.build(clock, batch, muzzle, rotors)
		var at := "at %s s" % clock
		var instances: Array = want.instances
		check(batch.count * EffectFrame.FLOATS == instances.size(), "%s: %d instances, not the browser's %d" % [at, batch.count, instances.size() / EffectFrame.FLOATS])
		var worst := 0.0
		var where := -1
		for k in mini(instances.size(), batch.count * EffectFrame.FLOATS):
			var off := absf(batch.data[k] - float(instances[k])) / maxf(1.0, absf(float(instances[k])))
			if off > worst:
				worst = off
				where = k
		check(worst <= TOLERANCE, "%s: instance %d float %d is %s, not the browser's %s" % [at, where / EffectFrame.FLOATS, where % EffectFrame.FLOATS, batch.data[maxi(where, 0)], instances[maxi(where, 0)]])
		check(batch.light_count == want.lights.size(), "%s: %d lights, not %d" % [at, batch.light_count, want.lights.size()])
		for i in mini(batch.light_count, want.lights.size()):
			var light: Dictionary = want.lights[i]
			var mine := batch.lights.slice(i * EffectFrame.LIGHT_FLOATS, i * EffectFrame.LIGHT_FLOATS + 7)
			var theirs: Array = light.at + [light.radius] + light.rgb
			var close := true
			for k in 7:
				close = close and absf(mine[k] - float(theirs[k])) <= TOLERANCE * maxf(1.0, absf(float(theirs[k])))
			check(close and batch.causes[i] == light.cause, "%s: light %d is %s %s, not %s %s" % [at, i, batch.causes[i], mine, light.cause, theirs])
		check(_same(frame.counts(clock, batch), want.counts), "%s: counts %s, not %s" % [at, frame.counts(clock, batch), want.counts])
	return true

## A publication with the integer fields JSON reads as floats made integers.
static func _ints(pub: Dictionary) -> Dictionary:
	pub.tick = int(pub.tick)
	for s in pub.segments:
		if s.shooter != null:
			s.shooter = int(s.shooter)
		for r in s.ricochets:
			r.point = int(r.point)
	for u in pub.shooters:
		u.key = int(u.key)
		u.members = u.members.map(func(m): return int(m))
		if not u.has("airborne"):
			u.airborne = false
		if not u.has("smoking"):
			u.smoking = false
		for m in u.mounts:
			if m.muzzle != null and m.muzzle.on != null:
				m.muzzle.on = int(m.muzzle.on)
	return pub

static func _same(a: Dictionary, b: Dictionary) -> bool:
	for group in ["running", "instances"]:
		for key in b[group]:
			if int(a[group][key]) != int(b[group][key]):
				return false
	return int(a.lights) == int(b.lights)
