import { screenRay, viewMatrix } from "../../renderer-core/src/camera3d";
import {
  cameraUniformData,
  type CameraSnapshot,
} from "../../renderer-core/src/cameraUniform";
export type FrameCameraSnapshot = CameraSnapshot &
  Required<Pick<CameraSnapshot, "sunAzimuth" | "sunElevation">>;
/** Shared camera publication and perspective sky rays; dimensions come from the physical target. */
export function frameCamera(snapshot: FrameCameraSnapshot, width: number, height: number) {
  const params = { ...snapshot.camera3d, aspect: width / height };
  const bytes = cameraUniformData({ ...snapshot, camera3d: params, width, height });
  // Symmetric corners share a normalization factor; interpolate these rays before final normalization.
  const a = screenRay(params, -1, 1).dir,
    b = screenRay(params, 1, 1).dir,
    c = screenRay(params, -1, -1).dir;
  return {
    bytes,
    view: viewMatrix(params),
    rays: {
      origin: a,
      dx: [b[0] - a[0], b[1] - a[1], b[2] - a[2]] as const,
      dy: [c[0] - a[0], c[1] - a[1], c[2] - a[2]] as const,
    },
  };
}
