// The player camera: a Total War style rig (battle-look I5, I6). Held keys,
// the wheel, middle-drag and the screen edge become one CameraIntent; one
// step turns it, with elapsed time and the fixture's `presentation.camera`,
// into the next camera3d parameter set. Pure and GPU-free.
//
// Scene-authored framings go through the viewport's raw `setCamera`, which
// never passes through here, so they may sit outside the zoom and pitch
// limits. Only the wheel links pitch to zoom.

import type { Camera3DParams } from "./camera3d";

/** `presentation.camera` in the fixture. */
export interface CameraPresentation {
  /** Nearest and farthest orbit distance the wheel reaches, metres. */
  zoom_min: number;
  zoom_max: number;
  /** [orbit distance m, pitch rad] points, ascending in distance, pitch never
   *  falling; interpolated in log distance and covering the zoom range. */
  pitch_curve: readonly (readonly [number, number])[];
  /** Pan speed in orbit distances per second, so it scales with zoom. */
  pan_speed: number;
  /** Q/E turn rate, rad/s. */
  rotate_speed: number;
  /** Log-distance change per wheel pixel (deltaY). */
  zoom_speed: number;
  /** Middle-drag turn, radians per viewport height dragged. */
  orbit_speed: number;
}

/** What the player asks of the camera since the last step. Every field is optional. */
export interface CameraIntent {
  /** Held keys, by `KeyboardEvent.code`; only `CAMERA_KEYS` act. */
  held?: ReadonlySet<string>;
  /** Screen-edge pan axes: right and forward, each in [-1, 1]. */
  edge?: readonly [number, number];
  /** Wheel `deltaY` pixels; positive zooms out. */
  wheel?: number;
  /** Middle-drag in viewport heights: right and down. */
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

type CameraAxis = "right" | "forward" | "turn";

/** The camera's keys: WASD and the arrows pan, Q/E turn the view left/right. */
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
};

/** Orbit pitch limits: just above the horizon to just short of top-down. */
export const PITCH_LIMITS: readonly [number, number] = [0.12, Math.PI / 2 - 0.02];

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

export class CameraController {
  readonly config: CameraPresentation;
  private readonly groundAt?: (x: number, y: number) => number;

  /** `groundAt` keeps the orbit target on the ground as the camera moves. */
  constructor(config: CameraPresentation, groundAt?: (x: number, y: number) => number) {
    const curve = config.pitch_curve;
    if (curve.length < 1) throw new Error("camera pitch_curve is empty");
    for (let k = 1; k < curve.length; k++)
      if (!(curve[k][0] > curve[k - 1][0]) || curve[k][1] < curve[k - 1][1])
        throw new Error("camera pitch_curve must be monotonic: ascending distance, rising pitch");
    for (const [, pitch] of curve)
      if (pitch < PITCH_LIMITS[0] || pitch > PITCH_LIMITS[1])
        throw new Error(`camera pitch_curve leaves the pitch limits ${PITCH_LIMITS.join("–")}`);
    if (
      !(config.zoom_min > 0 && config.zoom_min < config.zoom_max) ||
      curve[0][0] > config.zoom_min ||
      curve.at(-1)![0] < config.zoom_max
    )
      throw new Error("camera pitch_curve must cover the zoom range zoom_min..zoom_max");
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
      pitch: clamp(pose.pitch, PITCH_LIMITS[0], PITCH_LIMITS[1]),
      yaw: pose.yaw,
    };
  }

  /** The next camera after `dt` seconds of `intent`; the same object when
   *  nothing moves, so an idle viewport never redraws. */
  step(camera: Camera3DParams, intent: CameraIntent, dt: number): Camera3DParams {
    const c = this.config;
    let right = intent.edge?.[0] ?? 0;
    let forward = intent.edge?.[1] ?? 0;
    let turn = 0;
    for (const code of intent.held ?? []) {
      const key = CAMERA_KEYS[code];
      if (!key) continue;
      if (key[0] === "right") right += key[1];
      else if (key[0] === "forward") forward += key[1];
      else turn += key[1];
    }
    right = clamp(right, -1, 1);
    forward = clamp(forward, -1, 1);
    turn = clamp(turn, -1, 1);
    const wheel = intent.wheel ?? 0;
    const [dragX, dragY] = intent.drag ?? [0, 0];
    if (!right && !forward && !turn && !wheel && !dragX && !dragY) return camera;

    let { distance, pitch, yaw } = camera;
    let [x, y, z] = camera.target;
    if (wheel) {
      const next = clamp(distance * Math.exp(wheel * c.zoom_speed), c.zoom_min, c.zoom_max);
      // The wheel carries pitch along the curve, keeping any tilt the player dragged in.
      pitch = clamp(
        pitch + this.pitchAt(next) - this.pitchAt(distance),
        PITCH_LIMITS[0],
        PITCH_LIMITS[1],
      );
      distance = next;
    }
    if (dragX || dragY) {
      yaw -= dragX * c.orbit_speed;
      pitch = clamp(pitch + dragY * c.orbit_speed, PITCH_LIMITS[0], PITCH_LIMITS[1]);
    }
    yaw += turn * c.rotate_speed * dt;
    const length = Math.hypot(right, forward);
    if (length > 0) {
      // Diagonals are no faster than straight lines.
      const step = (c.pan_speed * distance * dt) / Math.max(1, length);
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
}
