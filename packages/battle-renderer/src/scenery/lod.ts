// Scenery as the static chunk owner (`frame/staticChunks.ts`) draws it:
// placed instances of a few appearances (trees and hedgerow shrubs, a town's
// massing boxes), their GPU record, and the rule for the detail tier each
// draws at. An instance's tier comes from its projected height in pixels (the
// biome's `lod_px`), as ~/dev/game's sceneryDetail chooses leaf detail
// (technique): placement, not upload order, decides.
//
// A chunk too far for any of its instances to exceed the coarsest threshold
// draws whole at the last tier; a nearer chunk's instances each take their
// own.
import { vec3 } from "math";
import { box3 } from "math/shapes";
import type { DetailView } from "../frame/detailView";
import {
  createStaticChunks,
  NEAR,
  type Chunk,
  type StaticChunks,
  type SunShadow,
} from "../frame/staticChunks";
import { TREE_FIELD, TREE_FLOATS, type KindSize } from "./placement";

export const TIER_COUNT = 4;
/** Floats per GPU instance: pose (x, y, z, yaw), shape (scale x, y, z,
 *  heart), tint (r, g, b, seed). The heart is the height above the foot
 *  where a whole tree probes fog; the seed offsets its leaf noise. */
export const INSTANCE_FLOATS = 12;
/** A crown's heart, as a fraction of the tree's height. */
const HEART = 0.62;

/** Placed scenery: the GPU's records, and what bucketing and tier selection
 *  read of each. */
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

const _scenery_corner = vec3.create();

/** Bucket `placed` instances of `kinds` appearances in `chunkM` chunks: each
 *  stands on its foot, its reach about it and its height above. */
export function sceneryChunks(
  placed: PlacedInstances,
  kinds: number,
  chunkM: number,
): StaticChunks {
  const { records, heights, reaches } = placed;
  return createStaticChunks(
    {
      records,
      stride: INSTANCE_FLOATS,
      kinds: placed.kinds,
      sizes: heights,
      bound(i, box) {
        const o = i * INSTANCE_FLOATS;
        const [x, y, z] = [records[o], records[o + 1], records[o + 2]];
        const r = reaches[i];
        box3.expandByPoint(box, box, vec3.set(_scenery_corner, x - r, y - r, z));
        box3.expandByPoint(box, box, vec3.set(_scenery_corner, x + r, y + r, z + heights[i]));
      },
    },
    kinds,
    chunkM,
    TIER_COUNT,
  );
}

/** What the camera sees this frame, for tier selection. */
export interface TierView extends DetailView {
  /** Tier thresholds in projected pixels (`biome.trees.lod_px`). */
  lodPx: readonly [number, number, number];
  shadow: SunShadow;
}

/** The tier an instance of height `h` draws at from `distance` metres. */
export function tierFor(h: number, distance: number, view: TierView): number {
  const px = (h * view.pixelsPerMetre) / Math.max(distance, 1e-3);
  return px > view.lodPx[0] ? 0 : px > view.lodPx[1] ? 1 : px > view.lodPx[2] ? 2 : 3;
}

/** The last tier for a chunk whose tallest instance would take it from the
 *  chunk's nearest point; any nearer chunk's instances choose their own. */
export function chunkTier(chunk: Chunk, distance: number, view: TierView): number {
  return tierFor(chunk.size, distance, view) === TIER_COUNT - 1 ? TIER_COUNT - 1 : NEAR;
}
