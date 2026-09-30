/** Public exact paved primitives, retained in their native export order. */
export interface SurfaceGeometry {
  surfaceStrokes: Float32Array;
  surfaceStrokeStride: number;
  surfaceTriangles: Float32Array;
  surfaceTriangleStride: number;
  surfaceBoundaries: Float32Array;
  surfaceBoundaryStride: number;
}

export const SURFACE_ROAD = 1;
/** Agricultural guides read road strokes and the native exposed road boundary.
 * Sidewalks and triangulation diagonals never become field boundaries. */
export function roadPlotEdges(site: SurfaceGeometry): Float32Array {
  const strokes = site.surfaceStrokes,
    boundaries = site.surfaceBoundaries;
  let count = 0;
  for (let o = 0; o < strokes.length; o += site.surfaceStrokeStride)
    if (strokes[o + 5] === SURFACE_ROAD) count++;
  for (let o = 0; o < boundaries.length; o += site.surfaceBoundaryStride)
    if (boundaries[o + 4] === SURFACE_ROAD) count++;
  const edges = new Float32Array(count * 4);
  let at = 0;
  for (let o = 0; o < strokes.length; o += site.surfaceStrokeStride)
    if (strokes[o + 5] === SURFACE_ROAD) {
      edges.set(strokes.subarray(o, o + 4), at);
      at += 4;
    }
  for (let o = 0; o < boundaries.length; o += site.surfaceBoundaryStride)
    if (boundaries[o + 4] === SURFACE_ROAD) {
      edges.set(boundaries.subarray(o, o + 4), at);
      at += 4;
    }
  return edges;
}
