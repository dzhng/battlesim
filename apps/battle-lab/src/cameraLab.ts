// The camera-clearance lab's trajectories (`fixtures/camera-lab.json`) over
// its catalogue map (`camera-lab`): a wall, tower, corner, courtyard and
// concave compound. A flight is
// a trajectory flown through the rig and its clearance, frame by frame: the
// eye asked for and the eye drawn.
import { vec3, type Vec3 } from "math";
import lab from "@fixtures/camera-lab.json";
import { eyePosition, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import {
  createClearanceState,
  type ClearanceHold,
} from "@packages/renderer-core/src/cameraClearance";
import type { CameraController, CameraPose } from "@packages/renderer-core/src/cameraController";
import type { CameraObstacles } from "@packages/renderer-core/src/cameraObstacles";
import { sampleTour, type BenchmarkTour, type TourKeyframe } from "@web/battle/benchmark/camera";

export interface Trajectory {
  id: string;
  label: string;
  seconds: number;
  tour: BenchmarkTour;
}

/** The lab's trajectories, each a keyframed tour of one phase. */
export const TRAJECTORIES: readonly Trajectory[] = lab.trajectories.map((t) => ({
  id: t.id,
  label: t.label,
  seconds: t.seconds,
  tour: {
    version: t.id,
    phases: [{ name: t.id, from: 0, to: 1 }],
    keyframes: t.keyframes as unknown as TourKeyframe[],
  },
}));

/** Where the lab's free camera opens: the whole arena from outside. */
export const OBSERVER = lab.observer as unknown as CameraPose;
/** The eye paths' width as a share of the distance they are watched from. */
export const PATH_WIDTH = lab.watching.path_width;

/** Where the free camera stands to watch `trajectory`: over the middle of
 *  the ground it covers, far enough out to take all of it in. */
export function watchingPose(trajectory: Trajectory): CameraPose {
  const keys = trajectory.tour.keyframes;
  const [xs, ys] = [keys.map((k) => k[1]), keys.map((k) => k[2])];
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const reach = Math.hypot(x1 - x0, y1 - y0) + 2 * Math.max(...keys.map((k) => k[3]));
  return {
    ...OBSERVER,
    target: [(x0 + x1) / 2, (y0 + y1) / 2],
    distance: Math.min(OBSERVER.distance, lab.watching.reach_scale * reach),
  };
}

/** The framing `trajectory` asks for `seconds` in (held at its last past the end). */
export function trajectoryPose(trajectory: Trajectory, seconds: number): CameraPose {
  return sampleTour(trajectory.tour, seconds * 1000, trajectory.seconds * 1000).pose;
}

/** The owner of the building blue can be made to see collapse: one low
 *  enough that the simulation brings it down to remains. */
export const COLLAPSING_OWNER = lab.collapses.owner;

/** One frame of a flight. */
export interface FlightFrame {
  seconds: number;
  asked: Camera3DParams;
  drawn: Camera3DParams;
  askedEye: Vec3;
  eye: Vec3;
  hold: ClearanceHold;
  /** The eye is against a wall or a roof. */
  pressed: boolean;
  cut: boolean;
  blocked: boolean;
}

/** `trajectory` flown at `hz` frames a second by `rig` over `obstacles`,
 *  through the lens of `lens` (its field of view, near plane and aspect). */
export function flyTrajectory(
  trajectory: Trajectory,
  rig: CameraController,
  obstacles: CameraObstacles,
  lens: Camera3DParams,
  hz = 60,
): FlightFrame[] {
  const state = createClearanceState();
  const frames: FlightFrame[] = [];
  let asked = lens;
  for (let k = 0; k <= trajectory.seconds * hz; k++) {
    asked = rig.place(asked, trajectoryPose(trajectory, k / hz));
    const drawn = rig.resolve(state, asked, 1 / hz, obstacles);
    frames.push({
      seconds: k / hz,
      asked,
      drawn,
      askedEye: eyePosition(vec3.create(), asked),
      eye: eyePosition(vec3.create(), drawn),
      hold: state.hold,
      pressed: state.pressed,
      cut: state.cut,
      blocked: state.blocked,
    });
  }
  return frames;
}
