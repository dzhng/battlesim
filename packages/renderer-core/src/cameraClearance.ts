// Camera clearance: the pose to draw for the pose the player (or a script)
// asks for, so that the eye and the near plane round it never enter a
// building or the ground. One pure resolver over the desired orbit pose, the
// eye it last emitted, elapsed time and an obstacle view; every camera path
// (input, scripted placement, the opening framing, recovery while idle) goes
// through it.
//
// The target is never moved, so the player keeps looking at what they chose:
// only the eye is. Where the eye should go is a policy, in order:
//
//   1. where the desired pose puts it, when that is clear;
//   2. a nearby clear pose: the smallest clear lift (pitch up the orbit arc),
//      then the smallest clear slide (yaw round the target), each within a
//      bounded reach and step count;
//   3. pushback: the desired eye pushed out of whatever it is in.
//
// The policy looks a little way ahead along the desired pose's own motion,
// so a lift or a slide starts before the building it is for arrives.
//
// The eye eases toward that goal as a critically damped spring. While the
// eased point is not itself clear the eye is pushed out of what it meets, on
// the side it is already on, so it slides along a wall or over a roof edge
// instead of passing through. What is emitted is therefore always clear. An
// eye held by something that stands between it and its goal goes over it
// when that is within a lift's reach; past anything taller it cuts to the
// goal, and the step is reported as a cut.
import { lerp, vec3, type Vec3 } from "math";
import { spring3, type Spring } from "math/time";
import { eyePosition, type Camera3DParams } from "./camera3d";
import type { CameraObstacles } from "./cameraObstacles";

/** `presentation.camera.clearance` in the fixture. */
export interface ClearanceTuning {
  /** Clear space kept beyond the near plane's envelope, metres. */
  margin_m: number;
  /** Extra clear space the desired pose needs before an adjusted camera
   *  returns to it, metres: the hysteresis that stops a pose grazing an
   *  obstacle from switching the adjustment on and off. */
  release_m: number;
  /** The farthest the eye is lifted up its orbit arc, metres, and the steps
   *  that reach is tried in. */
  lift_max_m: number;
  lift_steps: number;
  /** The farthest the eye is slid round the target, metres along its orbit
   *  circle and never more than `slide_max_rad`, and the steps each way. */
  slide_max_m: number;
  slide_max_rad: number;
  slide_steps: number;
  /** Halvings that tighten a found lift or slide toward the obstacle. */
  refine_steps: number;
  /** The spring's smooth time, seconds: about how long the eye takes to
   *  reach a new goal. */
  smoothing_s: number;
  /** How far ahead along the desired pose's own motion the policy looks
   *  for buildings, seconds, so the eye starts round or over what it is
   *  about to meet before it gets there; 0 looks no further than the pose
   *  itself. */
  lookahead_s: number;
  /** The longest eye move resolved as one step, metres, and the most steps a
   *  frame is split into; a longer move is a cut, placed without a sweep. */
  sweep_step_m: number;
  sweep_max_steps: number;
}

/** Throws unless `t` is a usable tuning. */
export function validateClearanceTuning(t: ClearanceTuning): ClearanceTuning {
  const positive: (keyof ClearanceTuning)[] = [
    "lift_max_m",
    "slide_max_m",
    "slide_max_rad",
    "smoothing_s",
    "sweep_step_m",
  ];
  const counts: (keyof ClearanceTuning)[] = ["lift_steps", "slide_steps", "sweep_max_steps"];
  for (const key of positive)
    if (!(t[key] > 0)) throw new Error(`camera clearance ${key} must be positive`);
  for (const key of counts)
    if (!(Number.isInteger(t[key]) && t[key] >= 1 && t[key] <= 64))
      throw new Error(`camera clearance ${key} must be a whole number from 1 to 64`);
  if (!(Number.isInteger(t.refine_steps) && t.refine_steps >= 0 && t.refine_steps <= 16))
    throw new Error("camera clearance refine_steps must be a whole number from 0 to 16");
  if (!(t.margin_m >= 0 && t.release_m > 0))
    throw new Error("camera clearance margin_m must not be negative and release_m positive");
  if (!(t.lookahead_s >= 0)) throw new Error("camera clearance lookahead_s must not be negative");
  return t;
}

