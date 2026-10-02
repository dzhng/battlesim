// The static chunk owner: instances placed once and drawn many times (trees
// and hedgerow shrubs, a town's massing boxes, the fallen), bucketed in
// square chunks, with the detail level each draws at chosen on the CPU when
// the view changes. It owns the bookkeeping only: storage in chunk order,
// per-chunk culling, the level a chunk or an instance draws at, the merged
// draw ranges and the sun's casters. Records are any stride, and a level is
// whatever the population's layer draws for it (a mesh tier, an impostor
// card); buffers, meshes and materials stay with that layer.
//
// A chunk is drawn one of two ways. The population's rule may give the whole
// chunk a level: it then draws straight from the static buffer in chunk
// order, neighbouring chunks merged into one `[first, count]` range, so the
// kilometres of hedgerow past the map cost no per-instance work. Otherwise
// the chunk is near: it is listed, and its layer chooses per instance, either
// through `stageNear` (a copy of each record into its level's list) or in a
// loop of its own.
//
// Only chunks inside the camera's side planes are drawn into the view. A
// population that casts sun shadows also lists, for the cascades, every chunk
// whose shadow can land in view: its box swept along the shadow's fall, where
// the sun's shadows are still received. So a population's cost follows what
// the camera sees, not the map.
import { vec3, type Vec3 } from "math";
import { box3, frustum, type Box3 } from "math/shapes";
import type { DetailView } from "./detailView";

/** Placed instances as the owner takes them. */
export interface PlacedRecords {
  /** `stride` floats per instance, its position in the first three. */
  records: Float32Array;
  stride: number;
  /** Per instance: its kind. A kind has a static buffer and draw ranges of
   *  its own (an appearance with its own mesh). */
  kinds: Uint16Array;
  /** Per instance: the size its detail is chosen by, metres (a tree's
   *  height, a corpse's length). */
  sizes: Float32Array;
  /** Grow `box` to hold instance `i`: all of it that is drawn, and any
   *  margin that must keep it drawn (a shadow reaching into view). */
  bound(i: number, box: Box3): void;
}

export interface Chunk {
  box: Box3;
  /** The largest instance's size in the chunk, metres. */
  size: number;
  /** Per kind: the chunk's instances in `sorted[kind]`, `[start, end)`. */
  start: number[];
  end: number[];
}

/** A population's instances in chunk order, and what a view draws of them. */
export interface StaticChunks {
  stride: number;
  kinds: number;
  /** How many levels a chunk or an instance can draw at. */
  levels: number;
  /** Per kind: the records in chunk order (a static buffer's contents). */
  sorted: Float32Array<ArrayBuffer>[];
  /** Per kind: which placed instance each sorted record is. */
  order: Uint32Array[];
  /** Per kind: each sorted record's size, metres. */
  sizes: Float32Array[];
  chunks: Chunk[];
  /** Per kind and level: the chunks this view draws whole at that level, as
   *  merged `[first, count]` ranges in `sorted`. */
  ranges: number[][][];
  /** Per kind: the ranges drawn into the sun's cascades from `sorted`:
   *  `ranges`' within the shadows' reach, and every chunk out of view whose
   *  shadow can land in it. Near instances are their layer's to cast. */
  cast: number[][];
  /** The chunks in view whose instances choose their own level, by index. */
  near: number[];
}

/** Bucket `placed` instances of `kinds` kinds in `chunkM` chunks, to draw at
 *  one of `levels` levels. */
