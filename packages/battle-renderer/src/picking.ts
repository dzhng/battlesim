// Picking: which drawn thing a camera ray meets. Units are picked by the
// simulation's own boxes (a soldier's cylinder as its box, a vehicle's hull),
// never by the drawn model, so what a click selects is what the rules say is
// there however the appearance overhangs it. Proxies without a unit behind
// them (lab markers) keep their visual box.
import { vec2, vec3, type Vec3 } from "math";
import { box3 } from "math/shapes";
import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import { rayBox3Interval } from "@packages/renderer-core/src/math";
import { PROXY_ASSETS } from "./proxies";
import type { SceneInstance } from "./scene";

/** An oriented box to pick: placed at (x, y, z) turned by `yaw` about +Z,
 *  spanning `center ± half` in its own frame. */
export interface PickBox {
  x: number;
  y: number;
  z: number;
  yaw: number;
  center: readonly [number, number, number];
  half: readonly [number, number, number];
}

/** The soldier's body every squad shares, as the rules' `physics` block gives it. */
export interface SoldierBody {
  soldier_radius_m: number;
  soldier_height_m: number;
}

/** The simulation's box for a unit standing at its foot: its type's hull
 *  (`UnitCatalog.hull`), or with none a soldier's upright cylinder as its
 *  bounding box. */
export function bodyBox(
  rules: SoldierBody,
  hull: { half_extents_m: readonly number[] } | null,
): Pick<PickBox, "center" | "half"> {
  if (hull) {
    const [x, y, z] = hull.half_extents_m;
    return { center: [0, 0, z], half: [x, y, z] };
  }
  const r = rules.soldier_radius_m;
  const h = rules.soldier_height_m / 2;
  return { center: [0, 0, h], half: [r, r, h] };
}

/** A proxy's own visual box (lab markers and crates with no unit behind them). */
export function proxyPickBox(inst: SceneInstance): PickBox {
  const asset = PROXY_ASSETS[inst.kind];
  return {
    x: inst.x,
    y: inst.y,
    z: inst.z,
    yaw: inst.yaw,
    center: asset.center,
    half: asset.halfExtents,
  };
}

const _pick_origin = vec3.create();
const _pick_dir = vec3.create();
const _pick_box = box3.create();
const _pick_interval = vec2.create();
const WORLD_ORIGIN: Vec3 = vec3.create();

/** Distance along `ray` to `box`: tested in the box's own frame (the ray
 *  turned by −yaw about +Z). `Infinity` on a miss. */
export function rayBoxDistance(ray: WorldRay, box: PickBox): number {
  vec3.set(_pick_origin, ray.origin[0] - box.x, ray.origin[1] - box.y, ray.origin[2] - box.z);
  vec3.rotateZ(_pick_origin, _pick_origin, WORLD_ORIGIN, -box.yaw);
  vec3.rotateZ(_pick_dir, ray.dir, WORLD_ORIGIN, -box.yaw);
  const [cx, cy, cz] = box.center;
  const [hx, hy, hz] = box.half;
  box3.set(_pick_box, cx - hx, cy - hy, cz - hz, cx + hx, cy + hy, cz + hz);
  return rayBox3Interval(_pick_interval, _pick_origin, _pick_dir, _pick_box)
    ? _pick_interval[0]
    : Infinity;
}

/** Index of the nearest box hit by `ray`, or -1. */
export function pickBox(ray: WorldRay, boxes: readonly PickBox[]): number {
  let best = -1,
    bestT = Infinity;
  for (let i = 0; i < boxes.length; i++) {
    const t = rayBoxDistance(ray, boxes[i]);
    if (t < bestT) {
      bestT = t;
      best = i;
    }
  }
  return best;
}
