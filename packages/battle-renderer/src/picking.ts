import { vec2, vec3 } from "math";
import { box3 } from "math/shapes";
import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import { rayBox3Interval } from "@packages/renderer-core/src/math";
import { PROXY_ASSETS } from "./proxies";
import type { SceneInstance } from "./scene";

const _pick_origin = vec3.create();
const _pick_dir = vec3.create();
const _pick_box = box3.create();
const _pick_interval = vec2.create();
const WORLD_ORIGIN = vec3.create();

/** Distance along `ray` to the instance's oriented visual box: the box is
 *  tested in the instance's own frame (the ray turned by −yaw about +Z).
 *  `Infinity` on a miss. */
export function rayInstanceDistance(ray: WorldRay, inst: SceneInstance): number {
  const asset = PROXY_ASSETS[inst.kind];
  vec3.set(_pick_origin, ray.origin[0] - inst.x, ray.origin[1] - inst.y, ray.origin[2] - inst.z);
  vec3.rotateZ(_pick_origin, _pick_origin, WORLD_ORIGIN, -inst.yaw);
  vec3.rotateZ(_pick_dir, ray.dir, WORLD_ORIGIN, -inst.yaw);
  const [cx, cy, cz] = asset.center;
  const [hx, hy, hz] = asset.halfExtents;
  box3.set(_pick_box, cx - hx, cy - hy, cz - hz, cx + hx, cy + hy, cz + hz);
  return rayBox3Interval(_pick_interval, _pick_origin, _pick_dir, _pick_box)
    ? _pick_interval[0]
    : Infinity;
}

/** Index of the nearest instance hit by `ray`, or -1. */
export function pickInstance(ray: WorldRay, instances: readonly SceneInstance[]): number {
  let best = -1,
    bestT = Infinity;
  for (let i = 0; i < instances.length; i++) {
    const t = rayInstanceDistance(ray, instances[i]);
    if (t < bestT) {
      bestT = t;
      best = i;
    }
  }
  return best;
}