/** What holds the eye off the desired pose. */
export type ClearanceHold = "none" | "lift" | "slide" | "pushback";

/** What the resolver remembers between frames; the caller owns it. */
export interface ClearanceState {
  /** The desired pose last resolved; null before the first (a placement). */
  desired: Camera3DParams | null;
  /** Where the eye is heading, as an offset from the desired eye, metres. */
  goal: Vec3;
  /** The offset easing toward the goal. */
  eased: Spring<Vec3>;
  hold: ClearanceHold;
  /** The height the eye is passing over what stands between it and its goal
   *  at; −Infinity while nothing does. */
  over: number;
  /** The eye last emitted. */
  eye: Vec3;
  /** The eye is against something: it was pushed out of what its easing
   *  would have carried it into. */
  pressed: boolean;
  /** Nothing is left to move: the same desired pose resolves the same again. */
  settled: boolean;
  /** The last step was a cut: no clear straight move led from the eye before it. */
  cut: boolean;
  /** The pose emitted is not clear. It never is over buildings on ground the
   *  eye can rise above; a check, not a mode. */
  blocked: boolean;
}

export function createClearanceState(): ClearanceState {
  return {
    desired: null,
    goal: vec3.create(),
    eased: spring3.create(),
    hold: "none",
    over: -Infinity,
    eye: vec3.create(),
    pressed: false,
    settled: false,
    cut: false,
    blocked: false,
  };
}

/** The radius of the sphere round the eye that holds the near plane's
 *  rectangle: geometry outside it is never cut by the near plane. */
export function nearEnvelope(camera: Camera3DParams): number {
  const tan = Math.tan(camera.fovY / 2);
  return camera.near * Math.sqrt(1 + tan * tan * (1 + camera.aspect * camera.aspect));
}

/** An eased offset this near its goal (metres), and this slow (metres a
 *  second), has arrived. */
const ARRIVED = 1e-3;
/** An eye this near the vertical through its target (metres) has no yaw of
 *  its own. */
const UPRIGHT_M = 1e-6;

const NONE: Vec3 = [0, 0, 0];
const _probe = { target: vec3.create(), distance: 0, pitch: 0, yaw: 0 } as Camera3DParams;
const _probe_eye = vec3.create();
const _slide_eye = vec3.create();
const _goal_eye = vec3.create();
const _desired_eye = vec3.create();
const _eased_eye = vec3.create();
const _step_eye = vec3.create();
const _last_offset = vec3.create();
const _step_target = vec3.create();
const _step_pose = { target: vec3.create() } as Camera3DParams;
const _from_eye = vec3.create();
const _to_eye = vec3.create();
const _ahead = { target: vec3.create(), distance: 0, pitch: 0, yaw: 0 } as Camera3DParams;
const _ahead_eye = vec3.create();
const _rate_target = vec3.create();

/** What one resolve call reads throughout. */
interface Resolve {
  obstacles: CameraObstacles;
  tuning: ClearanceTuning;
  maxPitch: number;
  /** The near plane's envelope, the clear space kept round the eye, and the
   *  clear space a goal keeps: the release margin more, so the easing eye
   *  reaches clear space in finite time and a held goal is one the camera
   *  could be released at. */
  envelope: number;
  radius: number;
  goalRadius: number;
  /** The eye last emitted, which a nearby pose must be reachable from; null
   *  for a placement, which may land anywhere clear. */
  from: Vec3 | null;
  /** Whether the policy looks ahead: not when the desired pose is not known
   *  to be changing (a placement, a frame of no time, no lookahead). */
  looking: boolean;
  /** How the desired pose is changing, a second: its target, the log of its
   *  distance, its pitch and yaw. */
  rateTarget: Vec3;
  rateDistance: number;
  ratePitch: number;
  rateYaw: number;
}

/** The eye of `desired` lifted by `lift` rad of pitch and slid by `slide` rad of yaw. */
function orbitEye(out: Vec3, desired: Camera3DParams, lift: number, slide: number): Vec3 {
  _probe.target = desired.target;
  _probe.distance = desired.distance;
  _probe.pitch = desired.pitch + lift;
  _probe.yaw = desired.yaw + slide;
  return eyePosition(out, _probe);
}

