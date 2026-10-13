// The live camera of a viewport: the orbit camera and the pixels it is drawn
// into. The presentation frames it (`presentedCamera`) and CPU picking reads
// it, both through `liveCamera`, so a resize moves the projection in one place.
// `liveCamera` is adapted from ~/dev/game renderer-core/src/cameraUniform.ts
// (its private `realParams`).
import type { Camera3DParams } from "./camera3d";

/** The projection camera and the viewport it is drawn into. */
export interface ViewportCamera {
  /** The real 3D perspective camera (camera3d). `aspect` is overridden by the
   *  live width/height so the projection follows every resize. */
  camera3d: Camera3DParams;
  /** Viewport in device pixels. */
  width: number;
  height: number;
}

/** camera3d params with aspect pinned to the live viewport. */
export function liveCamera(camera: ViewportCamera): Camera3DParams {
  return { ...camera.camera3d, aspect: camera.width / Math.max(1, camera.height) };
}
