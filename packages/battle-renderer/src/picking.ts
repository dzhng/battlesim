import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import { PROXY_ASSETS } from "./proxies";
import type { SceneInstance } from "./scene";

/** Distance along `ray` to the instance's oriented visual box, or null on miss. */
export function rayInstanceDistance(ray: WorldRay, inst: SceneInstance): number | null {
  const asset = PROXY_ASSETS[inst.kind];
  const c = Math.cos(inst.yaw),
    s = Math.sin(inst.yaw);
  // Ray into the instance's local frame (inverse yaw about +Z).
  const ox = ray.origin[0] - inst.x,
    oy = ray.origin[1] - inst.y,
    oz = ray.origin[2] - inst.z;
  const o = [
    ox * c + oy * s - asset.center[0],
    -ox * s + oy * c - asset.center[1],
    oz - asset.center[2],
  ];
  const dir = [ray.dir[0] * c + ray.dir[1] * s, -ray.dir[0] * s + ray.dir[1] * c, ray.dir[2]];
  let t0 = 0,
    t1 = Infinity;
  for (let a = 0; a < 3; a++) {
    const h = asset.halfExtents[a];
    if (Math.abs(dir[a]) < 1e-12) {
      if (Math.abs(o[a]) > h) return null;
      continue;
    }
    let near = (-h - o[a]) / dir[a],
      far = (h - o[a]) / dir[a];
    if (near > far) [near, far] = [far, near];
    t0 = Math.max(t0, near);
    t1 = Math.min(t1, far);
    if (t0 > t1) return null;
  }
  return t0;
}

/** Index of the nearest instance hit by `ray`, or -1. */
export function pickInstance(ray: WorldRay, instances: readonly SceneInstance[]): number {
  let best = -1,
    bestT = Infinity;
  instances.forEach((inst, i) => {
    const t = rayInstanceDistance(ray, inst);
    if (t !== null && t < bestT) {
      bestT = t;
      best = i;
    }
  });
  return best;
}
