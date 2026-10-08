// The player camera's numbers, from the one fixture owner's presentation block.
import game from "@fixtures/game.json";
import generated from "@fixtures/generated-battle.json";
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
      pitch: Math.min(Math.PI / 2 - 0.08, rig.pitchAt(distance) + 0.18),
      yaw,
      ...gameCamera.lens,
    };
  },
  /** Opening battle framing: put the eye on the player's entry edge and look
   *  across the map so that own reinforcements are always in the foreground. */
  fromStart(start: readonly [number, number], size: readonly [number, number]): Camera3DParams {
    const center: [number, number] = [size[0] / 2, size[1] / 2];
    const dx = start[0] - center[0];
    const dy = start[1] - center[1];
    // Spawn reports sit on one of the four map edges. Snap the opening view
    // to that edge's axis so the map border stays parallel to the view.
    const edgeX = Math.min(start[0], size[0] - start[0]);
    const edgeY = Math.min(start[1], size[1] - start[1]);
    const edgeDirection: [number, number] =
      edgeX < edgeY ? [start[0] < center[0] ? -1 : 1, 0] : [0, start[1] < center[1] ? -1 : 1];
    const heading = Math.atan2(edgeDirection[1], edgeDirection[0]);
    const radial = Math.hypot(dx, dy);
    // A slightly higher opening elevation keeps the entry chevrons legible on
    // the ground while preserving the direct spawn-to-enemy heading.
    const pitch = Math.min(
      Math.PI / 2 - 0.08,
      rig.pitchAt(Math.max(presentation.default.distance, radial)) + 0.28,
    );
    const baseDistance = Math.max(
      presentation.default.distance,
      Math.max(1, radial) / Math.max(0.01, Math.cos(pitch)),
    );
    // Keep the player's entry edge in the opening frame so the deployment
    // marker answers "where do I enter?" before the first camera pan.
    const target: [number, number] = [center[0] + dx * 0.35, center[1] + dy * 0.35];
    const distance = baseDistance * 0.95;
    return {
      target: [target[0], target[1], 0],
      distance,
      pitch,
      // Look directly down the entry lane toward the enemy so the map edges
      // stay parallel to the battle's direction of travel.
      yaw: heading,
      ...gameCamera.lens,
    };
  },
  /** The rig for a map `size` metres across: the game's, with the wheel
   *  reaching far enough out, and tilting far enough down, to take the whole
   *  map in. */
  forMap(
    size: readonly [number, number],
    rendered?: readonly [number, number, number, number],
  ): CameraPresentation {
    const span = rendered
      ? Math.max(rendered[2] - rendered[0], rendered[3] - rendered[1])
      : Math.max(...size);
    const far = Math.max(config.zoom_max, span * generated.camera.overview_span);
    if (far === config.zoom_max) return config;
    return {
      ...config,
      zoom_max: far,
      pitch_curve: [...config.pitch_curve, [far, generated.camera.overview_pitch]],
    };
  },
};
