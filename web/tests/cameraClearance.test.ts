// @vitest-environment node
import { expect, test } from "vitest";
import { vec3, type Vec3 } from "math";
import { eyePosition, type Camera3DParams } from "@packages/renderer-core/src/camera3d.ts";
import {
  createClearanceState,
  nearEnvelope,
  resolveClearance,
  validateClearanceTuning,
  type ClearanceState,
  type ClearanceTuning,
} from "@packages/renderer-core/src/cameraClearance.ts";
import {
  createCameraObstacles,
  type CameraObstacles,
  type ObstacleBox,
} from "@packages/renderer-core/src/cameraObstacles.ts";
import {
  knownStanding,
  type KnownProp,
  type MapProp,
} from "@packages/battle-renderer/src/models/propAppearance";

// Fixed numbers, so behaviour tests don't move when the fixture is tuned.
const TUNING: ClearanceTuning = {
  margin_m: 0.5,
  release_m: 0.75,
  lift_max_m: 14,
  lift_steps: 4,
  slide_max_m: 16,
  slide_max_rad: 0.6,
  slide_steps: 4,
  refine_steps: 6,
  smoothing_s: 0.25,
  lookahead_s: 0.5,
  sweep_step_m: 1,
  sweep_max_steps: 32,
};
const MAX_PITCH = Math.PI / 2 - 0.02;
const FLAT = () => 0;

/** A camera looking north (+y) at `target` from `distance` out: its eye is
 *  south of the target, `distance · cos(pitch)` behind it. */
const look = (
  x: number,
  y: number,
  distance = 30,
  pitch = 0.3,
  yaw = -Math.PI / 2,
): Camera3DParams => ({
  target: [x, y, 0],
  distance,
  pitch,
  yaw,
  fovY: 0.8,
  aspect: 16 / 9,
  near: 1,
});
const eyeOf = (pose: Camera3DParams) => eyePosition(vec3.create(), pose);
const ENVELOPE = nearEnvelope(look(0, 0));
/** How far behind its target the default camera's eye stands. */
const BEHIND = 30 * Math.cos(0.3);

/** Boxes round a point 40 m south of the origin: where the default camera's
 *  eye is when its target is on `ROW`. The eye (8.9 m up) passes under the
 *  house's roof line by less than its clearance, and deep inside the others. */
const HOUSE: ObstacleBox = { center: [0, -40], yaw: 0, half: [10, 6, 4], baseZ: 0 };
const TOWER: ObstacleBox = { center: [0, -40], yaw: 0, half: [8, 8, 30], baseZ: 0 };
const SLAB: ObstacleBox = { center: [0, -43], yaw: 0, half: [60, 8, 30], baseZ: 0 };
const ROW = BEHIND - 40;

/** The exact distance from `p` to the nearest box (0 inside one): the
 *  invariant's own measure, apart from the view's grid and grown boxes. */
function gap(p: Vec3, boxes: readonly ObstacleBox[]): number {
  let nearest = Infinity;
  for (const b of boxes) {
    const [cos, sin] = [Math.cos(b.yaw), Math.sin(b.yaw)];
    const [dx, dy] = [p[0] - b.center[0], p[1] - b.center[1]];
    const out: Vec3 = [
      Math.max(0, Math.abs(dx * cos + dy * sin) - b.half[0]),
      Math.max(0, Math.abs(dy * cos - dx * sin) - b.half[1]),
      Math.max(0, b.baseZ - p[2], p[2] - (b.baseZ + 2 * b.half[2])),
    ];
    nearest = Math.min(nearest, vec3.length(out));
  }
  return nearest;
}

/** Whether the straight line from `a` to `b` keeps the near plane's envelope
 *  out of every box, sampled every 5 cm. */
function lineClear(a: Vec3, b: Vec3, boxes: readonly ObstacleBox[]): boolean {
  const length = vec3.distance(a, b);
  for (let s = 0; s <= length; s += 0.05)
    if (gap(vec3.lerp(vec3.create(), a, b, s / Math.max(length, 1e-9)), boxes) < ENVELOPE - 0.01)
      return false;
  return true;
}

