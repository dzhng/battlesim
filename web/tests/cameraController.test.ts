// @vitest-environment node
import { expect, test } from "vitest";
import { gameCamera } from "@apps/battle-lab/src/gameCamera";
import { mat4, type Vec3 } from "math";
import {
  createProjectedPoint,
  projectPoint,
  viewProjMatrix,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d.ts";
import {
  CAMERA_KEYS,
  CameraController,
  PITCH_LIMITS,
  type CameraPresentation,
} from "@packages/renderer-core/src/cameraController.ts";
import { CommandBindings } from "../src/battle/input/commandBindings";

// Fixed numbers, so behaviour tests don't move when the fixture is tuned.
const CONFIG: CameraPresentation = {
  fov_y: 0.8,
  near_m: 1,
  zoom_min: 20,
  zoom_max: 2000,
  pitch_curve: [
    [20, 0.2],
    [100, 0.8],
    [2000, 1.0],
  ],
  pan_speed: 0.5,
  rotate_speed: 1.2,
  zoom_speed: 0.001,
  orbit_speed: 2.5,
  clearance: gameCamera.config.clearance,
};

const CAM: Camera3DParams = {
  target: [0, 0, 0],
  distance: 100,
  pitch: 0.8,
  yaw: 1.1,
  fovY: 0.8,
  aspect: 1.6,
  near: 1,
};

const held = (...codes: string[]) => ({ held: new Set(codes) });
const ndc = (c: Camera3DParams, p: Vec3) =>
  projectPoint(createProjectedPoint(), viewProjMatrix(mat4.create(), c), p).ndc;

test("the fixture's pitch curve rises with zoom and stays within the pitch limits", () => {
  const rig = new CameraController(gameCamera.config);
  const { zoom_min, zoom_max } = gameCamera.config;
  let last = -Infinity;
  for (let k = 0; k <= 200; k++) {
    const d = zoom_min * (zoom_max / zoom_min) ** (k / 200);
    const pitch = rig.pitchAt(d);
    expect(pitch).toBeGreaterThanOrEqual(last);
    expect(pitch).toBeGreaterThanOrEqual(PITCH_LIMITS[0]);
    expect(pitch).toBeLessThanOrEqual(PITCH_LIMITS[1]);
    last = pitch;
  }
});

test("a pitch curve that falls, leaves the limits or misses the zoom range is refused", () => {
  const curve = (pitch_curve: [number, number][]) => () =>
    new CameraController({ ...CONFIG, pitch_curve });
  expect(
    curve([
      [20, 0.9],
      [2000, 0.5],
    ]),
  ).toThrow(/monotonic/);
  expect(
    curve([
      [20, 0.01],
      [2000, 0.5],
    ]),
  ).toThrow(/limits/);
  expect(
    curve([
      [50, 0.3],
      [2000, 0.5],
    ]),
  ).toThrow(/zoom range/);
});

test("held keys combine: W+D pans diagonally, W+S cancels", () => {
  const rig = new CameraController(CONFIG);
  const diag = rig.step(CAM, held("KeyW", "KeyD"), 0.2);
  // The old centre slides down and left: the view moved forward and right.
  const [x, y] = ndc(diag, [0, 0, 0]);
  expect(x).toBeLessThan(-0.01);
  expect(y).toBeLessThan(-0.01);
  // Diagonal is no faster than straight.
  const straight = rig.step(CAM, held("KeyW"), 0.2);
  const moved = (c: Camera3DParams) => Math.hypot(c.target[0], c.target[1]);
  expect(moved(diag)).toBeCloseTo(moved(straight), 6);
  expect(rig.step(CAM, held("KeyW", "KeyS"), 0.2)).toBe(CAM);
});

test("arrows pan like WASD, at pan_speed camera distances per second", () => {
  const rig = new CameraController(CONFIG);
  const d = rig.step(CAM, held("KeyD"), 0.5);
  expect(rig.step(CAM, held("ArrowRight"), 0.5)).toEqual(d);
  expect(Math.hypot(d.target[0], d.target[1])).toBeCloseTo(0.5 * 100 * 0.5, 6);
  expect(d.distance).toBe(CAM.distance);
  expect(d.pitch).toBe(CAM.pitch);
});

test("Q turns the view left and E turns it right, about the target", () => {
  const rig = new CameraController(CONFIG);
  // A point straight ahead of the target, on screen above the centre.
  const ahead: [number, number, number] = [-Math.cos(CAM.yaw) * 30, -Math.sin(CAM.yaw) * 30, 0];
  const q = rig.step(CAM, held("KeyQ"), 0.25);
  const e = rig.step(CAM, held("KeyE"), 0.25);
  expect(ndc(q, ahead)[0]).toBeGreaterThan(0.01);
  expect(ndc(e, ahead)[0]).toBeLessThan(-0.01);
  expect(q.yaw - CAM.yaw).toBeCloseTo(1.2 * 0.25, 6);
  expect(q.target).toEqual(CAM.target);
});

test("the wheel zooms within the limits and moves pitch along the curve", () => {
  const rig = new CameraController(CONFIG);
  const out = rig.step(CAM, { wheel: 1e6 }, 0);
  expect(out.distance).toBe(2000);
  expect(out.pitch).toBeCloseTo(1.0, 6);
  const inn = rig.step(CAM, { wheel: -1e6 }, 0);
  expect(inn.distance).toBe(20);
  expect(inn.pitch).toBeCloseTo(0.2, 6);
  // A tilt the player chose by dragging is kept as an offset from the curve.
  const tilted = { ...CAM, pitch: 0.6 };
  expect(rig.step(tilted, { wheel: 1e6 }, 0).pitch).toBeCloseTo(0.8, 6);
});

test("middle-drag orbits yaw and pitch, never zoom, and pitch stays in its limits", () => {
  const rig = new CameraController(CONFIG);
  const turned = rig.step(CAM, { drag: [0.1, 0] }, 0);
  expect(turned.yaw).toBeCloseTo(CAM.yaw - 0.25, 6);
  expect(turned.distance).toBe(CAM.distance);
  expect(rig.step(CAM, { drag: [0, 10] }, 0).pitch).toBe(PITCH_LIMITS[1]);
  expect(rig.step(CAM, { drag: [0, -10] }, 0).pitch).toBe(PITCH_LIMITS[0]);
});

test("an idle intent returns the camera untouched, so nothing redraws", () => {
  const rig = new CameraController(CONFIG);
  expect(rig.step(CAM, held(), 0.1)).toBe(CAM);
  expect(rig.step(CAM, held("KeyZ"), 0.1)).toBe(CAM);
  expect(rig.step(CAM, { wheel: 0, drag: [0, 0], edge: [0, 0] }, 0.1)).toBe(CAM);
});

test("the target rides the ground as the camera moves", () => {
  const rig = new CameraController(CONFIG, (x, y) => 0.1 * x + 0.05 * y);
  const moved = rig.step(CAM, held("KeyD"), 0.5);
  expect(moved.target[2]).toBeCloseTo(0.1 * moved.target[0] + 0.05 * moved.target[1], 9);
});

test("a scripted framing is placed within the rig's limits, on the ground", () => {
  const rig = new CameraController(CONFIG, (x, y) => x + y);
  const inside = rig.place(CAM, { target: [10, 20], distance: 300, yaw: -7.5, pitch: 0.6 });
  // Angles stay unwrapped: a tour turning past ±π keeps turning.
  expect(inside).toEqual({ ...CAM, target: [10, 20, 30], distance: 300, yaw: -7.5, pitch: 0.6 });
  const outside = rig.place(CAM, { target: [0, 0], distance: 5000, yaw: 0, pitch: 3 });
  expect(outside.distance).toBe(CONFIG.zoom_max);
  expect(outside.pitch).toBe(PITCH_LIMITS[1]);
});

test("the screen edge pans like the keys", () => {
  const rig = new CameraController(CONFIG);
  expect(rig.step(CAM, { edge: [1, 0] }, 0.5)).toEqual(rig.step(CAM, held("KeyD"), 0.5));
});

test("no command key is a camera key", () => {
  const camera = new Set(Object.keys(CAMERA_KEYS));
  for (const { code } of Object.values(CommandBindings)) expect(camera.has(code)).toBe(false);
});

test("either Shift key triples keyboard pan without speeding rotation", () => {
  const rig = new CameraController(CONFIG);
  const normal = rig.step(CAM, held("KeyW", "KeyD", "KeyQ"), 0.2);
  for (const shift of ["ShiftLeft", "ShiftRight"]) {
    const fast = rig.step(CAM, held("KeyW", "KeyD", "KeyQ", shift), 0.2);
    expect(Math.hypot(...fast.target)).toBeCloseTo(3 * Math.hypot(...normal.target), 6);
    expect(fast.yaw).toBe(normal.yaw);
  }
});
