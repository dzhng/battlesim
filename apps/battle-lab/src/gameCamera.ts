// The player camera's numbers, from the one fixture owner's presentation block.
import game from "@fixtures/game.json";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import {
  CameraController,
  type CameraPresentation,
} from "@packages/renderer-core/src/cameraController";

const presentation = game.presentation.camera;
const config: CameraPresentation = {
  ...presentation,
  pitch_curve: presentation.pitch_curve as [number, number][],
};
const rig = new CameraController(config);

export const gameCamera = {
  config,
  /** The lens every lab camera looks through: the fixture's field of view
   *  and near plane (aspect is the viewport's, set as it draws). */
  lens: { fovY: config.fov_y, aspect: 1, near: config.near_m },
  /** The battle's opening framing (Defilade's): the fixture's target, distance
   *  and yaw, pitched by the curve. */
  opening(): Camera3DParams {
    const { target, distance, yaw } = presentation.default;
    return {
      target: [target[0], target[1], 0],
      distance,
      pitch: rig.pitchAt(distance),
      yaw,
      ...gameCamera.lens,
    };
  },
};