interface Flown {
  desired: Camera3DParams;
  drawn: Camera3DParams;
  eye: Vec3;
  hold: ClearanceState["hold"];
  cut: boolean;
}

/** A camera flown through `poses`, one frame of `dt` seconds each, over
 *  `boxes` on flat ground. Every drawn frame must look at the target asked
 *  for and keep the near plane's envelope out of every box and above the
 *  ground. So must every short move between two drawn frames that is not a
 *  cut (a long frame's move is swept along the path asked for, not along the
 *  straight line between its ends). */
function fly(
  poses: readonly Camera3DParams[],
  boxes: readonly ObstacleBox[],
  { dt = 1 / 60, tuning = TUNING, state = createClearanceState() } = {},
): { frames: Flown[]; state: ClearanceState; view: CameraObstacles } {
  const view = createCameraObstacles(boxes, FLAT);
  const frames: Flown[] = [];
  for (const desired of poses) {
    const drawn = resolveClearance(state, desired, dt, view, tuning, MAX_PITCH);
    const eye = eyeOf(drawn);
    const at = `frame ${frames.length}, eye ${eye}`;
    expect(state.blocked, at).toBe(false);
    expect(drawn.target, at).toEqual(desired.target);
    expect(gap(eye, boxes), at).toBeGreaterThanOrEqual(ENVELOPE);
    expect(eye[2], at).toBeGreaterThanOrEqual(ENVELOPE);
    const last = frames.at(-1);
    if (last && !state.cut && vec3.distance(last.eye, eye) <= 2 * tuning.sweep_step_m)
      expect(lineClear(last.eye, eye, boxes), `the move to ${at}`).toBe(true);
    frames.push({ desired, drawn, eye, hold: state.hold, cut: state.cut });
  }
  return { frames, state, view };
}

/** `n` frames from pose `a` to pose `b`, evenly. */
const path = (a: Camera3DParams, b: Camera3DParams, n: number): Camera3DParams[] =>
  Array.from({ length: n + 1 }, (_, k) => ({
    ...a,
    target: vec3.lerp(vec3.create(), a.target, b.target, k / n),
    distance: a.distance * (b.distance / a.distance) ** (k / n),
    pitch: a.pitch + ((b.pitch - a.pitch) * k) / n,
    yaw: a.yaw + ((b.yaw - a.yaw) * k) / n,
  }));
const hold = (pose: Camera3DParams, n: number): Camera3DParams[] =>
  Array.from({ length: n }, () => pose);
/** How many times the adjustment switched on or off. */
const switches = (frames: readonly Flown[]) =>
  frames.filter((f, k) => k > 0 && (f.hold === "none") !== (frames[k - 1].hold === "none")).length;

test("a clear pose is drawn as asked", () => {
  const asked = look(200, 200);
  const { frames, state } = fly([asked, asked], [HOUSE]);
  expect(frames[0].drawn).toBe(asked);
  expect(frames[1].drawn).toBe(asked);
  expect(state.settled).toBe(true);
});

test("a camera placed inside a slab comes out on the side its target is on; one carried in stays on its own", () => {
  // The eye is 3 m from the slab's south wall and 13 m from its north one;
  // the target is to the north.
  const slab: ObstacleBox = { ...SLAB, center: [0, -35] };
  const inside = look(0, ROW);
  const placed = fly([inside], [slab]).frames[0];
  expect(placed.eye[1]).toBeGreaterThan(-27 + ENVELOPE);
  // Dollied in from the south (the target leading), it is held at the south wall.
  const carried = fly(path(look(0, ROW - 30), inside, 120), [slab]).frames.at(-1)!;
  expect(carried.cut).toBe(false);
  expect(carried.eye[1]).toBeLessThan(-43 - ENVELOPE);
});