export function createStaticChunks(
  placed: PlacedRecords,
  kinds: number,
  chunkM: number,
  levels: number,
): StaticChunks {
  const { records, stride } = placed;
  const count = placed.sizes.length;
  const cells = new Map<number, number[]>();
  for (let i = 0; i < count; i++) {
    const o = i * stride;
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
  const sorted = perKind.map((n) => new Float32Array(n * stride));
  const order = perKind.map((n) => new Uint32Array(n));
  const sizes = perKind.map((n) => new Float32Array(n));
  const fill = Array.from({ length: kinds }, () => 0);
  const chunks: Chunk[] = [];
  for (const id of ids) {
    const box = box3.create();
    const chunk: Chunk = { box, size: 0, start: [...fill], end: [] };
    for (const i of cells.get(id)!) {
      const kind = placed.kinds[i];
      placed.bound(i, box);
      chunk.size = Math.max(chunk.size, placed.sizes[i]);
      const at = fill[kind]++;
      order[kind][at] = i;
      sizes[kind][at] = placed.sizes[i];
      sorted[kind].set(records.subarray(i * stride, (i + 1) * stride), at * stride);
    }
    chunk.end = [...fill];
    chunks.push(chunk);
  }
  return {
    stride,
    kinds,
    levels,
    sorted,
    order,
    sizes,
    chunks,
    ranges: sorted.map(() => Array.from({ length: levels }, () => [])),
    cast: sorted.map(() => []),
    near: [],
  };
}

/** What `ChunkLevel` answers for a chunk whose instances choose their own. */
export const NEAR = -1;

/** A population's rule for a chunk in view, `distance` metres from the eye
 *  at its nearest: the level it draws whole at, or `NEAR`. */
export type ChunkLevel<V extends DetailView> = (
  chunk: Chunk,
  index: number,
  distance: number,
  view: V,
) => number;

/** The sun's shadows: how far a shadow's tip falls from its caster's foot,
 *  metres across the ground per metre of height, and the farthest distance
 *  from the eye at which a shadow is still received. */
export interface SunShadow {
  fall: readonly [number, number];
  reach: number;
}

const _chunk_centre = vec3.create();
const _chunk_closest = vec3.create();
const _chunk_corner = vec3.create();
const _chunk_swept = box3.create();

/** Append a chunk's instances `[first, end)` to merged `[first, count]` ranges. */
function appendRange(ranges: number[], first: number, end: number) {
  if (end === first) return;
  const last = ranges.length - 2;
  if (last >= 0 && ranges[last] + ranges[last + 1] === first) ranges[last + 1] += end - first;
  else ranges.push(first, end - first);
}

/** The eye's distance to the nearest point of `box`. */
function distanceTo(box: Box3, eye: Vec3): number {
  vec3.max(_chunk_closest, eye, box3.min(_chunk_corner, box));
  vec3.min(_chunk_closest, _chunk_closest, box3.max(_chunk_corner, box));
  return vec3.distance(_chunk_closest, eye);
}

/** Whether a chunk out of view can cast into it: its box, swept along the
 *  fall of its largest instance's shadow, meets the view within the reach. */
function castsIntoView(chunk: Chunk, view: DetailView, shadow: SunShadow): boolean {
  const [fx, fy] = [shadow.fall[0] * chunk.size, shadow.fall[1] * chunk.size];
  const b = chunk.box;
  box3.set(
    _chunk_swept,
    b[0] + Math.min(fx, 0),
    b[1] + Math.min(fy, 0),
    b[2],
    b[3] + Math.max(fx, 0),
    b[4] + Math.max(fy, 0),
    b[5],
  );
  return (
    distanceTo(_chunk_swept, view.eye) <= shadow.reach &&
    frustum.sidesIntersectsBox3(view.sides, _chunk_swept)
  );
}

function clear(pop: StaticChunks) {
  for (let k = 0; k < pop.kinds; k++) {
    for (let level = 0; level < pop.levels; level++) pop.ranges[k][level].length = 0;
    pop.cast[k].length = 0;
  }
  pop.near.length = 0;
}

/** Choose what `view` draws of `pop`: `ranges`, `near` and, for a population
 *  that casts the sun's shadows (`shadow` given), `cast`. */
export function selectChunks<V extends DetailView>(
  pop: StaticChunks,
  view: V,
  levelOf: ChunkLevel<V>,
  shadow: SunShadow | null,
): void {
  clear(pop);
  for (let c = 0; c < pop.chunks.length; c++) {
    const chunk = pop.chunks[c];
    if (!frustum.sidesIntersectsBox3(view.sides, chunk.box)) {
      if (shadow && castsIntoView(chunk, view, shadow))
        for (let k = 0; k < pop.kinds; k++) appendRange(pop.cast[k], chunk.start[k], chunk.end[k]);
      continue;
    }
    const distance = distanceTo(chunk.box, view.eye);
    const level = levelOf(chunk, c, distance, view);
    if (level === NEAR) {
      pop.near.push(c);
      continue;
    }
    const casts = shadow !== null && distance <= shadow.reach;
    for (let k = 0; k < pop.kinds; k++) {
      appendRange(pop.ranges[k][level], chunk.start[k], chunk.end[k]);
      if (casts) appendRange(pop.cast[k], chunk.start[k], chunk.end[k]);
    }
  }
}

/** Draw `pop` with no view (an offline draw): every chunk is near. */
export function selectEveryChunk(pop: StaticChunks): void {
  clear(pop);
  for (let c = 0; c < pop.chunks.length; c++) pop.near.push(c);
}

/** Near instances copied into a list per level, for a layer that draws each
 *  level from a buffer of its own. */
export interface StagedLevels {
  /** Per kind and level: this view's records. */
  records: Float32Array<ArrayBuffer>[][];
  counts: number[][];
}

export function createStagedLevels(pop: StaticChunks): StagedLevels {
  return {
    records: pop.sorted.map((s) =>
      Array.from({ length: pop.levels }, () => new Float32Array(s.length)),
    ),
    counts: pop.sorted.map(() => Array.from({ length: pop.levels }, () => 0)),
  };
}

/** Sort the near chunks' instances into `staged` by the level `levelOf`
 *  gives each, from its size and its centre's distance (half its size up). */
export function stageNear<V extends DetailView>(
  pop: StaticChunks,
  staged: StagedLevels,
  view: V,
  levelOf: (size: number, distance: number, view: V) => number,
): void {
  const { stride } = pop;
  for (let k = 0; k < pop.kinds; k++) staged.counts[k].fill(0);
  for (const c of pop.near) {
    const chunk = pop.chunks[c];
    for (let k = 0; k < pop.kinds; k++) {
      const sorted = pop.sorted[k];
      for (let i = chunk.start[k]; i < chunk.end[k]; i++) {
        const o = i * stride;
        const size = pop.sizes[k][i];
        vec3.set(_chunk_centre, sorted[o], sorted[o + 1], sorted[o + 2] + size / 2);
        const level = levelOf(size, vec3.distance(_chunk_centre, view.eye), view);
        const at = staged.counts[k][level]++;
        staged.records[k][level].set(sorted.subarray(o, o + stride), at * stride);
      }
    }
  }
}