/** Where `desired` will be `lookahead_s` on, at the rate it is changing:
 *  into `_ahead`. */
function lookAhead(r: Resolve, desired: Camera3DParams): void {
  const t = r.tuning.lookahead_s;
  vec3.scaleAndAdd(_ahead.target, desired.target, r.rateTarget, t);
  _ahead.distance = desired.distance * Math.exp(r.rateDistance * t);
  _ahead.pitch = desired.pitch + r.ratePitch * t;
  _ahead.yaw = desired.yaw + r.rateYaw * t;
}

/** Whether the straight way from `eye` to where the same adjustment (`lift`,
 *  `slide`) puts the eye of the pose looked ahead to (`_ahead`) keeps
 *  `clearance` from every building. The ground is not looked ahead for: the
 *  eye rides up it as it comes. */
function clearAhead(
  r: Resolve,
  eye: Vec3,
  lift: number,
  slide: number,
  clearance: number,
): boolean {
  return (
    !r.looking || r.obstacles.sweepClear(eye, orbitEye(_ahead_eye, _ahead, lift, slide), clearance)
  );
}

/** The height the eye at `from` would pass over what stands between it and
 *  `to` at; −Infinity when the straight way is clear, Infinity when what
 *  stands there is beyond a lift's reach above it. */
function wayOver(r: Resolve, from: Vec3, to: Vec3): number {
  const top = r.obstacles.sweepTop(from, to, r.envelope);
  if (top === -Infinity) return top;
  const over = top + r.goalRadius;
  return over - from[2] <= r.tuning.lift_max_m ? over : Infinity;
}

/** Whether `share` of the adjustment (`lift`, `slide`) puts the eye in the
 *  clear, now and on the way the desired pose is heading, somewhere the
 *  camera can get to without a cut. */
function clearAt(
  r: Resolve,
  desired: Camera3DParams,
  lift: number,
  slide: number,
  share: number,
): boolean {
  const eye = orbitEye(_probe_eye, desired, lift * share, slide * share);
  return (
    r.obstacles.clear(eye, r.goalRadius) &&
    clearAhead(r, eye, lift * share, slide * share, r.goalRadius) &&
    (r.from === null || wayOver(r, r.from, eye) < Infinity)
  );
}

/**
 * The smallest clear share of the adjustment (`lift`, `slide`), tried in
 * `steps` equal steps and then tightened by halving. Its eye is written to
 * `out` and the share returned; 0 when no step is clear.
 */
function seek(
  out: Vec3,
  r: Resolve,
  desired: Camera3DParams,
  lift: number,
  slide: number,
  steps: number,
): number {
  for (let k = 1; k <= steps; k++) {
    if (!clearAt(r, desired, lift, slide, k / steps)) continue;
    let blocked = (k - 1) / steps;
    let clear = k / steps;
    for (let h = 0; h < r.tuning.refine_steps; h++) {
      const mid = (blocked + clear) / 2;
      if (clearAt(r, desired, lift, slide, mid)) clear = mid;
      else blocked = mid;
    }
    orbitEye(out, desired, lift * clear, slide * clear);
    return clear;
  }
  return 0;
}

/** The smallest clear lift up the orbit arc: its eye into `out`. */
function seekLift(out: Vec3, r: Resolve, desired: Camera3DParams): boolean {
  const most = Math.min(r.tuning.lift_max_m / desired.distance, r.maxPitch - desired.pitch);
  return most > 0 && seek(out, r, desired, most, 0, r.tuning.lift_steps) > 0;
}

/** The smallest clear slide round the target, on the nearer side: its eye
 *  into `out`. */
function seekSlide(out: Vec3, r: Resolve, desired: Camera3DParams): boolean {
  const { slide_max_m, slide_max_rad, slide_steps } = r.tuning;
  const circle = desired.distance * Math.cos(desired.pitch);
  const most = Math.min(slide_max_m / Math.max(circle, UPRIGHT_M), slide_max_rad);
  const one = seek(_slide_eye, r, desired, 0, most, slide_steps);
  const other = seek(out, r, desired, 0, -most, slide_steps);
  if (one > 0 && (other === 0 || one <= other)) vec3.copy(out, _slide_eye);
  return one > 0 || other > 0;
}

