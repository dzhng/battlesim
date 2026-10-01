// The static chunk path: placed instances of a few appearances (trees and
// hedgerow shrubs, a town's massing boxes), bucketed in square chunks, with
// the detail tier each draws at chosen on the CPU when the view changes.
// An instance's tier comes from its projected height in pixels (the biome's
// `lod_px`), as ~/dev/game's sceneryDetail chooses leaf detail (technique):
// placement, not upload order, decides.
//
// A chunk too far for any of its instances to exceed the coarsest threshold
// draws whole at tier 3, straight from a static buffer in chunk order, so the
// kilometres of hedgerow past the map cost no per-instance work; neighbouring
// far chunks merge into one draw range. A nearer chunk sorts each instance
// into a per-tier staging list.
//
// Only chunks inside the camera's side planes are drawn into the view. A
// population that casts sun shadows also lists, for the cascades, every chunk
// whose shadow can land in view: its box swept along the shadow's fall, where
// the sun's shadows are still received. So a forest's cost follows what the
// camera sees, not the map.
import { vec3, type Vec3 } from "math";
import { box3, frustum, type Box3 } from "math/shapes";
import type { DetailView } from "../frame/detailView";
import { TREE_FIELD, TREE_FLOATS, type KindSize } from "./placement";

export const TIER_COUNT = 4;
/** Floats per GPU instance: pose (x, y, z, yaw), shape (scale x, y, z,
 *  heart), tint (r, g, b, seed). The heart is the height above the foot
 *  where a whole tree probes fog; the seed offsets its leaf noise. */
export const INSTANCE_FLOATS = 12;
/** A crown's heart, as a fraction of the tree's height. */
const HEART = 0.62;

/** Placed instances as the chunk path takes them: the GPU's records, and
 *  what bucketing and tier selection read of each. */
export interface PlacedInstances {
  /** `INSTANCE_FLOATS` per instance. */
  records: Float32Array;
  /** Per instance: which appearance it draws. */
  kinds: Uint16Array;
  /** Per instance: its drawn height, metres. */
  heights: Float32Array;
  /** Per instance: its farthest horizontal reach from its origin, metres. */
  reaches: Float32Array;
}

export function createPlacedInstances(count: number): PlacedInstances {
  return {
    records: new Float32Array(count * INSTANCE_FLOATS),
    kinds: new Uint16Array(count),
    heights: new Float32Array(count),
    reaches: new Float32Array(count),
  };
}

/** Placed trees (`TREE_FLOATS` each, of `sizes.length` kinds) as instances. */
export function treeInstances(placed: Float32Array, sizes: readonly KindSize[]): PlacedInstances {
  const count = placed.length / TREE_FLOATS;
  const out = createPlacedInstances(count);
  for (let i = 0; i < count; i++) {
    const o = i * TREE_FLOATS;
    const kind = placed[o + TREE_FIELD.kind];
    const [x, y] = [placed[o + TREE_FIELD.x], placed[o + TREE_FIELD.y]];
    const [sxy, sz] = [placed[o + TREE_FIELD.scaleXY], placed[o + TREE_FIELD.scaleZ]];
    const h = sizes[kind].height * sz;
    out.kinds[i] = kind;
    out.heights[i] = h;
    out.reaches[i] = sizes[kind].radius * sxy;
    out.records.set(
      [
        x,
        y,
        placed[o + TREE_FIELD.z],
        placed[o + TREE_FIELD.yaw],
        sxy,
        sxy,
        sz,
        HEART * h,
        placed[o + TREE_FIELD.r],
        placed[o + TREE_FIELD.g],
        placed[o + TREE_FIELD.b],
        // A per-tree noise offset, so neighbours' leaf clumps differ.
        (x * 0.618 + y * 0.382) % 97,
      ],
      i * INSTANCE_FLOATS,
    );
  }
  return out;
}

