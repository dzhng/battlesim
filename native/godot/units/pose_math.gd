# Frozen: a port of the per-frame presentation `crates/presentation` now owns; no fixes here, deleted with Godot (slice 25).
extends RefCounted

## The small numeric helpers the browser's pose code takes from the `math`
## package, ported exactly: the short way between two angles and the sine
## easings (its seeded sequence is `core/mulberry32.gd`).

const TAU_ := TAU

## The signed turn from `current` to `target`, in (−π, π] (`deltaAngle`).
static func delta_angle(current: float, target: float) -> float:
	var diff := target - current
	var delta := diff - TAU_ * floorf(diff / TAU_)
	return delta - TAU_ if delta > PI else delta

static func sine_in(x: float) -> float:
	return 1.0 - cos(x * PI / 2.0)

static func sine_in_out(x: float) -> float:
	return -(cos(PI * x) - 1.0) / 2.0

## `sideKey`: one number per (side, id), the `even` side's ids even.
static func side_key(id: int, side: String, even: String) -> int:
	return id * 2 + (0 if side == even else 1)

## `a` toward `b` by `t`, on `[x, y, z]` points in double precision.
static func lerp3(a: Array, b: Array, t: float) -> Array:
	return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
