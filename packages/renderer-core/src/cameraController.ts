// The player camera: a Total War style rig (battle-look I5, I6). Held keys,
// the wheel, a drag and the screen edge become one CameraIntent; one
// step turns it, with elapsed time and the fixture's `presentation.camera`,
// into the next camera3d parameter set. Pure and GPU-free.
//
// `step` and `place` give the pose asked for; `resolve` gives the pose to
// draw for it, clear of buildings and the ground (cameraClearance.ts). Every
// playable and scripted camera path draws the resolved pose.
//
// Scene-authored framings go through the viewport's raw `setCamera`, which
// never passes through here, so they may sit outside the zoom and pitch
// limits, or inside a building. Only the wheel links pitch to zoom.

import { clamp } from "math";
import type { Camera3DParams } from "./camera3d";
import { resolveClearance, type ClearanceState, type ClearanceTuning } from "./cameraClearance";
import type { CameraObstacles } from "./cameraObstacles";

/** `presentation.camera` in the fixture. */
export interface CameraPresentation {
  /** The lens: vertical field of view, radians, and the near plane, metres. */
  fov_y: number;
  near_m: number;
  /** Nearest and farthest orbit distance the wheel reaches, metres. */
  zoom_min: number;
  zoom_max: number;
  /** [orbit distance m, pitch rad] points, ascending in distance, pitch never
   *  falling; interpolated in log distance and covering the zoom range. */
  pitch_curve: readonly (readonly [number, number])[];
  /** Pan speed in orbit distances per second, so it scales with zoom. */
  pan_speed: number;
  /** Q/E turn and Z/X tilt rate, rad/s. */
  rotate_speed: number;
  /** Log-distance change per wheel pixel (deltaY). */
  zoom_speed: number;
  /** Drag turn, radians per viewport height dragged. */
  drag_speed: number;
  /** What a drag turns about: the eye (the player looks round from where
   *  they stand) or the target (the eye swings round what it looks at). */
  drag_pivot: "eye" | "target";
  /** Orbit pitch limits, radians: lowest and highest. */
  pitch_limits: readonly [number, number];
  /** How the drawn pose keeps clear of buildings and the ground. */
  clearance: ClearanceTuning;
}

/** What the player asks of the camera since the last step. Every field is optional. */
export interface CameraIntent {
  /** Held keys, by `KeyboardEvent.code`; only `CAMERA_KEYS` act. */
  held?: ReadonlySet<string>;
  /** Screen-edge pan axes: right and forward, each in [-1, 1]. */
  edge?: readonly [number, number];
  /** Wheel `deltaY` pixels; positive zooms out. */
  wheel?: number;
  /** A drag in viewport heights: right and down. Right turns the view right
   *  and down tilts it down, about `drag_pivot`. */
  drag?: readonly [number, number];
}

/** A framing asked for by a script rather than input: the orbit target on the
 *  ground plane, distance in metres, yaw and pitch in radians. */
export interface CameraPose {
  target: readonly [number, number];
  distance: number;
  yaw: number;
  pitch: number;
}

type CameraAxis = "right" | "forward" | "turn" | "pitch";

/** The parts of a camera a drag changes. */
type OrbitPose = Pick<Camera3DParams, "target" | "distance" | "pitch" | "yaw">;

/** The camera's keys: WASD and arrows pan, Q/E turn, Z/X tilt. */
export const CAMERA_KEYS: Readonly<Record<string, readonly [CameraAxis, number]>> = {
  KeyW: ["forward", 1],
  ArrowUp: ["forward", 1],
  KeyS: ["forward", -1],
  ArrowDown: ["forward", -1],
  KeyD: ["right", 1],
  ArrowRight: ["right", 1],
  KeyA: ["right", -1],
  ArrowLeft: ["right", -1],
  KeyQ: ["turn", 1],
  KeyE: ["turn", -1],
  KeyZ: ["pitch", -1],
  KeyX: ["pitch", 1],
};

/** The battle camera's pitch limits: just above the horizon to just short
 *  of top-down. */
export const PITCH_LIMITS: readonly [number, number] = [0.12, Math.PI / 2 - 0.02];

export class CameraController {
  readonly config: CameraPresentation;
  private readonly groundAt?: (x: number, y: number) => number;

  /** `groundAt` keeps the orbit target on the ground as the camera moves. */
  constructor(config: CameraPresentation, groundAt?: (x: number, y: number) => number) {
    this.config = config;
    this.groundAt = groundAt;
  }

  /** The curve's pitch at an orbit distance (log-distance interpolation, clamped). */
  pitchAt(distance: number): number {
    const curve = this.config.pitch_curve;
    if (distance <= curve[0][0]) return curve[0][1];
    for (let k = 1; k < curve.length; k++) {
      const [d1, p1] = curve[k];
      if (distance <= d1) {
        const [d0, p0] = curve[k - 1];
        const t = Math.log(distance / d0) / Math.log(d1 / d0);
        return p0 + (p1 - p0) * t;
      }
    }
    return curve.at(-1)![1];
  }

  /** A scripted framing (the benchmark tour) as a camera the rig could reach:
   *  distance within the zoom range, pitch within its limits, the target on
   *  the ground. Yaw is kept as given, unwrapped. */
  place(camera: Camera3DParams, pose: CameraPose): Camera3DParams {
    const [x, y] = pose.target;
    return {
      ...camera,
      target: [x, y, this.groundAt ? this.groundAt(x, y) : camera.target[2]],
      distance: clamp(pose.distance, this.config.zoom_min, this.config.zoom_max),
      pitch: clamp(pose.pitch, ...this.config.pitch_limits),
      yaw: pose.yaw,
    };
  }