test("placed inside a building, the first pose drawn is already clear of it", () => {
  for (const box of [HOUSE, TOWER, SLAB]) {
    const inside = look(0, ROW);
    expect(gap(eyeOf(inside), [box])).toBeLessThan(ENVELOPE);
    const { frames, state } = fly([inside], [box]);
    expect(frames[0].drawn).not.toBe(inside);
    expect(state.settled).toBe(true);
  }
  // Looking straight down from inside a tower: its nearest wall is the way out.
  const under = look(0, -40, 30, 1.5);
  const out = fly([under], [TOWER]).frames[0];
  expect(vec3.distance(out.eye, eyeOf(under))).toBeLessThan(8 + ENVELOPE + 2);
});

test("a nearby pose is tried before pushback: lift, then slide, then out of the obstacle", () => {
  const inside = look(0, ROW);
  // A house the eye can rise over: only the pitch changes.
  const lifted = fly([inside], [HOUSE]).frames[0];
  expect(lifted.hold).toBe("lift");
  expect(lifted.drawn.pitch).toBeGreaterThan(inside.pitch + 0.01);
  expect(lifted.drawn.yaw).toBeCloseTo(inside.yaw, 9);
  expect(lifted.drawn.distance).toBeCloseTo(inside.distance, 9);
  // A tower too tall to rise over, narrow enough to step round: only the yaw.
  const slid = fly([inside], [TOWER]).frames[0];
  expect(slid.hold).toBe("slide");
  expect(Math.abs(slid.drawn.yaw - inside.yaw)).toBeGreaterThan(0.1);
  expect(slid.drawn.pitch).toBeCloseTo(inside.pitch, 9);
  expect(slid.drawn.distance).toBeCloseTo(inside.distance, 9);
  // A slab too tall and too wide for either: pushed out of it the nearest
  // way, its north face, which is off the orbit the pose asked for.
  const pushed = fly([inside], [SLAB]).frames[0];
  expect(pushed.hold).toBe("pushback");
  expect(pushed.eye[1]).toBeGreaterThan(-35 + ENVELOPE);
  expect(pushed.eye[1]).toBeLessThan(-35 + 4);
  expect(pushed.drawn.distance).toBeLessThan(inside.distance - 1);
});

test("the nearby poses are bounded: a lift or slide past its reach is not taken", () => {
  const inside = look(0, ROW);
  const slid = fly([inside], [TOWER]).frames[0];
  expect(Math.abs(slid.drawn.yaw - inside.yaw) * BEHIND).toBeLessThanOrEqual(TUNING.slide_max_m);
  // With a shorter reach the same tower cannot be stepped round, nor the
  // house risen over: pushback.
  const short = { ...TUNING, slide_max_m: 6, lift_max_m: 0.5 };
  expect(fly([inside], [TOWER], { tuning: short }).frames[0].hold).toBe("pushback");
  expect(fly([inside], [HOUSE], { tuning: short }).frames[0].hold).toBe("pushback");
});

test("the search is bounded: a frame asks the obstacle view a fixed number of questions", () => {
  const asked = { count: 0 };
  const boxes = createCameraObstacles([SLAB, HOUSE], FLAT);
  const counting: CameraObstacles = {
    count: boxes.count,
    groundAt: boxes.groundAt,
    ceiling: boxes.ceiling,
    tested: 0,
    clear: (p, c) => (asked.count++, boxes.clear(p, c)),
    sweepClear: (a, b, c) => (asked.count++, boxes.sweepClear(a, b, c)),
    sweepTop: (a, b, c) => (asked.count++, boxes.sweepTop(a, b, c)),
    pushOut: (out, p, toward, c) => (asked.count++, boxes.pushOut(out, p, toward, c)),
  };
  const t = TUNING;
  // One step asks three things of the desired pose and of each step and
  // halving of a lift and of a slide either way (is it clear, is the way
  // ahead, can the camera get there), twice when the way ahead rules them all
  // out; then pushes the goal and the eye out, and sweeps the eye's move and
  // its way to the goal.
  const nearby = t.lift_steps + 2 * t.slide_steps + 3 * t.refine_steps;
  const perStep = 3 * (1 + 2 * nearby) + 5;
  const state = createClearanceState();
  let worst = 0;
  for (const pose of path(look(-100, ROW), look(100, ROW + 10, 60, 0.5), 400)) {
    asked.count = 0;
    resolveClearance(state, pose, 1 / 60, counting, t, MAX_PITCH);
    worst = Math.max(worst, asked.count);
  }
  expect(worst).toBeGreaterThan(10);
  expect(worst).toBeLessThanOrEqual(perStep * t.sweep_max_steps + 1);
});

