// How much of a model the frame draws: nothing (off screen), one of its four
// mesh tiers, or its impostor card, chosen on the CPU from its projected
// height in pixels (`presentation.models`). Rewritten from reading
// ~/dev/game's crowd tiering (technique): placement and the camera decide,
// not upload order.
//
// A lab's own models are tiered here; the units, corpses, ghosts and the
// map's props by the same rule in the presentation crate
// (`crates/presentation/src/view.rs`).
import { vec3, type Vec3 } from "math";
import { frustum, type Sphere } from "math/shapes";
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
const _shadow_sphere: Sphere = { center: vec3.create(), radius: 0 };

/** `CULLED`, a tier, or `IMPOSTOR` for a model standing at (x, y, z), `size`
 *  metres tall and within `radius` of its centre (half its size up). A
 *  model in the air (an airframe) casts its shadow far from itself: given
 *  where that lands (`shadowAt`, the shadow of its foot), it draws while
 *  either is in view, so its shadow never pops at the screen's edge. */
export function modelDetail(
  detail: ModelDetailPresentation,
  view: DetailView,
  x: number,
  y: number,
  z: number,
  size: number,
  radius: number,
  impostor: boolean,
  shadowAt: Readonly<Vec3> | null = null,
): number {
  const c = _detail_sphere.center;
  vec3.set(c, x, y, z + size / 2);
  _detail_sphere.radius = radius + SHADOW_MARGIN_M;
  if (!frustum.sidesIntersectsSphere(view.sides, _detail_sphere)) {
    if (!shadowAt) return CULLED;
    vec3.set(_shadow_sphere.center, shadowAt[0], shadowAt[1], shadowAt[2] + size / 2);
    _shadow_sphere.radius = _detail_sphere.radius;
    if (!frustum.sidesIntersectsSphere(view.sides, _shadow_sphere)) return CULLED;
  }
  return detailAt(detail, view, size, vec3.distance(c, view.eye), impostor);
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
