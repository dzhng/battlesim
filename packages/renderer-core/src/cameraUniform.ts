// Adapted from ~/dev/game renderer-core/src/cameraUniform.ts (reuse manifest):
// the 48-float camera every world, shadow and overlay shader binds at group 0.
// Local changes: `liveCamera` is exported (the source's private `realParams`)
// for CPU picking, and the unused screen-space helpers are dropped.
import { eyePosition, invViewProj, viewProjMatrix, type Camera3DParams } from "./camera3d";

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
  /** The camera's ground view centre — NOT a projection input. Feeds the
   *  `cam.focus` uniform for distance-keyed surface effects. */
  x: number;
  y: number;
  /** Device pixels per world unit at the look target — NOT a projection
   *  input. Feeds the `cam.zoom` detail-gate scalar. */
  zoom: number;
  /** Animation clock (seconds) packed into `cam.time`. Defaults to 0 — unset
   *  means a frozen frame. */
  time?: number;
  /** Sun azimuth/elevation (radians) — the frame's environment-owned light
   *  direction for shared effects. */
  sunAzimuth?: number;
  sunElevation?: number;
}

// Packed layout of `struct Camera` (battle-renderer world/camera.ts),
// WGSL 16-byte alignment. Float offsets:
//   0..15  viewProj (mat4x4, 64B)
//  16..31  invViewProj (mat4x4, 64B)
//  32..34  eye (vec3, padded to 16B by znear)
//  35      znear
//  36..37  focus (vec2)
//  38      width
//  39      height
//  40      zoom  (detail gate — px per world unit at the target)
//  41      tilt  (sin(camera3d pitch): 1 top-down → 0 horizon)
//  42      time
//  43      zfar  (0 = infinite-far sentinel)
//  44      sunAz
//  45      sunEl
//  46..47  pad (struct rounds up to 48 floats / 192 bytes)
export const CAMERA_UNIFORM_FLOATS = 48;
export const CAMERA_UNIFORM_BYTES = CAMERA_UNIFORM_FLOATS * 4;
const VIEW_PROJ_OFFSET = 0;
const INV_VIEW_PROJ_OFFSET = 16;
const EYE_OFFSET = 32;
const ZNEAR_OFFSET = 35;
const FOCUS_OFFSET = 36;
const WIDTH_OFFSET = 38;
const HEIGHT_OFFSET = 39;
const ZOOM_OFFSET = 40;
const TILT_OFFSET = 41;
const TIME_OFFSET = 42;
const ZFAR_OFFSET = 43;
const SUN_AZ_OFFSET = 44;
const SUN_EL_OFFSET = 45;

/** camera3d params with aspect pinned to the live viewport — the single owner
 *  for both the GPU packing and CPU picking. */
export function liveCamera(camera: ViewportCamera): Camera3DParams {
  return { ...camera.camera3d, aspect: camera.width / Math.max(1, camera.height) };
}

export function cameraUniformData(
  camera: CameraSnapshot & Required<Pick<CameraSnapshot, "sunAzimuth" | "sunElevation">>,
): Float32Array<ArrayBuffer> {
  const data = new Float32Array(CAMERA_UNIFORM_FLOATS);
  const params = liveCamera(camera);
  data.set(viewProjMatrix(params), VIEW_PROJ_OFFSET);
  data.set(invViewProj(params), INV_VIEW_PROJ_OFFSET);
  const eye = eyePosition(params);
  data[EYE_OFFSET] = eye[0];
  data[EYE_OFFSET + 1] = eye[1];
  data[EYE_OFFSET + 2] = eye[2];
  data[ZNEAR_OFFSET] = params.near;
  data[FOCUS_OFFSET] = camera.x;
  data[FOCUS_OFFSET + 1] = camera.y;
  data[WIDTH_OFFSET] = camera.width;
  data[HEIGHT_OFFSET] = camera.height;
  data[ZOOM_OFFSET] = camera.zoom;
  data[TILT_OFFSET] = Math.sin(Math.min(Math.PI / 2, Math.max(0, params.pitch)));
  data[TIME_OFFSET] = camera.time ?? 0;
  data[ZFAR_OFFSET] = params.far ?? 0; // 0 = infinite far sentinel
  data[SUN_AZ_OFFSET] = camera.sunAzimuth;
  data[SUN_EL_OFFSET] = camera.sunElevation;
  return data;
}