test("panning through a house lifts the eye over it and returns to the pose asked for", () => {
  const { frames, state } = fly(
    [...path(look(-40, ROW), look(40, ROW), 480), ...hold(look(40, ROW), 240)],
    [HOUSE],
  );
  const over = frames.filter((f) => Math.abs(f.desired.target[0]) < 8);
  expect(over.length).toBeGreaterThan(50);
  for (const f of over) {
    expect(f.hold).toBe("lift");
    expect(f.eye[2]).toBeGreaterThan(8 + ENVELOPE);
  }
  expect(frames.some((f) => f.cut)).toBe(false);
  expect(switches(frames)).toBe(2);
  expect(frames.at(-1)!.drawn).toBe(frames.at(-1)!.desired);
  expect(state.settled).toBe(true);
});

test("the eye starts over a house before it reaches it, and only when heading for it", () => {
  const heading = fly(path(look(-40, ROW), look(40, ROW), 480), [HOUSE]).frames;
  const first = heading.find((f) => f.hold === "lift")!;
  // Still short of the house's wall (at x = −10) by more than its clearance.
  expect(eyeOf(first.desired)[0]).toBeLessThan(-10 - 2 * ENVELOPE);
  // The same pan with no lookahead waits for the wall.
  const waiting = fly(path(look(-40, ROW), look(40, ROW), 480), [HOUSE], {
    tuning: { ...TUNING, lookahead_s: 0 },
  }).frames;
  const late = waiting.find((f) => f.hold !== "none")!;
  expect(eyeOf(late.desired)[0]).toBeGreaterThan(eyeOf(first.desired)[0] + 3);
  // A pan that stops short of the house comes back to the pose asked for.
  const short = fly(
    [...path(look(-40, ROW), look(-16, ROW), 150), ...hold(look(-16, ROW), 150)],
    [HOUSE],
  ).frames;
  expect(short.at(-1)!.drawn).toBe(short.at(-1)!.desired);
});

test("a long frame and a fast input meet what a slow one would", () => {
  // The same pan in 480 frames, 48 frames and 8 frames (10 m a frame: the
  // eye would jump the house's roof line in two).
  for (const n of [480, 48, 8]) {
    const { frames } = fly(path(look(-40, ROW), look(40, ROW), n), [HOUSE], { dt: 8 / n });
    expect(frames.some((f) => f.cut)).toBe(false);
    expect(frames.some((f) => f.hold === "lift")).toBe(true);
  }
  // One frame from one side of the house to the other, both ends clear: the
  // eye does not arrive through the house.
  const [before, across] = fly([look(-15, ROW), look(15, ROW)], [HOUSE]).frames;
  expect(lineClear(eyeOf(before.desired), eyeOf(across.desired), [HOUSE])).toBe(false);
  expect(across.cut).toBe(false);
  expect(lineClear(before.eye, across.eye, [HOUSE])).toBe(true);
  // Across a tower it does, and says so.
  expect(fly([look(-12, ROW), look(12, ROW)], [TOWER]).frames[1].cut).toBe(true);
  // A move too long to sweep is a cut, and still lands clear.
  const jump = fly([look(-400, ROW), look(0, ROW)], [HOUSE]).frames[1];
  expect(jump.cut).toBe(true);
  expect(jump.drawn).not.toBe(jump.desired);
});

