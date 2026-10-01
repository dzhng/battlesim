/** Public exact paved primitives, retained in their native export order,
 *  and the runs fields are cut along. */
export interface SurfaceGeometry {
  surfaceStrokes: Float32Array;
  surfaceStrokeStride: number;
  /** The road strokes' authored control runs: `ax, ay, bx, by`. */
  surfaceRuns: Float32Array;
  surfaceRunStride: number;
  surfaceTriangles: Float32Array;
  surfaceTriangleStride: number;
  surfaceBoundaries: Float32Array;
  surfaceBoundaryStride: number;
  /** The rivers' long runs between authored points: `ax, ay, bx, by`. */
  riverRuns: Float32Array;
  riverRunStride: number;
}

export const SURFACE_ROAD = 1;
/** Agricultural guides read the road strokes' authored runs and the rivers'
 * long runs (so a field meets a rounded bend edge-on, not its short samples) and
 * the native exposed road boundary. Sidewalks and triangulation diagonals
 * never become field boundaries. */
export function plotGuideEdges(site: SurfaceGeometry): Float32Array {
  const boundaries = site.surfaceBoundaries;
  let count =
    site.surfaceRuns.length / site.surfaceRunStride + site.riverRuns.length / site.riverRunStride;
  for (let o = 0; o < boundaries.length; o += site.surfaceBoundaryStride)
    if (boundaries[o + 4] === SURFACE_ROAD) count++;
  const edges = new Float32Array(count * 4);
  let at = 0;
  for (const [runs, stride] of [
    [site.surfaceRuns, site.surfaceRunStride],
    [site.riverRuns, site.riverRunStride],
  ] as const)
    for (let o = 0; o < runs.length; o += stride) {
      edges.set(runs.subarray(o, o + 4), at);
      at += 4;
    }
  for (let o = 0; o < boundaries.length; o += site.surfaceBoundaryStride)
    if (boundaries[o + 4] === SURFACE_ROAD) {
      edges.set(boundaries.subarray(o, o + 4), at);
      at += 4;
    }
  return edges;
}