  /** The pose to draw for `desired` (from `step` or `place`), `dt` seconds
   *  after the last call with this `state`: `desired` itself when it is clear
   *  of `obstacles`, else lifted, slid or pushed clear of them. */
  resolve(
    state: ClearanceState,
    desired: Camera3DParams,
    dt: number,
    obstacles: CameraObstacles,
  ): Camera3DParams {
    const maxPitch = this.config.pitch_limits[1];
    return resolveClearance(state, desired, dt, obstacles, this.config.clearance, maxPitch);
  }

  /** The next camera after `dt` seconds of `intent`; the same object when
   *  nothing moves, so an idle viewport never redraws. */
  step(camera: Camera3DParams, intent: CameraIntent, dt: number): Camera3DParams {
    const c = this.config;
    let right = intent.edge?.[0] ?? 0;
    let forward = intent.edge?.[1] ?? 0;
    let turn = 0;
    let pitchTurn = 0;
    for (const code of intent.held ?? []) {
      const key = CAMERA_KEYS[code];
      if (!key) continue;
      if (key[0] === "right") right += key[1];
      else if (key[0] === "forward") forward += key[1];
      else if (key[0] === "turn") turn += key[1];
      else pitchTurn += key[1];
    }
    right = clamp(right, -1, 1);
    forward = clamp(forward, -1, 1);
    turn = clamp(turn, -1, 1);
    pitchTurn = clamp(pitchTurn, -1, 1);
    const wheel = intent.wheel ?? 0;
    const [dragX, dragY] = intent.drag ?? [0, 0];
    if (!right && !forward && !turn && !pitchTurn && !wheel && !dragX && !dragY) return camera;

    let { distance, pitch, yaw } = camera;
    const [low, high] = c.pitch_limits;
    let [x, y, z] = camera.target;
    if (wheel) {
      // A look can leave the target nearer than the wheel reaches; the wheel
      // never snaps it back, only stops at whichever limit it starts past.
      const next = clamp(
        distance * Math.exp(wheel * c.zoom_speed),
        Math.min(c.zoom_min, distance),
        Math.max(c.zoom_max, distance),
      );
      // The wheel carries pitch along the curve, keeping any tilt the player dragged in.
      pitch = clamp(pitch + this.pitchAt(next) - this.pitchAt(distance), low, high);
      distance = next;
    }
    if (dragX || dragY) {
      const [right, down] = [dragX * c.drag_speed, dragY * c.drag_speed];
      if (c.drag_pivot === "target") {
        yaw -= right;
        pitch = clamp(pitch + down, low, high);
      } else {
        const pose = this.look({ target: [x, y, z], distance, pitch, yaw }, right, down);
        ({
          target: [x, y, z],
          distance,
          pitch,
          yaw,
        } = pose);
      }
    }
    yaw += turn * c.rotate_speed * dt;
    pitch = clamp(pitch + pitchTurn * c.rotate_speed * dt, low, high);
    const length = Math.hypot(right, forward);
    if (length > 0) {
      // Diagonals are no faster than straight lines.
      const boost = intent.held?.has("ShiftLeft") || intent.held?.has("ShiftRight") ? 3 : 1;
      const step = (c.pan_speed * distance * dt * boost) / Math.max(1, length);
      // The eye sits at yaw around the target: screen-forward is −(cos yaw, sin yaw),
      // screen-right is forward × up.
      const fx = -Math.cos(yaw),
        fy = -Math.sin(yaw);
      x += (fy * right + fx * forward) * step;
      y += (-fx * right + fy * forward) * step;
    }
    if (this.groundAt) z = this.groundAt(x, y);
    return { ...camera, target: [x, y, z], distance, pitch, yaw };
  }

  /** `pose` turned `right` and tilted `down` radians about its eye: the eye
   *  stays, and the target moves to where the new view meets the ground.
   *  The tilt stops at the pitch limits, and where looking up would carry the
   *  target past the farthest zoom. */
  private look(pose: OrbitPose, right: number, down: number): OrbitPose {
    const c = this.config;
    const [tx, ty, tz] = pose.target;
    const flat = pose.distance * Math.cos(pose.pitch);
    const eye = [
      tx + flat * Math.cos(pose.yaw),
      ty + flat * Math.sin(pose.yaw),
      tz + pose.distance * Math.sin(pose.pitch),
    ];
    const yaw = pose.yaw - right;
    const [low, high] = c.pitch_limits;
    // Like the wheel, a pose already past the farthest zoom is not snapped.
    const farthest = Math.asin(Math.min(1, (eye[2] - tz) / c.zoom_max));
    const lowest = Math.max(low, Math.min(pose.pitch, farthest));
    const pitch = clamp(pose.pitch + down, Math.min(lowest, high), high);
    // The view's ground point, refined as the ground under it rises or falls.
    let [x, y, z] = [tx, ty, tz];
    for (let k = 0; k < 3; k++) {
      const reach = Math.max(0, eye[2] - z) / Math.tan(pitch);
      x = eye[0] - reach * Math.cos(yaw);
      y = eye[1] - reach * Math.sin(yaw);
      if (!this.groundAt) break;
      z = this.groundAt(x, y);
    }
    const distance = Math.max(c.near_m, eye[2] - z) / Math.sin(pitch);
    return { target: [x, y, z], distance, pitch, yaw };
  }
}