test("a low building in the way is gone over; a tall one is cut past, and the cut is reported", () => {
  // A camera looking almost straight down from 8.9 m, panned across the
  // boxes: no lift or slide is left to it, so it is held at the near wall
  // until the pose asked for comes clear on the far side.
  const down = (x: number) => look(x, 8.9 * Math.cos(1.5) - 40, 8.9, 1.5);
  const pass = [...path(down(-30), down(30), 240), ...hold(down(30), 150)];
  const low = fly(pass, [HOUSE]).frames;
  expect(low.some((f) => f.hold === "pushback")).toBe(true);
  expect(low.some((f) => f.cut)).toBe(false);
  expect(Math.max(...low.map((f) => f.eye[2]))).toBeGreaterThan(8 + ENVELOPE);
  expect(low.at(-1)!.drawn).toBe(pass.at(-1));
  // A tower is too tall to go over: one cut, reported, to the far side.
  const tall = fly(pass, [TOWER]).frames;
  expect(tall.filter((f) => f.cut)).toHaveLength(1);
  expect(Math.max(...tall.map((f) => f.eye[2]))).toBeLessThan(12);
  expect(tall.at(-1)!.drawn).toBe(pass.at(-1));
});

test("reversing direction inside an obstacle stays clear and comes back out", () => {
  const out = look(-40, ROW);
  const { frames } = fly(
    [...path(out, look(0, ROW), 120), ...path(look(0, ROW), out, 120), ...hold(out, 120)],
    [TOWER],
  );
  expect(frames.some((f) => f.hold !== "none")).toBe(true);
  expect(frames.some((f) => f.cut)).toBe(false);
  expect(frames.at(-1)!.drawn).toBe(out);
});

test("rapid zoom, orbit and pan input stays clear", () => {
  // A seeded jitter of large steps: wheel notches, drags and shifted pans.
  let seed = 7;
  const random = () => ((seed = (seed * 16807) % 2147483647) / 2147483647) * 2 - 1;
  const poses = [look(0, 0, 60, 0.6)];
  for (let k = 0; k < 600; k++) {
    const p = poses.at(-1)!;
    poses.push({
      ...p,
      target: [
        Math.max(-60, Math.min(60, p.target[0] + random() * 6)),
        Math.max(-60, Math.min(20, p.target[1] + random() * 6)),
        0,
      ],
      distance: Math.max(25, Math.min(120, p.distance * Math.exp(random() * 0.3))),
      pitch: Math.max(0.15, Math.min(1.4, p.pitch + random() * 0.15)),
      yaw: p.yaw + random() * 0.4,
    });
  }
  const town: ObstacleBox[] = [
    HOUSE,
    { ...TOWER, center: [30, -10] },
    { ...SLAB, center: [0, 40], half: [40, 6, 9] },
    // An alley too narrow for the eye, beside the tower.
    { ...TOWER, center: [49, -10] },
  ];
  const { frames } = fly(poses, town);
  expect(frames.filter((f) => f.hold !== "none").length).toBeGreaterThan(50);
});

test("a pose grazing an obstacle does not switch the adjustment on and off", () => {
  // A held pan along a roof line, the eye's height wobbling a few
  // centimetres either side of where it first touches the clearance.
  const touch = Math.asin((8 + ENVELOPE + TUNING.margin_m) / 30);
  const wobble = (k: number) => touch + 0.004 * Math.sin(k * 1.7);
  const grazing = (release_m: number) =>
    fly(
      Array.from({ length: 600 }, (_, k) => {
        const p = wobble(k);
        // The eye stays over the house: the target is `30 cos p` north of it.
        return look(-8 + k / 40, 30 * Math.cos(p) - 40, 30, p);
      }),
      [HOUSE],
      // The wobble is the one variable: no lookahead along it.
      { tuning: { ...TUNING, release_m, lookahead_s: 0 } },
    ).frames;
  expect(switches(grazing(TUNING.release_m))).toBeLessThanOrEqual(1);
  // Without the release margin the same pan flickers.
  expect(switches(grazing(1e-6))).toBeGreaterThan(20);
});

