// How much of a model the frame draws: nothing (off screen), one of its four
// mesh tiers, or its impostor card, chosen on the CPU from its projected
// height in pixels (`presentation.models`). Rewritten from reading
// ~/dev/game's crowd tiering (technique): placement and the camera decide,
// not upload order.
//
// Corpses number up to their presentation cap (`presentation.pose.corpses`,
// a thousand), so they are bucketed in square chunks once, when the list changes: a chunk
// off screen is skipped whole, and a chunk too far for any corpse in it to
// exceed the impostor size draws as one range of cards from a static buffer.
import { vec3 } from "math";
import { box3, frustum, type Box3, type Sphere } from "math/shapes";
import type { DetailView } from "../frame/detailView";

/** `presentation.models`: the detail tiers by projected height, in device pixels. */
export interface ModelDetailPresentation {
  /** Above `lod_px[0]` tier 0, above `[1]` tier 1, above `[2]` tier 2, else tier 3. */
  lod_px: [number, number, number];
  /** Below this a model with an impostor draws as its card. */
  impostor_px: number;
}

export const CULLED = -1;
export const IMPOSTOR = 4;

/** A model's sun shadow reaches this far past it: models this close outside
 *  the view still draw, into the cascades, so shadows never pop at the edge. */
export const SHADOW_MARGIN_M = 3;

/** The tier (0–3) or `IMPOSTOR` for a model `size` metres tall at `distance`. */
export function detailAt(
  detail: ModelDetailPresentation,
  view: DetailView,
  size: number,
  distance: number,
  impostor: boolean,
): number {
  const px = (size * view.pixelsPerMetre) / Math.max(distance, 1e-3);
  const [t0, t1, t2] = detail.lod_px;
  if (px > t0) return 0;
  if (px > t1) return 1;
  if (px > t2) return 2;
  return impostor && px < detail.impostor_px ? IMPOSTOR : 3;
}

const _detail_sphere: Sphere = { center: vec3.create(), radius: 0 };

/** `CULLED`, a tier, or `IMPOSTOR` for a model standing at (x, y, z), `size`
 *  metres tall and within `radius` of its centre (half its size up). */
export function modelDetail(
  detail: ModelDetailPresentation,
  view: DetailView,
  x: number,
  y: number,
  z: number,
  size: number,
  radius: number,
  impostor: boolean,
): number {
  const c = _detail_sphere.center;
  vec3.set(c, x, y, z + size / 2);
  _detail_sphere.radius = radius + SHADOW_MARGIN_M;
  if (!frustum.sidesIntersectsSphere(view.sides, _detail_sphere)) return CULLED;
  return detailAt(detail, view, size, vec3.distance(c, view.eye), impostor);
}

/** Square chunks corpses are bucketed in, metres. */
export const CORPSE_CHUNK_M = 64;

export interface CorpseChunk {
  /** Every corpse in it, grown by its reach and the shadow margin. */
  box: Box3;
  /** The largest corpse's size, metres. */
  size: number;
  /** Its corpses in chunk order, `[start, end)`. */
  start: number;
  end: number;
}

/**
 * Chunk order for `count` corpses at `positions` (x, y, z per corpse), each
 * `sizes[i]` metres across: `order[k]` is the corpse at chunk position k.
 */
export function chunkCorpses(
  positions: Float32Array,
  sizes: Float32Array,
  count: number,
): { order: Int32Array; chunks: CorpseChunk[] } {
  const cells = new Map<number, number[]>();
  for (let i = 0; i < count; i++) {
    const cx = Math.floor(positions[i * 3] / CORPSE_CHUNK_M);
    const cy = Math.floor(positions[i * 3 + 1] / CORPSE_CHUNK_M);
    // Row-major ids, so neighbours in a row are neighbours in memory.
    const id = (cy + 4096) * 8192 + (cx + 4096);
    let list = cells.get(id);
    if (!list) cells.set(id, (list = []));
    list.push(i);
  }
  const order = new Int32Array(count);
  const chunks: CorpseChunk[] = [];
  let at = 0;
  for (const id of [...cells.keys()].sort((a, b) => a - b)) {
    const box = box3.create();
    box3.empty(box);
    let size = 0;
    const start = at;
    for (const i of cells.get(id)!) {
      const [x, y, z] = [positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]];
      const r = sizes[i] + SHADOW_MARGIN_M;
      box3.expandByPoint(box, box, [x - r, y - r, z - r]);
      box3.expandByPoint(box, box, [x + r, y + r, z + r]);
      size = Math.max(size, sizes[i]);
      order[at++] = i;
    }
    chunks.push({ box, size, start, end: at });
  }
  return { order, chunks };
}

const _chunk_closest = vec3.create();
const _chunk_corner = vec3.create();

/** Whether every corpse in `chunk` is below the impostor size from `view`. */
export function chunkIsFar(
  detail: ModelDetailPresentation,
  view: DetailView,
  chunk: CorpseChunk,
): boolean {
  // The chunk's point nearest the eye: the eye clamped into its box.
  vec3.max(_chunk_closest, view.eye, box3.min(_chunk_corner, chunk.box));
  vec3.min(_chunk_closest, _chunk_closest, box3.max(_chunk_corner, chunk.box));
  const near = vec3.distance(_chunk_closest, view.eye);
  return detailAt(detail, view, chunk.size, near, true) === IMPOSTOR;
}

/** `presentation.models`, checked: tiers strictly finer toward the camera,
 *  the impostor size under the coarsest tier's. */
export function validateModelDetail(detail: ModelDetailPresentation): ModelDetailPresentation {
  const [t0, t1, t2] = detail.lod_px;
  if (!(t0 > t1 && t1 > t2 && t2 >= detail.impostor_px && detail.impostor_px >= 0))
    throw new Error(
      `presentation.models: lod_px must fall and end above impostor_px, got ${JSON.stringify(detail)}`,
    );
  return detail;
}
