// Where shadow receivers are, in view depth: the part of the map box the
// camera sees. Cascades split over this range rather than from the camera's
// near plane, so a camera a kilometre up does not spend them on air
// (spike 01, landmine 1).
import { screenRay, viewMatrix, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { VERTEX_FLOATS, type Mesh } from "../mesh";

/** An axis-aligned world box: [min x, min y, min z], [max x, max y, max z]. */
export type MapBox = readonly [
  readonly [number, number, number],
  readonly [number, number, number],
];

/** Headroom above the highest ground for what stands on it (buildings, trees). */
const STANDING_M = 25;
/** Screen rays per side of the sampling grid. */
const GRID = 12;

/** The box around a world mesh, raised to hold what stands on the ground. */
export function mapBox(mesh: Mesh): MapBox | null {
  if (mesh.length === 0) return null;
  const lo = [Infinity, Infinity, Infinity];
  const hi = [-Infinity, -Infinity, -Infinity];
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
    for (let a = 0; a < 3; a++) {
      lo[a] = Math.min(lo[a], mesh[i + a]);
      hi[a] = Math.max(hi[a], mesh[i + a]);
    }
  }
  return [
    [lo[0], lo[1], lo[2] - 1],
    [hi[0], hi[1], hi[2] + STANDING_M],
  ];
}

/** The view-depth range [near, far] of the box where screen rays cross it,
 *  sampled on a grid of rays. Without a box, or when no ray meets it, the
 *  range collapses to just past the near plane. */
export function receiverRange(camera: Camera3DParams, box: MapBox | null): [number, number] {
  const fallback: [number, number] = [camera.near, camera.near * 2];
  if (!box) return fallback;
  const [lo, hi] = box;
  const view = viewMatrix(camera);
  const depthOf = (p: readonly number[]) =>
    -(view[2] * p[0] + view[6] * p[1] + view[10] * p[2] + view[14]);
  let near = Infinity;
  let far = 0;
  for (let iy = 0; iy <= GRID; iy++) {
    for (let ix = 0; ix <= GRID; ix++) {
      const { origin, dir } = screenRay(camera, (ix / GRID) * 2 - 1, (iy / GRID) * 2 - 1);
      // Slab test against the box.
      let t0 = 0;
      let t1 = Infinity;
      for (let a = 0; a < 3; a++) {
        if (Math.abs(dir[a]) < 1e-9) {
          if (origin[a] < lo[a] || origin[a] > hi[a]) t1 = -1;
          continue;
        }
        let ta = (lo[a] - origin[a]) / dir[a];
        let tb = (hi[a] - origin[a]) / dir[a];
        if (ta > tb) [ta, tb] = [tb, ta];
        t0 = Math.max(t0, ta);
        t1 = Math.min(t1, tb);
      }
      if (t1 < t0) continue;
      const at = (t: number) => [0, 1, 2].map((a) => origin[a] + dir[a] * t);
      near = Math.min(near, depthOf(at(t0)));
      far = Math.max(far, depthOf(at(t1)));
    }
  }
  if (!Number.isFinite(near)) return fallback;
  return [Math.max(camera.near, near), far];
}
