// The player camera's numbers, from the one fixture owner's presentation block.
import village from "@fixtures/village.json";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import {
  CameraController,
  type CameraPresentation,
} from "@packages/renderer-core/src/cameraController";

const presentation = village.presentation.camera;
const config: CameraPresentation = {
  ...presentation,
  pitch_curve: presentation.pitch_curve as [number, number][],
};
const rig = new CameraController(config);

export const villageCamera = {
  config,
  /** The battle's opening framing (Defilade's): the fixture's target, distance
   *  and yaw, pitched by the curve. */
  opening(): Camera3DParams {
    const { target, distance, yaw } = presentation.default;
    return {
      target: [target[0], target[1], 0],
      distance,
      pitch: rig.pitchAt(distance),
      yaw,
      fovY: 0.8,
      aspect: 1,
      near: 1,
    };
  },
};