/** The nearby clear pose for `desired`, its eye into `_goal_eye`: a lift,
 *  then a slide. Null when neither reaches clear space. */
function seekNearby(r: Resolve, desired: Camera3DParams): "lift" | "slide" | null {
  if (seekLift(_goal_eye, r, desired)) return "lift";
  if (seekSlide(_goal_eye, r, desired)) return "slide";
  return null;
}

/** Choose `state.goal` and `state.hold` for `desired`, whose own eye is
 *  `eye`: the policy. */
function chooseGoal(state: ClearanceState, r: Resolve, desired: Camera3DParams, eye: Vec3): void {
  // Returning to the desired pose needs the release margin as well.
  const clearance = state.hold === "none" ? r.radius : r.goalRadius;
  const here = r.obstacles.clear(eye, clearance);
  let nearby = here && clearAhead(r, eye, 0, 0, clearance) ? null : seekNearby(r, desired);
  if (!nearby && !here && r.looking) {
    // Nothing nearby is clear all the way ahead: one that is clear here will do.
    r.looking = false;
    nearby = seekNearby(r, desired);
    r.looking = true;
  }
  if (nearby) state.hold = nearby;
  else if (here) {
    // Clear, and either the way ahead is too or nothing nearby would clear
    // it: the pose asked for, until what is coming arrives.
    vec3.zero(state.goal);
    state.hold = "none";
    return;
  } else {
    // Pushback: out of what the eye is in, on the side the camera is on. A
    // placement has no side yet: the one its target is on, so it sees it.
    r.obstacles.pushOut(_goal_eye, eye, r.from ?? desired.target, r.goalRadius);
    state.hold = "pushback";
  }
  vec3.subtract(state.goal, _goal_eye, eye);
}

/** The eased offset jumps to the goal and rests there. */
function arrive(state: ClearanceState): void {
  vec3.copy(state.eased.value, state.goal);
  vec3.zero(state.eased.velocity);
}

/**
 * One step: choose the goal for `desired` and ease the eye toward it for `dt`
 * seconds, emitting only a clear eye into `state.eye`. An eye held by
 * something that stands between it and its goal goes over it when that is
 * within a lift's reach, and cuts to the goal when it is not. `swept` is
 * false for a placement, which takes the goal at once.
 */
function advance(
  state: ClearanceState,
  r: Resolve,
  desired: Camera3DParams,
  dt: number,
  swept: boolean,
): void {
  const eye = eyePosition(_desired_eye, desired);
  if (r.looking) lookAhead(r, desired);
  r.from = swept ? state.eye : null;
  chooseGoal(state, r, desired, eye);
  const { eased } = state;
  vec3.copy(_last_offset, eased.value);
  if (swept) {
    // While something stands in the way the eye heads for the goal's place
    // at the height that passes over it.
    vec3.copy(_step_target, state.goal);
    _step_target[2] = Math.max(_step_target[2], state.over - eye[2]);
    spring3.damp(eased, _step_target, r.tuning.smoothing_s, dt);
    if (vec3.distance(eased.value, state.goal) < ARRIVED && vec3.length(eased.velocity) < ARRIVED)
      arrive(state);
  } else arrive(state);
  vec3.add(_eased_eye, eye, eased.value);
  const free = r.obstacles.clear(_eased_eye, r.radius);
  if (free) vec3.copy(_step_eye, _eased_eye);
  else {
    // The eased point is in something: the eye is pushed out of it, on the
    // side it is already on, and the easing carries on from where it is
    // held, at the speed it was let move.
    r.obstacles.pushOut(_step_eye, _eased_eye, swept ? state.eye : _eased_eye, r.radius);
    vec3.subtract(eased.value, _step_eye, eye);
    if (dt > 0)
      vec3.scale(eased.velocity, vec3.subtract(eased.velocity, eased.value, _last_offset), 1 / dt);
  }
  state.cut = swept && !r.obstacles.sweepClear(state.eye, _step_eye, r.envelope);
  // A held eye, or one already on its way over something, looks at what
  // stands between it and its goal.
  vec3.add(_goal_eye, eye, state.goal);
  const over = free && state.over === -Infinity ? -Infinity : wayOver(r, _step_eye, _goal_eye);
  if (over === Infinity) {
    vec3.copy(_step_eye, _goal_eye);
    arrive(state);
    state.cut = swept;
  }
  state.over = Number.isFinite(over) ? Math.max(state.over, over) : -Infinity;
  state.pressed = !free && over !== Infinity;
  vec3.copy(state.eye, _step_eye);
  state.settled =
    free && vec3.exactEquals(eased.value, state.goal) && vec3.exactEquals(eased.velocity, NONE);
}

