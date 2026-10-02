/** A ground stroke's stretches as the simulation exports them, and the one
 *  distance every CPU reader takes from them. A stroke is round where two
 *  stretches meet and cut square across its first and last point
 *  (`contract::ground::stretch_contains`); the terrain material's
 *  `strokeCutInside` is the same arithmetic in f32. */
import { vec2, type Vec2 } from "math";
import { segment2 } from "math/shapes";

/** Floats per exported stretch: `ax, ay, bx, by, half width`, the surface's
 *  kind or the forest's id, and which ends are cut. The native layout must
 *  publish the same strides. */
export const STROKE_FLOATS = 7;
/** Where a stretch's cut ends are in its record. */
export const STROKE_CUTS = 6;
/** The stretch is cut square at its end `a` (`b`): its stroke ends there, or
 *  within half its width of there. */
export const CUT_A = 1;
export const CUT_B = 2;

/** The native layout's own words for the stretch record. */
export interface StrokeLayout {
  surfaceStrokeStride: number;
  forestStrokeStride: number;
  strokeCuts: { a: number; b: number };
}

/** Refuse a native layout whose stretches this module would misread. */
export function checkStrokeLayout(layout: StrokeLayout): void {
  if (
    layout.surfaceStrokeStride !== STROKE_FLOATS ||
    layout.forestStrokeStride !== STROKE_FLOATS ||
    layout.strokeCuts.a !== CUT_A ||
    layout.strokeCuts.b !== CUT_B
  )
    throw new Error("stroke stretches differ from the records the ground readers expect");
}

const _point: Vec2 = [0, 0],
  _a: Vec2 = [0, 0],
  _b: Vec2 = [0, 0],
  _closest: Vec2 = [0, 0];

/** How far `(x, y)` lies inside the stretch at `records[o]` (negative
 *  outside): its half width less the distance to the stretch, and past a cut
 *  end the distance to that square end instead. */
export function strokeInside(records: ArrayLike<number>, o: number, x: number, y: number): number {
  vec2.set(_point, x, y);
  vec2.set(_a, records[o], records[o + 1]);
  vec2.set(_b, records[o + 2], records[o + 3]);
  segment2.closestPoint(_closest, _point, _a, _b);
  const half = records[o + 4];
  const inside = half - vec2.distance(_point, _closest);
  const cuts = records[o + STROKE_CUTS];
  if (cuts === 0) return inside;
  const dx = _b[0] - _a[0],
    dy = _b[1] - _a[1];
  const length = Math.hypot(dx, dy);
  const along = ((x - _a[0]) * dx + (y - _a[1]) * dy) / length;
  // How far past a cut end, along the stretch: negative short of it.
  const past = Math.max(
    cuts & CUT_A ? -along : -Infinity,
    cuts & CUT_B ? along - length : -Infinity,
  );
  if (past <= 0) return Math.min(inside, -past);
  const aside = Math.max(Math.abs((x - _a[0]) * dy - (y - _a[1]) * dx) / length - half, 0);
  return -Math.hypot(aside, past);
}
