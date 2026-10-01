/** Native forest primitives. Bounds never substitute for physical membership. */
import { vec2, type Vec2 } from "math";
import { segment2, triangle2 } from "math/shapes";
import type { WorldExports, WorldLayout } from "../worldMesh";

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
const EMPTY = new Float32Array(0);

/** The floats per record the forest-floor shader reads; the native layout
 *  must publish the same strides. */
export const FOREST_STROKE_FLOATS = 6;
export const FOREST_TRIANGLE_FLOATS = 7;
export const FOREST_BOUNDARY_FLOATS = 5;

/** Each primitive stream is grouped in authored-ID order by the native owner. */
function ranges(records: Float32Array, stride: number): Map<number, Float32Array> {
  const out = new Map<number, Float32Array>();
  for (let first = 0; first < records.length; ) {
    const id = records[first + stride - 1];
    let end = first + stride;
    while (end < records.length && records[end + stride - 1] === id) end += stride;
    if (out.has(id)) throw new Error("forest primitive groups must be contiguous");
    out.set(id, records.subarray(first, end));
    first = end;
  }
  return out;
}

export function buildForestShapes(exports: WorldExports, layout: WorldLayout): ForestShape[] {
  if (
    layout.forestStrokeStride !== FOREST_STROKE_FLOATS ||
    layout.forestTriangleStride !== FOREST_TRIANGLE_FLOATS ||
    layout.forestBoundaryStride !== FOREST_BOUNDARY_FLOATS
  )
    throw new Error("forest primitive strides differ from the forest-floor shader's records");
  const rects = new Map<number, readonly [number, number, number, number]>();
  for (let r = 0; r < exports.forestRectIds.length; r++) {
    const o = r * layout.areaStride;
    rects.set(exports.forestRectIds[r], [
      exports.forests[o],
      exports.forests[o + 1],
      exports.forests[o + 2],
      exports.forests[o + 3],
    ]);
  }
  const strokes = ranges(exports.forestStrokes, layout.forestStrokeStride);
  const triangles = ranges(exports.forestTriangles, layout.forestTriangleStride);
  const boundaries = ranges(exports.forestBoundaries, layout.forestBoundaryStride);
  const out: ForestShape[] = [];
  for (let o = 0; o < exports.forestMetadata.length; o += layout.forestMetadataStride) {
    const id = exports.forestMetadata[o];
    const kind = layout.forestShapeKinds[exports.forestMetadata[o + 2]] as ForestShapeKind;
    const rect = rects.get(id);
    if (kind === "rectangle" && !rect)
      throw new Error("rectangular forest has no native rectangle");
    const range = id * layout.forestTrunkRangeStride;
    if (range + 1 >= exports.forestTrunkRanges.length)
      throw new Error("forest has no native source trunk range");
    out.push({
      canopy: exports.forestMetadata[o + 1],
      trunkRange: [exports.forestTrunkRanges[range], exports.forestTrunkRanges[range + 1]],
      kind,
      rect,
      strokes: strokes.get(id) ?? EMPTY,
      triangles: triangles.get(id) ?? EMPTY,
      boundaries: boundaries.get(id) ?? EMPTY,
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
 * original arithmetic; polygons use native membership and real ring edges. */
export function forestInside(shape: ForestShape, x: number, y: number): number {
  if (shape.kind === "rectangle") {
    const [sx, sy, w, h] = shape.rect!;
    return Math.min(x - sx, sx + w - x, y - sy, sy + h - y);
  }
  vec2.set(_forestPoint, x, y);
  let distance = -1e9;
  if (shape.kind === "stroke") {
    for (let o = 0; o < shape.strokes.length; o += FOREST_STROKE_FLOATS) {
      vec2.fromBuffer(_forestA, shape.strokes, o);
      vec2.fromBuffer(_forestB, shape.strokes, o + 2);
      segment2.closestPoint(_forestClosest, _forestPoint, _forestA, _forestB);
      distance = Math.max(
        distance,
        shape.strokes[o + 4] - vec2.distance(_forestPoint, _forestClosest),
      );
    }
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