test("an idle camera recovers on its own when the obstacle goes", () => {
  const inside = look(0, ROW);
  const { state, frames } = fly(hold(inside, 30), [TOWER]);
  expect(frames.at(-1)!.drawn).not.toBe(inside);
  expect(state.settled).toBe(true);
  // The tower is gone (the side saw it fall); no input arrives.
  const after = fly(hold(inside, 240), [], { state }).frames;
  const off = after.map((f) => vec3.distance(f.eye, eyeOf(inside)));
  expect(off[0]).toBeGreaterThan(5);
  for (let k = 1; k < off.length; k++) expect(off[k]).toBeLessThanOrEqual(off[k - 1]);
  // Smoothly: no frame gives back more than a fifth of what was left.
  for (let k = 1; k < 20; k++) expect(off[k]).toBeGreaterThan(off[k - 1] * 0.8);
  expect(after.at(-1)!.drawn).toBe(inside);
  expect(state.settled).toBe(true);
});

test("a pose under the ground is lifted above it", () => {
  // A hillside rising to the south, behind the camera.
  const hill = createCameraObstacles([], (_x, y) => Math.max(0, -y) * 0.6);
  const asked = look(0, 0);
  const state = createClearanceState();
  const drawn = resolveClearance(state, asked, 0, hill, TUNING, MAX_PITCH);
  const eye = eyeOf(drawn);
  expect(eyeOf(asked)[2]).toBeLessThan(BEHIND * 0.6);
  expect(eye[2] - hill.groundAt(eye[0], eye[1])).toBeGreaterThanOrEqual(ENVELOPE);
  expect(state.blocked).toBe(false);
});

test("a fall the side has not seen still blocks the camera; one it has seen does not", () => {
  const tower: MapProp = { id: 7, kind: "building", ...TOWER };
  const ruin: KnownProp = {
    kind: "ruin",
    center: TOWER.center,
    yaw: 0,
    half: [8, 8, 2],
    baseZ: 0,
    authoredProp: 7,
    replaces: 7,
  };
  const inside = look(0, ROW);
  const drawnKnowing = (known: KnownProp[], asked = inside) => {
    const standing = knownStanding([tower], known, new Set([7]));
    return fly([asked], standing).frames[0].drawn;
  };
  // The tower has fallen, unseen: the side still draws it, and the camera
  // still keeps out of it.
  expect(drawnKnowing([])).not.toBe(inside);
  // Seen: its remains are what is drawn and what blocks. The asked pose is
  // over them, and a low one inside them is lifted out.
  expect(drawnKnowing([ruin])).toBe(inside);
  const low = look(0, 25 * Math.cos(0.15) - 40, 25, 0.15);
  expect(drawnKnowing([ruin], low).pitch).toBeGreaterThan(low.pitch);
  // Seen destroyed with nothing left: nothing blocks.
  expect(drawnKnowing([{ ...ruin, destroyed: true }], low)).toBe(low);
  // What the side learns of another prop changes nothing here.
  expect(drawnKnowing([{ ...ruin, authoredProp: 8, replaces: 8 }])).not.toBe(inside);
});

test("a tuning that cannot bound the search is refused", () => {
  expect(validateClearanceTuning(TUNING)).toBe(TUNING);
  for (const bad of [
    { lift_steps: 0 },
    { slide_steps: 2.5 },
    { sweep_max_steps: 1000 },
    { refine_steps: -1 },
    { smoothing_s: 0 },
    { lookahead_s: -1 },
    { margin_m: -1 },
    { release_m: 0 },
  ])
    expect(() => validateClearanceTuning({ ...TUNING, ...bad })).toThrow(/camera clearance/);
});
