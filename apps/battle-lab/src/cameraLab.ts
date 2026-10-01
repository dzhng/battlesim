// The camera-clearance lab's fixture (`fixtures/camera-lab.json`): a small
// flat map of prototype buildings (a wall, a tower, a corner, a courtyard, a
// concave compound) and scripted camera trajectories round them. A flight is
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
import type { KnownProp, MapProp } from "@packages/battle-renderer/src/models/propAppearance";
import type { PublicBuildings } from "@packages/battle-renderer/src/worldMesh";
import { sampleTour, type BenchmarkTour, type TourKeyframe } from "@web/battle/benchmark/camera";
import type { Wasm } from "@web/battle/sim/module";

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

/** The lab's map, compiled from its plan and the prototype template
 *  catalogue (`templates`, the catalogue's JSON) by the map compiler. */
export function compileCameraLabMap(
  wasm: Pick<Wasm, "compile_map" | "template_catalogue_json">,
  templates: string,
): string {
  const { hash } = JSON.parse(wasm.template_catalogue_json(templates)) as { hash: string };
  const outcome = JSON.parse(
    wasm.compile_map(
      JSON.stringify({
        generator_version: "camera-lab",
        preset_revision: "camera-lab",
        seed: "0",
        template_catalog_hash: hash,
        limits: lab.limits,
        plan: lab.plan,
      }),
      templates,
    ),
  ) as
    | { status: "ok"; result: { map: unknown } }
    | { status: "error"; diagnostics: { message: string }[] };
  if (outcome.status !== "ok")
    throw new Error(outcome.diagnostics.map((d) => d.message).join("; "));
  return JSON.stringify(outcome.result.map);
}

/** What blue knows once it has seen the lab's falling building come down:
 *  each of its parts replaced by low remains on the same plan. */
export function seenFallen(buildings: PublicBuildings, props: readonly MapProp[]): KnownProp[] {
  const fallen = lab.plan.buildings.find((b) => b.id === lab.fallen.building)!;
  const parts = buildings.buildings.find((b) => b.owner === fallen.owner)!.parts;
  return parts.map(({ prop }) => {
    const standing = props.find((p) => p.id === prop)!;
    return {
      ...standing,
      half: [standing.half[0], standing.half[1], lab.fallen.remains_height_m / 2],
      authoredProp: prop,
      replaces: prop,
    };
  });
}

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
