// The Armory's camera. The stage draws the whole screen behind the glass
// panels, the way the yard fills the mock, and the unit stands in the centre
// cell between the list and the sheet: the camera's target slides sideways so
// the orbit's centre falls in the middle of that cell.
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { gameCamera } from "../gameCamera";
import type { CameraPose } from "./yard";

interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** `pose` through the game's lens, drawn into `view` (the whole canvas),
 *  with its target where the middle of `cell` falls: both boxes in CSS
 *  pixels. */
export function framedOrbit(pose: CameraPose, view: Box, cell: Box): Camera3DParams {
  const orbit = { ...gameCamera.lens, ...pose };
  if (!(view.height > 0)) return orbit;
  // Metres per pixel at the target's distance.
  const perPx = (2 * pose.distance * Math.tan(orbit.fovY / 2)) / view.height;
  const right = (cell.left + cell.width / 2 - (view.left + view.width / 2)) * perPx;
  const down = (cell.top + cell.height / 2 - (view.top + view.height / 2)) * perPx;
  // The view's right on the ground (+Y at yaw 0: the eye looks down -X) and
  // its up, tilted back by the pitch.
  const [rx, ry] = [-Math.sin(pose.yaw), Math.cos(pose.yaw)];
  const s = Math.sin(pose.pitch);
  const up = [-s * Math.cos(pose.yaw), -s * Math.sin(pose.yaw), Math.cos(pose.pitch)];
  const [x, y, z] = pose.target;
  // The target moves against the cell's offset, so the unit lands in it.
  return {
    ...orbit,
    target: [x - rx * right + up[0] * down, y - ry * right + up[1] * down, z + up[2] * down],
  };
}
