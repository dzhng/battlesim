/** The forests as shapes of the native forest primitives. Bounds never
 *  substitute for physical membership. */
import { vec2, type Vec2 } from "math";
import { segment2, triangle2 } from "math/shapes";
import type { WorldExports } from "../worldMesh";
import { STROKE_FLOATS, strokeInside } from "./strokes";

export type ForestShapeKind = "rectangle" | "stroke" | "polygon";

export interface ForestShape {
  canopy: number;
  /** Original source prop IDs, immutable after native forest generation. */
  trunkRange: readonly [number, number];
  /** Named by the native layout's `forestShapeKinds`. */
  kind: ForestShapeKind;
  /** A rectangle's `[x, y, w, h]`; only rectangles have one. */
  rect?: readonly [number, number, number, number];
  strokes: Float32Array;
  triangles: Float32Array;
  boundaries: Float32Array;
}

/** The floats per record the forest-floor shader reads; the native layout
 *  must publish the same strides. */
export const FOREST_STROKE_FLOATS = STROKE_FLOATS;
export const FOREST_TRIANGLE_FLOATS = 7;
export const FOREST_BOUNDARY_FLOATS = 5;
const KINDS: ForestShapeKind[] = ["rectangle", "stroke", "polygon"];
/** Numbers per forest in the ground's table (`Ground.forests`). */
const FOREST_NUMBERS = 14;

/** The forests the ground resolved (`table`, `Ground.forests`) over the
 *  streams of `source` they index. */
export function forestShapes(
  table: Float64Array,
  source: Pick<WorldExports, "forestStrokes" | "forestTriangles" | "forestBoundaries">,
): ForestShape[] {
  const out: ForestShape[] = [];
  for (let o = 0; o < table.length; o += FOREST_NUMBERS) {
    const f = table.subarray(o, o + FOREST_NUMBERS);
    out.push({
      canopy: f[0],
      trunkRange: [f[1], f[2]],
      kind: KINDS[f[3]],
      ...(!Number.isNaN(f[4]) && { rect: [f[4], f[5], f[6], f[7]] as const }),
      strokes: source.forestStrokes.subarray(f[8], f[9]),
      triangles: source.forestTriangles.subarray(f[10], f[11]),
      boundaries: source.forestBoundaries.subarray(f[12], f[13]),
    });
  }
  return out;
}

const _forestPoint: Vec2 = [0, 0],
  _forestA: Vec2 = [0, 0],
  _forestB: Vec2 = [0, 0],
  _forestC: Vec2 = [0, 0],
  _forestClosest: Vec2 = [0, 0];

/** Signed primitive distance for crown fitting. Exact rectangles retain their
 * original arithmetic; polygons use native membership and real ring edges;
 * a stroke ends square, as the simulation's does. */
export function forestInside(shape: ForestShape, x: number, y: number): number {
  if (shape.kind === "rectangle") {
    const [sx, sy, w, h] = shape.rect!;
    return Math.min(x - sx, sx + w - x, y - sy, sy + h - y);
  }
  vec2.set(_forestPoint, x, y);
  let distance = -1e9;
  if (shape.kind === "stroke") {
    for (let o = 0; o < shape.strokes.length; o += FOREST_STROKE_FLOATS)
      distance = Math.max(distance, strokeInside(shape.strokes, o, x, y));
    return distance;
  }
  let inside = false,
    nearest = 1e9;
  for (let o = 0; o < shape.triangles.length; o += FOREST_TRIANGLE_FLOATS) {
    vec2.fromBuffer(_forestA, shape.triangles, o);
    vec2.fromBuffer(_forestB, shape.triangles, o + 2);
    vec2.fromBuffer(_forestC, shape.triangles, o + 4);
    inside ||= triangle2.containsPoint(_forestA, _forestB, _forestC, _forestPoint);
  }
  for (let o = 0; o < shape.boundaries.length; o += FOREST_BOUNDARY_FLOATS) {
    vec2.fromBuffer(_forestA, shape.boundaries, o);
    vec2.fromBuffer(_forestB, shape.boundaries, o + 2);
    segment2.closestPoint(_forestClosest, _forestPoint, _forestA, _forestB);
    nearest = Math.min(nearest, vec2.distance(_forestPoint, _forestClosest));
  }
  return inside ? nearest : -nearest;
}