interface Chunk {
  box: Box3;
  /** The tallest instance in the chunk, metres. */
  height: number;
  /** Per kind: the chunk's instances in `sorted[kind]`, `[start, end)`. */
  start: number[];
  end: number[];
}

/** A population's instances, bucketed for tier selection. */
export interface TierPopulation {
  kinds: number;
  /** Whether the population casts sun shadows (`cast` is filled). */
  casts: boolean;
  /** Per kind: GPU instances in chunk order (the static far buffer's contents). */
  sorted: Float32Array<ArrayBuffer>[];
  /** Per kind: each sorted instance's drawn height, metres. */
  heights: Float32Array[];
  chunks: Chunk[];
  /** Per kind and tier (0–3): this frame's instances drawn from staging. */
  staging: Float32Array<ArrayBuffer>[][];
  counts: number[][];
  /** Per kind: this frame's merged tier-3 ranges in `sorted`, `[first, count]` pairs. */
  far: number[][];
  /** Per kind: the ranges drawn into the cascades from `sorted`: `far`'s, and
   *  every chunk out of view whose shadow can land in it. The staged tiers
   *  cast too. */
  cast: number[][];
}

/** Bucket `placed` instances of `kinds` appearances in `chunkM` chunks. */
export function createTierPopulation(
  placed: PlacedInstances,
  kinds: number,
  chunkM: number,
  casts: boolean,
): TierPopulation {
  const { records } = placed;
  const count = placed.kinds.length;
  const cells = new Map<number, number[]>();
  for (let i = 0; i < count; i++) {
    const o = i * INSTANCE_FLOATS;
    const cx = Math.floor(records[o] / chunkM);
    const cy = Math.floor(records[o + 1] / chunkM);
    // Row-major ids, so neighbours in a row are neighbours in memory.
    const id = (cy + 4096) * 8192 + (cx + 4096);
    let list = cells.get(id);
    if (!list) cells.set(id, (list = []));
    list.push(i);
  }
  const ids = [...cells.keys()].sort((a, b) => a - b);
  const perKind = Array.from({ length: kinds }, () => 0);
  for (let i = 0; i < count; i++) perKind[placed.kinds[i]]++;
  const sorted = perKind.map((n) => new Float32Array(n * INSTANCE_FLOATS));
  const heights = perKind.map((n) => new Float32Array(n));
  const fill = Array.from({ length: kinds }, () => 0);
  const chunks: Chunk[] = [];
  for (const id of ids) {
    const box = box3.create();
    const chunk: Chunk = { box, height: 0, start: [...fill], end: [] };
    for (const i of cells.get(id)!) {
      const o = i * INSTANCE_FLOATS;
      const kind = placed.kinds[i];
      const [h, r] = [placed.heights[i], placed.reaches[i]];
      const [x, y, z] = [records[o], records[o + 1], records[o + 2]];
      box3.expandByPoint(box, box, [x - r, y - r, z]);
      box3.expandByPoint(box, box, [x + r, y + r, z + h]);
      chunk.height = Math.max(chunk.height, h);
      const at = fill[kind]++;
      heights[kind][at] = h;
      sorted[kind].set(records.subarray(o, o + INSTANCE_FLOATS), at * INSTANCE_FLOATS);
    }
    chunk.end = [...fill];
    chunks.push(chunk);
  }
  return {
    kinds,
    casts,
    sorted,
    heights,
    chunks,
    staging: sorted.map((s) =>
      Array.from({ length: TIER_COUNT }, () => new Float32Array(s.length)),
    ),
    counts: sorted.map(() => Array.from({ length: TIER_COUNT }, () => 0)),
    far: sorted.map(() => []),
    cast: sorted.map(() => []),
  };
}

/** What the camera sees this frame, for tier selection. */
export interface TierView extends DetailView {
  /** Tier thresholds in projected pixels (`biome.trees.lod_px`). */
  lodPx: readonly [number, number, number];
  /** The sun's shadows: how far a shadow's tip falls from its caster's foot,
   *  metres across the ground per metre of height, and the farthest distance
   *  from the eye at which a shadow is still received. */
  shadow: { fall: readonly [number, number]; reach: number };
}

