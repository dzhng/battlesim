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

/** The kinds a paved record's tag names, in the native layout's order: a
 *  stretch, triangle or boundary edge carries its area's kind as an index
 *  into these. The biome's `roads` rows are keyed by them. */
export const SURFACE_AREA_KINDS = ["road", "country_road", "dirt_track", "sidewalk"] as const;
export type SurfaceAreaKind = (typeof SURFACE_AREA_KINDS)[number];
/** Which of them are carriageways: all but the sidewalk. */
const ROAD_AREA_KINDS: readonly string[] = ["road", "country_road", "dirt_track"];

/** The native layout's own words for the paved kinds. */
export interface SurfaceKindLayout {
  surfaceAreaKinds: string[];
  roadAreaKinds: string[];
}

/** Refuse a native layout whose paved kinds this module would misname (one
 *  from an older build names none). */
export function checkSurfaceKinds(layout: Partial<SurfaceKindLayout>): void {
  const same = (a: readonly string[] | undefined, b: readonly string[]) =>
    a?.length === b.length && a.every((kind, k) => kind === b[k]);
  if (
    !same(layout.surfaceAreaKinds, SURFACE_AREA_KINDS) ||
    !same(layout.roadAreaKinds, ROAD_AREA_KINDS)
  )
    throw new Error("paved surface kinds differ from the ones the ground readers expect");
}

/** The paved kinds `site`'s areas name. */
export function pavedKinds(site: SurfaceGeometry): Set<SurfaceAreaKind> {
  const named = new Set<SurfaceAreaKind>();
  for (const [records, stride, kind] of [
    [site.surfaceStrokes, site.surfaceStrokeStride, 5],
    [site.surfaceTriangles, site.surfaceTriangleStride, 6],
  ] as const)
    for (let o = 0; o < records.length; o += stride)
      named.add(SURFACE_AREA_KINDS[records[o + kind]]);
  return named;
}

/** The kind an area of `kind` is drawn as on a map whose areas name `named`.
 *  A map that names no kind but `road` was drawn before roads had kinds: its
 *  roads are country roads, and are drawn as one until its map says so. */
export function drawnKind(
  kind: SurfaceAreaKind,
  named: ReadonlySet<SurfaceAreaKind>,
): SurfaceAreaKind {
  const unkinded = [...named].every((k) => k === "road");
  return kind === "road" && unkinded ? "country_road" : kind;
}

/** Whether a paved record's kind `tag` is a carriageway. */
export const isRoad = (tag: number) => ROAD_AREA_KINDS.includes(SURFACE_AREA_KINDS[tag]);

/** Agricultural guides read the road strokes' authored runs and the rivers'
 * long runs (so a field meets a rounded bend edge-on, not its short samples) and
 * the native exposed road boundary. Sidewalks and triangulation diagonals
 * never become field boundaries. */
export function plotGuideEdges(site: SurfaceGeometry): Float32Array {
  const boundaries = site.surfaceBoundaries;
  let count =
    site.surfaceRuns.length / site.surfaceRunStride + site.riverRuns.length / site.riverRunStride;
  for (let o = 0; o < boundaries.length; o += site.surfaceBoundaryStride)
    if (isRoad(boundaries[o + 4])) count++;
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
    if (isRoad(boundaries[o + 4])) {
      edges.set(boundaries.subarray(o, o + 4), at);
      at += 4;
    }
  return edges;
}
