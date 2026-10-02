import { CUT_A, CUT_B, STROKE_FLOATS } from "./strokes";

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

/** How a paved kind's strokes are drawn through built ground: as kind `as`
 *  (its tag) wherever that ground lies `besideM` beyond the stroke's edge, on
 *  either side, and on across any gap in it shorter than `gapM`. */
export interface StrokeThroughBuilt {
  as: number;
  besideM: number;
  gapM: number;
}

/** Built ground is looked for at points this far apart along a stretch, and
 *  a change between two of them is placed this closely. */
const BUILT_STEP_M = 4;
const BUILT_PLACED_M = 0.25;

/** `site`'s paved strokes as the ground draws them. A stroke of a kind that
 *  `through` (by tag; null for a kind that is always itself) draws as another
 *  is tagged as that kind along each run where `built(x, y)` holds beside
 *  it, and its stretches are split where a run starts and ends. Each
 *  piece starts where the last ended, on its stretch's own line and as wide.
 *  A run ends cut square, and the road it turns back into starts round over
 *  that end, so no edge of the road moves. Where nothing is drawn
 *  differently the result is `site.surfaceStrokes` itself. */
export function drawnStrokes(
  site: SurfaceGeometry,
  through: readonly (StrokeThroughBuilt | null)[],
  built: (x: number, y: number) => boolean,
): Float32Array {
  const strokes = site.surfaceStrokes;
  const stride = site.surfaceStrokeStride;
  const out: number[] = [];
  let changed = false;
  for (let o = 0; o < strokes.length; ) {
    // A stroke's stretches are exported in order, each starting where the
    // last ended.
    let end = o + stride;
    while (
      end < strokes.length &&
      strokes[end] === strokes[end - stride + 2] &&
      strokes[end + 1] === strokes[end - stride + 3] &&
      strokes[end + 5] === strokes[o + 5]
    )
      end += stride;
    const tag = strokes[o + 5];
    const rule = through[tag];
    const runs = rule && rule.as !== tag ? builtRuns(strokes, o, end, stride, rule, built) : [];
    let total = 0;
    for (let k = o; k < end; k += stride)
      total += Math.hypot(strokes[k + 2] - strokes[k], strokes[k + 3] - strokes[k + 1]);
    if (runs.length === 0) {
      for (let k = o; k < end; k++) out.push(strokes[k]);
      o = end;
      continue;
    }
    changed = true;
    let from = 0;
    for (; o < end; o += stride) {
      const [ax, ay, bx, by, half, , cuts] = strokes.subarray(o, o + STROKE_FLOATS);
      const length = Math.hypot(bx - ax, by - ay);
      // The stretch's pieces: it is split at each run's end that falls
      // inside it.
      const ends = [
        from,
        ...runs.flat().filter((at) => at > from && at < from + length),
        from + length,
      ];
      for (let k = 0; k + 1 < ends.length; k++) {
        const [start, stop] = [ends[k], ends[k + 1]];
        const run = runs.find((r) => r[0] <= start && stop <= r[1]);
        const point = (at: number) =>
          at === from
            ? [ax, ay]
            : at === from + length
              ? [bx, by]
              : [
                  Math.fround(ax + ((bx - ax) * (at - from)) / length),
                  Math.fround(ay + ((by - ay) * (at - from)) / length),
                ];
        // A run ends square across the road, where the road it turns back
        // into starts round over it; the stretch's own ends stay as cut.
        const cut =
          (k === 0 ? cuts & CUT_A : 0) |
          (k + 2 === ends.length ? cuts & CUT_B : 0) |
          (run && run[0] === start && start > 0 ? CUT_A : 0) |
          (run && run[1] === stop && stop < total ? CUT_B : 0);
        out.push(...point(start), ...point(stop), half, run ? rule!.as : tag, cut);
      }
      from += length;
    }
  }
  return changed ? Float32Array.from(out) : strokes;
}

/** The runs of one stroke (its stretches, `strokes[from..to)`) with built
 *  ground beside them, as `[start, stop]` distances along the stroke; two
 *  runs nearer than the rule's gap are one. */
function builtRuns(
  strokes: Float32Array,
  from: number,
  to: number,
  stride: number,
  rule: StrokeThroughBuilt,
  built: (x: number, y: number) => boolean,
): [number, number][] {
  const runs: [number, number][] = [];
  /** A run opens or closes `at`, as `is` says. */
  const turn = (at: number, is: boolean) => {
    const last = runs.at(-1);
    if (!is) last![1] = at;
    else if (last && at - last[1] < rule.gapM) last[1] = Infinity;
    else runs.push([at, Infinity]);
  };
  let was = false;
  let before = 0;
  for (let o = from; o < to; o += stride) {
    const [ax, ay, bx, by, half] = strokes.subarray(o, o + 5);
    const length = Math.hypot(bx - ax, by - ay);
    if (length === 0) continue;
    const [ux, uy] = [(bx - ax) / length, (by - ay) / length];
    const aside = half + rule.besideM;
    const builtAt = (along: number) => {
      const [x, y] = [ax + ux * along, ay + uy * along];
      return built(x - uy * aside, y + ux * aside) || built(x + uy * aside, y - ux * aside);
    };
    const steps = Math.ceil(length / BUILT_STEP_M);
    for (let s = 0; s <= steps; s++) {
      const at = (length * s) / steps;
      const is = builtAt(at);
      if (is === was) continue;
      // Between this point and the last; at a stretch's start, there.
      let [low, high] = [s === 0 ? at : (length * (s - 1)) / steps, at];
      while (high - low > BUILT_PLACED_M) {
        const middle = (low + high) / 2;
        if (builtAt(middle) === was) low = middle;
        else high = middle;
      }
      turn(before + (low + high) / 2, is);
      was = is;
    }
    before += length;
  }
  if (was) runs.at(-1)![1] = before;
  return runs;
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
