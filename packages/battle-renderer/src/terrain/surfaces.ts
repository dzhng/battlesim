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

/** How a paved kind's strokes are drawn through built ground: as kind `as`
 *  (its tag) wherever that ground lies `besideM` beyond the stroke's edge on
 *  both sides, and on across any gap in it shorter than `gapM`. Its surface
 *  runs `carryM` on under the stroke's own, either end. */
export interface StrokeThroughBuilt {
  as: number;
  besideM: number;
  gapM: number;
  carryM: number;
}

/** Built ground is looked for at points this far apart along a stretch, and
 *  a change between two of them is placed this closely. */
const BUILT_STEP_M = 4;
const BUILT_PLACED_M = 0.25;

/** `site`'s paved strokes as the ground draws them. A stroke of a kind that
 *  `through` (by tag; null for a kind that is always itself) draws as another
 *  is that kind along each run where `built(x, y)` holds either side of it:
 *  its own stretches stop at the run, round as a stroke's stretches meet,
 *  and the run is laid as the other kind on the same line and as wide, cut
 *  square a carry past each end, under the stroke's own surface. No edge of
 *  the road moves. Where nothing is drawn differently the result is
 *  `site.surfaceStrokes` itself. */
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
    if (runs.length === 0) {
      for (let k = o; k < end; k++) out.push(strokes[k]);
      o = end;
      continue;
    }
    changed = true;
    let total = 0;
    for (let k = o; k < end; k += stride)
      total += Math.hypot(strokes[k + 2] - strokes[k], strokes[k + 3] - strokes[k + 1]);
    /** The stroke's stretches within `spans` of its length, as `kind`: a
     *  part that ends with its span short of the stroke's own end is cut
     *  square there (`square`) or left round. */
    const lay = (spans: [number, number][], kind: number, square: boolean) => {
      let from = 0;
      for (let k = o; k < end; k += stride) {
        const [ax, ay, bx, by, half, , cuts] = strokes.subarray(k, k + STROKE_FLOATS);
        const length = Math.hypot(bx - ax, by - ay);
        const point = (at: number) =>
          at === from
            ? [ax, ay]
            : at === from + length
              ? [bx, by]
              : [
                  Math.fround(ax + ((bx - ax) * (at - from)) / length),
                  Math.fround(ay + ((by - ay) * (at - from)) / length),
                ];
        for (const [first, last] of spans) {
          const [start, stop] = [Math.max(first, from), Math.min(last, from + length)];
          if (stop <= start) continue;
          const cut =
            (start === from ? cuts & CUT_A : 0) |
            (stop === from + length ? cuts & CUT_B : 0) |
            (square && start === first && first > 0 ? CUT_A : 0) |
            (square && stop === last && last < total ? CUT_B : 0);
          out.push(...point(start), ...point(stop), half, kind, cut);
        }
        from += length;
      }
    };
    // The stroke as itself between the runs, then each run with its carry.
    const between: [number, number][] = [];
    const carried: [number, number][] = [];
    let at = 0;
    for (const [start, stop] of runs) {
      between.push([at, start]);
      at = stop;
      const span: [number, number] = [
        Math.max(0, start - rule!.carryM),
        Math.min(total, stop + rule!.carryM),
      ];
      const last = carried.at(-1);
      if (last && span[0] <= last[1]) last[1] = span[1];
      else carried.push(span);
    }
    between.push([at, total]);
    lay(between, tag, false);
    lay(carried, rule!.as, true);
    o = end;
  }
  return changed ? Float32Array.from(out) : strokes;
}

/** The runs of one stroke (its stretches, `strokes[from..to)`) with built
 *  ground either side of them, as `[start, stop]` distances along the stroke; two
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
      return built(x - uy * aside, y + ux * aside) && built(x + uy * aside, y - ux * aside);
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
