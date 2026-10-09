// Adapted from ~/dev/game renderer-core/src/cameraUniform.ts:
// the camera uniform every world, shadow and overlay shader binds at group 0.
// Local changes: `liveCamera` is exported (the source's private `realParams`)
// for CPU picking, the unused screen-space helpers are dropped, and the packer
// writes into a caller-owned buffer through `math` scratch.
import { vec3 } from "math";
import {
  createGpuMat4,
  eyePosition,
  invViewProj,
  viewProjMatrix,
  type Camera3DParams,
} from "./camera3d";

/** The projection camera and the viewport it is drawn into. */
export interface ViewportCamera {
  /** The real 3D perspective camera (camera3d) — the ONE projection owner.
   *  `aspect` is overridden by the live width/height so the projection follows
   *  every resize with a single owner. */
  camera3d: Camera3DParams;
  /** Viewport in device pixels. */
  width: number;
  height: number;
}

export interface CameraSnapshot extends ViewportCamera {
  /** Animation clock (seconds) packed into `cam.time`. Defaults to 0 — unset
   *  means a frozen frame. */
  time?: number;
}

// Packed layout of `struct Camera` (battle-renderer world/camera.ts),
// WGSL 16-byte alignment. Float offsets:
//   0..15  viewProj (mat4x4, 64B)
//  16..31  invViewProj (mat4x4, 64B)
//  32..34  eye (vec3, padded to 16B by znear)
//  35      znear
//  36      width
//  37      height
//  38      time
//  39      pad (struct rounds up to 40 floats / 160 bytes)
export const CAMERA_UNIFORM_FLOATS = 40;
export const CAMERA_UNIFORM_BYTES = CAMERA_UNIFORM_FLOATS * 4;
const VIEW_PROJ_OFFSET = 0;
const INV_VIEW_PROJ_OFFSET = 16;
const EYE_OFFSET = 32;
const ZNEAR_OFFSET = 35;
const WIDTH_OFFSET = 36;
const HEIGHT_OFFSET = 37;
const TIME_OFFSET = 38;

/** camera3d params with aspect pinned to the live viewport — the single owner
 *  for both the GPU packing and CPU picking. */
export function liveCamera(camera: ViewportCamera): Camera3DParams {
  return { ...camera.camera3d, aspect: camera.width / Math.max(1, camera.height) };
}

const _uniform_matrix = createGpuMat4();
const _uniform_eye = vec3.create();

/** Packs `camera` into `data` (`CAMERA_UNIFORM_FLOATS` long) and returns it. */
export function cameraUniformData<T extends Float32Array>(data: T, camera: CameraSnapshot): T {
  const params = liveCamera(camera);
  data.set(viewProjMatrix(_uniform_matrix, params), VIEW_PROJ_OFFSET);
  data.set(invViewProj(_uniform_matrix, params), INV_VIEW_PROJ_OFFSET);
  vec3.toBuffer(data, eyePosition(_uniform_eye, params), EYE_OFFSET);
  data[ZNEAR_OFFSET] = params.near;
  data[WIDTH_OFFSET] = camera.width;
  data[HEIGHT_OFFSET] = camera.height;
  data[TIME_OFFSET] = camera.time ?? 0;
  return data;
}