/**
 * The pose to draw for `desired`, `dt` seconds after the last call with this
 * `state`: `desired` itself (the same object) when it is clear and nothing is
 * left to recover, else a new pose looking at the same target from a clear
 * eye. The first call, and a move too long to sweep, are placements: the goal
 * is taken at once. `maxPitch` is the steepest pitch a lift may reach.
 */
export function resolveClearance(
  state: ClearanceState,
  desired: Camera3DParams,
  dt: number,
  obstacles: CameraObstacles,
  tuning: ClearanceTuning,
  maxPitch: number,
): Camera3DParams {
  const envelope = nearEnvelope(desired);
  const radius = envelope + tuning.margin_m;
  const from = state.desired;
  state.desired = desired;
  const looking = from !== null && dt > 0 && tuning.lookahead_s > 0;
  const r: Resolve = {
    obstacles,
    tuning,
    maxPitch,
    envelope,
    radius,
    goalRadius: radius + tuning.release_m,
    from: null,
    looking,
    rateTarget: looking
      ? vec3.scale(_rate_target, vec3.subtract(_rate_target, desired.target, from.target), 1 / dt)
      : NONE,
    rateDistance: looking ? Math.log(desired.distance / from.distance) / dt : 0,
    ratePitch: looking ? (desired.pitch - from.pitch) / dt : 0,
    rateYaw: looking ? (desired.yaw - from.yaw) / dt : 0,
  };
  const steps = from
    ? Math.max(
        1,
        Math.ceil(
          vec3.distance(eyePosition(_from_eye, from), eyePosition(_to_eye, desired)) /
            tuning.sweep_step_m,
        ),
      )
    : 0;
  if (!from || steps > tuning.sweep_max_steps) {
    r.looking = false;
    advance(state, r, desired, 0, false);
    state.cut = from !== null;
  } else {
    // The desired pose moves in steps no longer than `sweep_step_m`, so a
    // fast input or a long frame meets what a slow one would.
    let cut = false;
    for (let k = 1; k < steps; k++) {
      const t = k / steps;
      vec3.lerp(_step_pose.target, from.target, desired.target, t);
      _step_pose.distance = from.distance * (desired.distance / from.distance) ** t;
      _step_pose.pitch = lerp(from.pitch, desired.pitch, t);
      _step_pose.yaw = lerp(from.yaw, desired.yaw, t);
      advance(state, r, _step_pose, dt / steps, true);
      cut ||= state.cut;
    }
    advance(state, r, desired, dt / steps, true);
    state.cut ||= cut;
  }
  state.blocked = false;
  if (vec3.exactEquals(state.eye, eyePosition(_to_eye, desired))) return desired;
  // The orbit pose that looks at the same target from the emitted eye.
  const [dx, dy, dz] = [
    state.eye[0] - desired.target[0],
    state.eye[1] - desired.target[1],
    state.eye[2] - desired.target[2],
  ];
  const across = Math.hypot(dx, dy);
  const turned = Math.atan2(dy, dx) - desired.yaw;
  const drawn = {
    ...desired,
    distance: Math.hypot(across, dz),
    pitch: Math.atan2(dz, across),
    // The same turn count as the yaw asked for.
    yaw:
      across > UPRIGHT_M
        ? desired.yaw + Math.atan2(Math.sin(turned), Math.cos(turned))
        : desired.yaw,
  };
  // The projection's own eye for that pose (it holds pitch just short of
  // vertical) is the one that must be clear.
  state.blocked = !obstacles.clear(eyePosition(state.eye, drawn), envelope);
  return drawn;
}
