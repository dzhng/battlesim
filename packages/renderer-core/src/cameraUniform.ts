import { eyePosition, invViewProj, viewProjMatrix, type Camera3DParams } from "./camera3d";

export interface CameraSnapshot {
  /** The one projection owner. `aspect` is overridden by the live width/height
   *  so the projection follows every resize with a single owner. */
  camera3d: Camera3DParams;
  /** Viewport in device pixels. */
  width: number;
  height: number;
}

// Packed layout of the WGSL `Camera` struct (16-byte aligned). Float offsets:
//   0..15  viewProj   16..31 invViewProj   32..34 eye   35 znear
//   36 width   37 height   38..39 pad
export const CAMERA_UNIFORM_FLOATS = 40;
export const CAMERA_UNIFORM_BYTES = CAMERA_UNIFORM_FLOATS * 4;

/** camera3d params with aspect pinned to the live viewport — the single owner
 *  for both the GPU packing and CPU picking. */
export function liveCamera(camera: CameraSnapshot): Camera3DParams {
  return { ...camera.camera3d, aspect: camera.width / Math.max(1, camera.height) };
}

export function cameraUniformData(camera: CameraSnapshot): Float32Array<ArrayBuffer> {
  const data = new Float32Array(CAMERA_UNIFORM_FLOATS);
  const params = liveCamera(camera);
  data.set(viewProjMatrix(params), 0);
  data.set(invViewProj(params), 16);
  const eye = eyePosition(params);
  data.set(eye, 32);
  data[35] = params.near;
  data[36] = camera.width;
  data[37] = camera.height;
  return data;
}
