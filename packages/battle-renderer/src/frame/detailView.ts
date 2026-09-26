// What a frame's camera means for level of detail: where the eye is, how many
// pixels a metre covers at a metre's distance, and the view's side planes.
// The scenery layer and the models layer choose their tiers from it.
import { vec3, type Vec3 } from "math";
import { frustum, type Frustum } from "math/shapes";
import {
  createGpuMat4,
  eyePosition,
  projMatrix,
  viewMatrix,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d";

export interface DetailView {
  eye: Vec3;
  /** Device pixels per metre at one metre's distance: `height / (2·tan(fovY/2))`. */
  pixelsPerMetre: number;
  /** The camera's four side planes (near and far are not tested). */
  sides: Frustum;
}

export function createDetailView(): DetailView {
  return { eye: vec3.create(), pixelsPerMetre: 1, sides: frustum.create() };
}

const _detail_view = createGpuMat4();
const _detail_proj = createGpuMat4();

/** Pose `out` for `camera` drawing into a viewport `height` pixels tall. */
export function setDetailView(out: DetailView, camera: Camera3DParams, height: number): DetailView {
  eyePosition(out.eye, camera);
  out.pixelsPerMetre = height / (2 * Math.tan(camera.fovY / 2));
  frustum.setFromViewProjectionMatrixSides(
    out.sides,
    projMatrix(_detail_proj, camera),
    viewMatrix(_detail_view, camera),
  );
  return out;
}

/** A camera's identity for "has the view changed": every field that moves a pixel. */
export function detailKey(camera: Camera3DParams, height: number): string {
  return `${camera.target}|${camera.distance}|${camera.yaw}|${camera.pitch}|${camera.fovY}|${camera.aspect}|${height}`;
}
