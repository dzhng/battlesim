extends RefCounted

## The `math` package's Mulberry32 sequence (`math/random`), ported exactly:
## the browser seeds presentation choices with it (a soldier's manner, a
## car's paint), so native draws the same ones.

## JavaScript's `Math.imul`: the low 32 bits of `a · b`, as an unsigned word.
static func imul(a: int, b: int) -> int:
	a &= 0xffffffff
	b &= 0xffffffff
	return ((a & 0xffff) * b + ((((a >> 16) * b) & 0xffff) << 16)) & 0xffffffff

## A state seeded with the word `seed` (`mulberry32.create`).
static func create(seed: int) -> PackedInt64Array:
	return PackedInt64Array([seed & 0xffffffff])

## The next number in [0, 1) of `state`, advancing it (`mulberry32.sample`).
static func sample(state: PackedInt64Array) -> float:
	state[0] = (state[0] + 0x6d2b79f5) & 0xffffffff
	var t: int = state[0]
	t = imul(t ^ (t >> 15), t | 1)
	t = (t ^ ((t + imul(t ^ (t >> 7), t | 61)) & 0xffffffff)) & 0xffffffff
	return float((t ^ (t >> 14)) & 0xffffffff) / 4294967296.0
