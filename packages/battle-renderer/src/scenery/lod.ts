// Which detail tier each placed tree draws at, chosen on the CPU per frame.
// A tree's tier comes from its projected height in pixels (the biome's
// `lod_px`), as ~/dev/game's sceneryDetail chooses leaf detail (technique):
// placement, not upload order, decides.
//
// Trees are bucketed in square chunks. A chunk too far for any of its trees
// to exceed the coarsest threshold draws whole at tier 3, straight from a
// static buffer in chunk order, so the kilometres of hedgerow past the map
// cost no per-tree work; neighbouring far chunks merge into one draw range.
// A nearer chunk sorts each tree into a per-tier staging list. A population
// may be culled to the camera's side planes (scenery past the map casts no
// shadow, so nothing off screen needs it); the forest never is, since its
// trees cast shadows into view from off screen.
import { vec3 } from "math";
import { box3, frustum, type Box3 } from "math/shapes";
import type { DetailView } from "../frame/detailView";
import { TREE_FIELD, TREE_FLOATS, type KindSize } from "./placement";

export const TIER_COUNT = 4;
/** Floats per GPU instance: pose (x, y, z, yaw), shape (scaleXY, scaleZ,
 *  seed, heart), tint (r, g, b, 0). The heart is the crown's centre, metres
 *  above the foot: where the whole tree probes fog. */
export const INSTANCE_FLOATS = 12;
/** A crown's heart, as a fraction of the tree's height. */
const HEART = 0.62;

interface Chunk {
  box: Box3;
  /** The tallest tree in the chunk, metres. */
  height: number;
  /** Per kind: the chunk's instances in `sorted[kind]`, `[start, end)`. */
  start: number[];
  end: number[];
}

/** A population's trees, bucketed for tier selection. */
export interface TierPopulation {
  kinds: number;
  cull: boolean;
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
}

/** Bucket placed trees (`TREE_FLOATS` each) of `sizes.length` kinds. */
export function createTierPopulation(
  placed: Float32Array,
  sizes: readonly KindSize[],
  chunkM: number,
  cull: boolean,
): TierPopulation {
  const kinds = sizes.length;
  const count = placed.length / TREE_FLOATS;
  const cells = new Map<number, number[]>();
  for (let i = 0; i < count; i++) {
    const o = i * TREE_FLOATS;
    const cx = Math.floor(placed[o + TREE_FIELD.x] / chunkM);
    const cy = Math.floor(placed[o + TREE_FIELD.y] / chunkM);
    // Row-major ids, so neighbours in a row are neighbours in memory.
    const id = (cy + 4096) * 8192 + (cx + 4096);
    let list = cells.get(id);
    if (!list) cells.set(id, (list = []));
    list.push(i);
  }
  const ids = [...cells.keys()].sort((a, b) => a - b);
  const perKind = Array.from({ length: kinds }, () => 0);
  for (let i = 0; i < count; i++) perKind[placed[i * TREE_FLOATS + TREE_FIELD.kind]]++;
  const sorted = perKind.map((n) => new Float32Array(n * INSTANCE_FLOATS));
  const heights = perKind.map((n) => new Float32Array(n));
  const fill = Array.from({ length: kinds }, () => 0);
  const chunks: Chunk[] = [];
  for (const id of ids) {
    const box = box3.create();
    const chunk: Chunk = { box, height: 0, start: [...fill], end: [] };
    for (const i of cells.get(id)!) {
      const o = i * TREE_FLOATS;
      const kind = placed[o + TREE_FIELD.kind];
      const size = sizes[kind];
      const h = size.height * placed[o + TREE_FIELD.scaleZ];
      const r = size.radius * placed[o + TREE_FIELD.scaleXY];
      const [x, y, z] = [
        placed[o + TREE_FIELD.x],
        placed[o + TREE_FIELD.y],
        placed[o + TREE_FIELD.z],
      ];
      box3.expandByPoint(box, box, [x - r, y - r, z]);
      box3.expandByPoint(box, box, [x + r, y + r, z + h]);
      chunk.height = Math.max(chunk.height, h);
      const at = fill[kind]++;
      heights[kind][at] = h;
      sorted[kind].set(
        [
          x,
          y,
          z,
          placed[o + TREE_FIELD.yaw],
          placed[o + TREE_FIELD.scaleXY],
          placed[o + TREE_FIELD.scaleZ],
          // A per-tree noise offset, so neighbours' leaf clumps differ.
          (x * 0.618 + y * 0.382) % 97,
          HEART * h,
          placed[o + TREE_FIELD.r],
          placed[o + TREE_FIELD.g],
          placed[o + TREE_FIELD.b],
          0,
        ],
        at * INSTANCE_FLOATS,
      );
    }
    chunk.end = [...fill];
    chunks.push(chunk);
  }
  return {
    kinds,
    cull,
    sorted,
    heights,
    chunks,
    staging: sorted.map((s) =>
      Array.from({ length: TIER_COUNT }, () => new Float32Array(s.length)),
    ),
    counts: sorted.map(() => Array.from({ length: TIER_COUNT }, () => 0)),
    far: sorted.map(() => []),
  };
}

/** What the camera sees this frame, for tier selection. */
export interface TierView extends DetailView {
  /** Tier thresholds in projected pixels (`biome.trees.lod_px`). */
  lodPx: readonly [number, number, number];
}

const _tier_centre = vec3.create();
const _tier_closest = vec3.create();
const _tier_corner = vec3.create();

/** The tier a tree of height `h` draws at from `distance` metres. */
export function tierFor(h: number, distance: number, view: TierView): number {
  const px = (h * view.pixelsPerMetre) / Math.max(distance, 1e-3);
  return px > view.lodPx[0] ? 0 : px > view.lodPx[1] ? 1 : px > view.lodPx[2] ? 2 : 3;
}

/** Sort this frame's trees into tiers: `staging`/`counts` and `far`. */
export function selectTiers(pop: TierPopulation, view: TierView): void {
  for (let k = 0; k < pop.kinds; k++) {
    pop.counts[k].fill(0);
    pop.far[k].length = 0;
  }
  for (const chunk of pop.chunks) {
    if (pop.cull && !frustum.sidesIntersectsBox3(view.sides, chunk.box)) continue;
    // The chunk's point nearest the eye: the eye clamped into its box.
    vec3.max(_tier_closest, view.eye, box3.min(_tier_corner, chunk.box));
    vec3.min(_tier_closest, _tier_closest, box3.max(_tier_corner, chunk.box));
    const near = vec3.distance(_tier_closest, view.eye);
    if (tierFor(chunk.height, near, view) === 3) {
      for (let k = 0; k < pop.kinds; k++) {
        const [first, end] = [chunk.start[k], chunk.end[k]];
        if (end === first) continue;
        const far = pop.far[k];
        const last = far.length - 2;
        if (last >= 0 && far[last] + far[last + 1] === first) far[last + 1] += end - first;
        else far.push(first, end - first);
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
