import { vec3, type Mat4, type Vec3 } from "math";
import {
  createGpuMat4,
  createWorldRay,
  screenRay,
  viewMatrix,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d";
import {
  cameraUniformData,
  CAMERA_UNIFORM_FLOATS,
  type CameraSnapshot,
} from "@packages/renderer-core/src/cameraUniform";
export type FrameCameraSnapshot = CameraSnapshot &
  Required<Pick<CameraSnapshot, "sunAzimuth" | "sunElevation">>;

/** One frame's camera publication. The arrays are module scratch, rewritten by
 *  the next `frameCamera` call: consume them before drawing the next frame. */
export interface FrameCameraState {
  bytes: Float32Array<ArrayBuffer>;
  view: Mat4;
  rays: { origin: Vec3; dx: Vec3; dy: Vec3 };
}

const _frameCamera_state: FrameCameraState = {
  bytes: new Float32Array(CAMERA_UNIFORM_FLOATS),
  view: createGpuMat4(),
  rays: { origin: vec3.create(), dx: vec3.create(), dy: vec3.create() },
};
const _frameCamera_params: Camera3DParams = {
  target: vec3.create(),
  distance: 0,
  pitch: 0,
  yaw: 0,
  fovY: 0,
  aspect: 1,
  near: 1,
  far: undefined,
};
const _frameCamera_ray = createWorldRay();

/** Shared camera publication and perspective sky rays; dimensions come from the physical target. */
export function frameCamera(
  snapshot: FrameCameraSnapshot,
  width: number,
  height: number,
): FrameCameraState {
  const camera = snapshot.camera3d;
  const params = _frameCamera_params;
  params.target = camera.target;
  params.distance = camera.distance;
  params.pitch = camera.pitch;
  params.yaw = camera.yaw;
  params.fovY = camera.fovY;
  params.aspect = width / height;
  params.near = camera.near;
  params.far = camera.far;
  const state = _frameCamera_state;
  cameraUniformData(state.bytes, { ...snapshot, camera3d: params, width, height });
  viewMatrix(state.view, params);
  // Symmetric corners share a normalization factor; interpolate these rays before final normalization.
  const { origin, dx, dy } = state.rays;
  vec3.copy(origin, screenRay(_frameCamera_ray, params, -1, 1).dir);
  vec3.subtract(dx, screenRay(_frameCamera_ray, params, 1, 1).dir, origin);
  vec3.subtract(dy, screenRay(_frameCamera_ray, params, -1, -1).dir, origin);
  return state;
}
