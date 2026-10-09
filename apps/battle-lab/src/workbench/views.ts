// The workbench's named views: five studio views framed on the model's
// bounds, and the battle camera at three zooms — the game rig's own pitch
// curve and opening yaw, so a model is judged at the size and angle a player
// sees it. Each view is a camera the shared rig could hold, never a private
// projection.

import { vec3 } from "math";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { CameraPresentation } from "@packages/renderer-core/src/cameraController";
import { CameraController } from "@packages/renderer-core/src/cameraController";
import type { ReferenceView } from "@packages/scene-assets/src/references";
import type { Bounds } from "@packages/scene-assets/src/schema";
import { gameCamera } from "../gameCamera";

export const WORKBENCH_VIEWS = [
  "q-front",
  "front",
  "left",
  "rear",
  "top",
  "battle-near",
  "battle-mid",
  "battle-far",
] as const;
export type WorkbenchView = (typeof WORKBENCH_VIEWS)[number];

/** The surface sheet's close views: front and rear three-quarters, nearer
 *  than the studio views so a texture's grain reads. */
export const SURFACE_VIEWS = ["surface-front", "surface-rear"] as const;
/** A rear three-quarter studio view, which only the reference sheet needs. */
export type SheetView = WorkbenchView | (typeof SURFACE_VIEWS)[number] | "q-rear";

/** The studio view each reference view (`references.ts`) is set beside on the
 *  reference sheet; a close-up has none. `left` is the vehicle's left side. */
export const REFERENCE_CAMERAS: Record<ReferenceView, SheetView | null> = {
  three_quarter_front: "q-front",
  side: "left",
  three_quarter_rear: "q-rear",
  front: "front",
  rear: "rear",
  top: "top",
  detail: null,
};

/** Studio views: eye azimuth (the model faces +X), elevation (radians), and
 *  how near, as a share of the distance that frames the whole model. */
const STUDIO: Record<string, { yaw: number; pitch: number; near?: number }> = {
  "q-front": { yaw: Math.PI / 4, pitch: 0.38 },
  front: { yaw: 0, pitch: 0.12 },
  left: { yaw: Math.PI / 2, pitch: 0.12 },
  rear: { yaw: Math.PI, pitch: 0.12 },
  "q-rear": { yaw: (Math.PI * 3) / 4, pitch: 0.38 },
  top: { yaw: -Math.PI / 2, pitch: Math.PI / 2 - 0.02 },
  "surface-front": { yaw: Math.PI / 5, pitch: 0.3, near: 0.62 },
  "surface-rear": { yaw: -Math.PI * 0.7, pitch: 0.3, near: 0.62 },
};

/** Battle zooms: the rig's closest distance, its opening distance, and a
 *  company-scale view. */
const BATTLE_DISTANCES: Record<string, number> = {
  "battle-near": gameCamera.config.zoom_min,
  "battle-mid": gameCamera.opening().distance,
  "battle-far": 150,
};

const STUDIO_FOV = 0.6;
const rig = new CameraController(gameCamera.config);

/** The rig the workbench steers with: the game's feel, reaching in to a
 *  metre and holding whatever tilt the user drags in. */
export const WORKBENCH_CAMERA: CameraPresentation = {
  ...gameCamera.config,
  zoom_min: 0.8,
  zoom_max: 600,
  pitch_curve: [
    [0.8, 0.38],
    [600, 0.38],
  ],
};

/** The camera for `view` of a model whose bounds are `bounds`, standing at `at`. */
export function viewCamera(
  view: SheetView,
  bounds: Bounds,
  at: [number, number, number] = [0, 0, 0],
): Camera3DParams {
  const center = vec3.lerp(vec3.create(), bounds.min, bounds.max, 0.5);
  const radius = Math.max(0.3, vec3.distance(bounds.min, bounds.max) / 2);
  const studio = STUDIO[view];
  if (studio) {
    const whole = (radius / Math.sin(STUDIO_FOV / 2)) * 1.08;
    // A near view still keeps the model's full height in frame.
    const height = ((bounds.max[2] - bounds.min[2]) / 2 / Math.tan(STUDIO_FOV / 2)) * 1.15;
    return {
      target: [at[0] + center[0], at[1] + center[1], at[2] + center[2]],
      distance: studio.near ? Math.max(whole * studio.near, height) : whole,
      pitch: studio.pitch,
      yaw: studio.yaw,
      fovY: STUDIO_FOV,
      aspect: 1,
      near: 0.05,
    };
  }
  const opening = gameCamera.opening();
  const distance = BATTLE_DISTANCES[view];
  return {
    ...opening,
    target: [at[0] + center[0], at[1] + center[1], at[2]],
    distance,
    pitch: rig.pitchAt(distance),
    near: 0.2,
  };
}
