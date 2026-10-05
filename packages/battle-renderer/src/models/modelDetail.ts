// How much of a model the frame draws: nothing (off screen), one of its four
// mesh tiers, or its impostor card, chosen on the CPU from its projected
// height in pixels (`presentation.models`). Rewritten from reading
// ~/dev/game's crowd tiering (technique): placement and the camera decide,
// not upload order.
//
// Corpses number up to their presentation cap (`presentation.pose.corpses`,
// a thousand), so they are a population of the static chunk owner
// (`frame/staticChunks.ts`), bucketed once, when the list changes: a chunk
// off screen is skipped whole, and a chunk too far for any corpse in it to
// exceed the impostor size draws as one range of cards from a static buffer.
// A map's props stand for the whole battle and number in the tens of
// thousands, so they are a population too, visited by chunk in view.
// This file says only what a corpse's or a prop's bounds and a chunk's level are.
import { vec3 } from "math";
import { box3, frustum, type Sphere } from "math/shapes";
import type { DetailView } from "../frame/detailView";
import {
  createStaticChunks,
  NEAR,
  selectChunks,
  type ChunkLevel,
  type StaticChunks,
} from "../frame/staticChunks";

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

const _corpse_corner = vec3.create();

/**
 * Corpses as a chunked population of one kind, drawn at a mesh tier or as
 * cards (`IMPOSTOR`): `records` hold `stride` floats a corpse, its position
 * first, and corpse `i` is `sizes[i]` metres long. A corpse's bounds reach
 * its length and the shadow margin every way.
 */
export function corpseChunks(
  records: Float32Array,
  stride: number,
  sizes: Float32Array,
): StaticChunks {
  return createStaticChunks(
    {
      records,
      stride,
      kinds: new Uint16Array(sizes.length),
      sizes,
      bound(i, box) {
        const o = i * stride;
        const [x, y, z] = [records[o], records[o + 1], records[o + 2]];
        const r = sizes[i] + SHADOW_MARGIN_M;
        box3.expandByPoint(box, box, vec3.set(_corpse_corner, x - r, y - r, z - r));
        box3.expandByPoint(box, box, vec3.set(_corpse_corner, x + r, y + r, z + r));
      },
    },
    1,
    CORPSE_CHUNK_M,
    IMPOSTOR + 1,
  );
}

/** Square chunks a map's static models are bucketed in, metres. */
export const STATIC_CHUNK_M = 64;

/** A static model where `modelDetail` tests it: its foot, how tall it is
 *  and the radius it is drawn within. */
export interface StaticBounds {
  x: number;
  y: number;
  z: number;
  size: number;
  radius: number;
}

const _static_corner = vec3.create();
const everyNear: ChunkLevel<DetailView> = () => NEAR;

/**
 * A map's static models (props, standing for the whole battle) bucketed by
 * ground, so a frame visits only the chunks in view. A chunk's box holds each
 * model's sphere as `modelDetail` tests it, shadow margin and all, so a model
 * in a chunk out of view is one `modelDetail` culls.
 */
export function staticModelChunks(statics: readonly StaticBounds[]): StaticChunks {
  const records = new Float32Array(statics.length * 3);
  const sizes = new Float32Array(statics.length);
  statics.forEach((s, i) => {
    records.set([s.x, s.y, s.z], i * 3);
    sizes[i] = s.size;
  });
  return createStaticChunks(
    {
      records,
      stride: 3,
      kinds: new Uint16Array(statics.length),
      sizes,
      bound(i, box) {
        const s = statics[i];
        const r = s.radius + SHADOW_MARGIN_M;
        const z = s.z + s.size / 2;
        box3.expandByPoint(box, box, vec3.set(_static_corner, s.x - r, s.y - r, z - r));
        box3.expandByPoint(box, box, vec3.set(_static_corner, s.x + r, s.y + r, z + r));
      },
    },
    1,
    STATIC_CHUNK_M,
    1,
  );
}

/** Write into `out` the static models in the chunks `view` meets, as indices
 *  into the list they were chunked from, in that list's order; answer how many. */
export function staticsInView(pop: StaticChunks, view: DetailView, out: Int32Array): number {
  selectChunks(pop, view, everyNear, null);
  let count = 0;
  const order = pop.order[0];
  for (const c of pop.near) {
    const chunk = pop.chunks[c];
    for (let i = chunk.start[0]; i < chunk.end[0]; i++) out[count++] = order[i];
  }
  out.subarray(0, count).sort();
  return count;
}

/** A corpse chunk's level: cards for the whole chunk when every corpse in it
 *  has one (`carded`, per chunk) and its largest is below the impostor size
 *  from the chunk's nearest point; else each corpse chooses (`modelDetail`). */
export function corpseChunkLevel(
  detail: ModelDetailPresentation,
  carded: readonly boolean[],
): ChunkLevel<DetailView> {
  return (chunk, distance, view, index) =>
    carded[index] && detailAt(detail, view, chunk.size, distance, true) === IMPOSTOR
      ? IMPOSTOR
      : NEAR;
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
