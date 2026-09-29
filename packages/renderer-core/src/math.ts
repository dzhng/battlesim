// The few helpers the `math` package lacks, shared by every renderer package;
// one owner, no per-package copies. Everything else — vectors, matrices,
// shapes, clamp and lerp — comes from `math` itself.
import type { Vec2, Vec3 } from "math";
import type { Box3 } from "math/shapes";

/** Hermite smoothstep of `value` between two edges, clamped to [0, 1]
 *  (`math` has `fade`, a quintic, but no cubic smoothstep). */
export function smoothstep(edge0: number, edge1: number, value: number): number {
  const t = Math.max(0, Math.min(1, (value - edge0) / (edge1 - edge0)));
  return t * t * (3 - 2 * t);
}

/** A 32-bit hash of a string (FNV-1a): a stable seed from a name. */
export function hashString(key: string): number {
  let h = 2166136261;
  for (let i = 0; i < key.length; i++) h = Math.imul(h ^ key.charCodeAt(i), 16777619);
  return h >>> 0;
}

/** A direction component smaller than this is treated as parallel to that
 *  slab, the same cut `math/shapes`' `raycast3.intersectsBox3` makes. */
const PARALLEL_EPSILON = 1e-10;

/** Where a ray is inside a box: writes [enter, exit] distances along `dir`
 *  into `out`, the enter distance clamped to 0 when the origin is inside.
 *  False on a miss, leaving `out` unspecified. `math/shapes` answers only
 *  whether a ray meets a box (`raycast3.intersectsBox3`), not where; picking
 *  and the shadow receiver range need the distances. A ray along a slab face
 *  counts as inside it. */
export function rayBox3Interval(out: Vec2, origin: Vec3, dir: Vec3, box: Box3): boolean {
  let enter = 0;
  let exit = Infinity;
  for (let a = 0; a < 3; a++) {
    const lo = box[a];
    const hi = box[a + 3];
    if (Math.abs(dir[a]) < PARALLEL_EPSILON) {
      if (origin[a] < lo || origin[a] > hi) return false;
      continue;
    }
    const tLo = (lo - origin[a]) / dir[a];
    const tHi = (hi - origin[a]) / dir[a];
    enter = Math.max(enter, Math.min(tLo, tHi));
    exit = Math.min(exit, Math.max(tLo, tHi));
    if (exit < enter) return false;
  }
  out[0] = enter;
  out[1] = exit;
  return true;
}
