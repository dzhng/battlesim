// Where shadow receivers are, in view depth: the part of the map box the
// camera sees. Cascades split over this range rather than from the camera's
// near plane, so a camera a kilometre up does not spend them on air
// (spike 01, landmine 1).
import { vec2, vec3, type Mat4, type Vec2, type Vec3 } from "math";
import { box3, type Box3 } from "math/shapes";
import {
  createGpuMat4,
  createWorldRay,
  eyePosition,
  invViewProj,
  screenRayFrom,
  viewMatrix,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d";
import { rayBox3Interval } from "@packages/renderer-core/src/math";
import { VERTEX_FLOATS, type Mesh } from "../mesh";

/** Headroom above the highest ground for what stands on it (buildings, trees). */
const STANDING_M = 25;
/** Screen rays per side of the sampling grid. */
const GRID = 12;

const _mapBox_vertex = vec3.create();
const _range_view = createGpuMat4();
const _range_inverse = createGpuMat4();
const _range_eye = vec3.create();
const _range_ray = createWorldRay();
const _range_interval = vec2.create();
const _range_point = vec3.create();

/** The box around a world mesh, a metre below its lowest vertex and raised to
 *  hold what stands on the ground. */
export function mapBox(mesh: Mesh): Box3 | null {
  if (mesh.length === 0) return null;
  const box = box3.create();
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    box3.expandByPoint(box, box, vec3.fromBuffer(_mapBox_vertex, mesh, i));
  box[2] -= 1;
  box[5] += STANDING_M;
  return box;
}

/** View depth of a world point: the negated view-space z. */
function viewDepth(view: Mat4, p: Vec3): number {
  return -vec3.transformMat4(_range_point, p, view)[2];
}

/** The view-depth range [near, far] of the box where screen rays cross it,
 *  sampled on a grid of rays, written into `out`. Without a box, or when no
 *  ray meets it, the range collapses to just past the near plane. */
export function receiverRange(out: Vec2, camera: Camera3DParams, box: Box3 | null): Vec2 {
  if (!box) return vec2.set(out, camera.near, camera.near * 2);
  viewMatrix(_range_view, camera);
  invViewProj(_range_inverse, camera);
  eyePosition(_range_eye, camera);
  let near = Infinity;
  let far = 0;
  for (let iy = 0; iy <= GRID; iy++) {
    for (let ix = 0; ix <= GRID; ix++) {
      const ray = screenRayFrom(
        _range_ray,
        _range_inverse,
        _range_eye,
        (ix / GRID) * 2 - 1,
        (iy / GRID) * 2 - 1,
      );
      if (!rayBox3Interval(_range_interval, ray.origin, ray.dir, box)) continue;
      const [t0, t1] = _range_interval;
      near = Math.min(
        near,
        viewDepth(_range_view, vec3.scaleAndAdd(_range_point, ray.origin, ray.dir, t0)),
      );
      far = Math.max(
        far,
        viewDepth(_range_view, vec3.scaleAndAdd(_range_point, ray.origin, ray.dir, t1)),
      );
    }
  }
  if (!Number.isFinite(near)) return vec2.set(out, camera.near, camera.near * 2);
  return vec2.set(out, Math.max(camera.near, near), far);
}