const _tier_centre = vec3.create();
const _tier_closest = vec3.create();
const _tier_corner = vec3.create();
const _tier_swept = box3.create();

/** Append a chunk's instances of kind `k` to merged `[first, count]` ranges. */
function appendRange(ranges: number[], first: number, end: number) {
  if (end === first) return;
  const last = ranges.length - 2;
  if (last >= 0 && ranges[last] + ranges[last + 1] === first) ranges[last + 1] += end - first;
  else ranges.push(first, end - first);
}

/** The eye's distance to the nearest point of `box`. */
function distanceTo(box: Box3, eye: Vec3): number {
  vec3.max(_tier_closest, eye, box3.min(_tier_corner, box));
  vec3.min(_tier_closest, _tier_closest, box3.max(_tier_corner, box));
  return vec3.distance(_tier_closest, eye);
}

/** Whether a chunk out of view can cast into it: its box, swept along the
 *  fall of its tallest instance's shadow, meets the view within the reach. */
function castsIntoView(chunk: Chunk, view: TierView): boolean {
  const [fx, fy] = [view.shadow.fall[0] * chunk.height, view.shadow.fall[1] * chunk.height];
  const b = chunk.box;
  box3.set(
    _tier_swept,
    b[0] + Math.min(fx, 0),
    b[1] + Math.min(fy, 0),
    b[2],
    b[3] + Math.max(fx, 0),
    b[4] + Math.max(fy, 0),
    b[5],
  );
  return (
    distanceTo(_tier_swept, view.eye) <= view.shadow.reach &&
    frustum.sidesIntersectsBox3(view.sides, _tier_swept)
  );
}

/** The tier an instance of height `h` draws at from `distance` metres. */
export function tierFor(h: number, distance: number, view: TierView): number {
  const px = (h * view.pixelsPerMetre) / Math.max(distance, 1e-3);
  return px > view.lodPx[0] ? 0 : px > view.lodPx[1] ? 1 : px > view.lodPx[2] ? 2 : 3;
}

/** Sort this frame's instances into tiers: `staging`/`counts`, `far` and `cast`. */
export function selectTiers(pop: TierPopulation, view: TierView): void {
  for (let k = 0; k < pop.kinds; k++) {
    pop.counts[k].fill(0);
    pop.far[k].length = 0;
    pop.cast[k].length = 0;
  }
  for (const chunk of pop.chunks) {
    if (!frustum.sidesIntersectsBox3(view.sides, chunk.box)) {
      if (pop.casts && castsIntoView(chunk, view))
        for (let k = 0; k < pop.kinds; k++) appendRange(pop.cast[k], chunk.start[k], chunk.end[k]);
      continue;
    }
    const near = distanceTo(chunk.box, view.eye);
    if (tierFor(chunk.height, near, view) === 3) {
      const casts = pop.casts && near <= view.shadow.reach;
      for (let k = 0; k < pop.kinds; k++) {
        appendRange(pop.far[k], chunk.start[k], chunk.end[k]);
        if (casts) appendRange(pop.cast[k], chunk.start[k], chunk.end[k]);
      }
      continue;
    }
    for (let k = 0; k < pop.kinds; k++) {
      const sorted = pop.sorted[k];
      for (let i = chunk.start[k]; i < chunk.end[k]; i++) {
        const o = i * INSTANCE_FLOATS;
        const h = pop.heights[k][i];
        vec3.set(_tier_centre, sorted[o], sorted[o + 1], sorted[o + 2] + h / 2);
        const tier = tierFor(h, vec3.distance(_tier_centre, view.eye), view);
        const at = pop.counts[k][tier]++;
        pop.staging[k][tier].set(sorted.subarray(o, o + INSTANCE_FLOATS), at * INSTANCE_FLOATS);
      }
    }
  }
}
