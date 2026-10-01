// @vitest-environment node
import { expect, test } from "vitest";
import {
  createCameraObstacles,
  type ObstacleBox,
} from "@packages/renderer-core/src/cameraObstacles.ts";

const FLAT = () => 0;
/** A 20 × 10 m house, 8 m tall, turned a quarter turn: 10 m wide in x, 20 in y. */
const HOUSE: ObstacleBox = { center: [100, 50], yaw: Math.PI / 2, half: [10, 5, 4], baseZ: 0 };

test("a point is clear of a turned box by its clearance on every side and above", () => {
  const view = createCameraObstacles([HOUSE], FLAT);
  // Beside its narrow (x) side: the wall is at x = 105.
  expect(view.clear([107.1, 50, 3], 2)).toBe(true);
  expect(view.clear([106.9, 50, 3], 2)).toBe(false);
  // Beside its long (y) side: the wall is at y = 60. Unturned it would end at 55.
  expect(view.clear([100, 58, 3], 1)).toBe(false);
  expect(view.clear([100, 61.1, 3], 1)).toBe(true);
  // Over the roof at 8 m.
  expect(view.clear([100, 50, 9.9], 2)).toBe(false);
  expect(view.clear([100, 50, 10.1], 2)).toBe(true);
  expect(view.ceiling).toBe(8);
});

test("a point within its clearance of the ground is not clear", () => {
  const hill = createCameraObstacles([], (x) => x * 0.5);
  expect(hill.clear([10, 0, 7.1], 2)).toBe(true);
  expect(hill.clear([10, 0, 6.9], 2)).toBe(false);
  expect(hill.ceiling).toBe(-Infinity);
});

test("a move is swept: clear ends either side of a box do not make a clear move", () => {
  const view = createCameraObstacles([HOUSE], FLAT);
  const [west, east]: [number, number, number][] = [
    [90, 50, 3],
    [110, 50, 3],
  ];
  expect(view.clear(west, 1) && view.clear(east, 1)).toBe(true);
  expect(view.sweepClear(west, east, 1)).toBe(false);
  // Over the roof, and along the wall outside it, the same move is clear.
  expect(view.sweepClear([90, 50, 9.5], [110, 50, 9.5], 1)).toBe(true);
  expect(view.sweepClear([90, 62, 3], [110, 62, 3], 1)).toBe(true);
  // A move that starts inside is not clear, nor one too short to have a direction.
  expect(view.sweepClear([100, 50, 3], [120, 50, 3], 1)).toBe(false);
  expect(view.sweepClear([100, 50, 3], [100, 50, 3], 1)).toBe(false);
});

test("a blocked move says how tall the tallest thing in its way is", () => {
  const tower: ObstacleBox = { center: [130, 50], yaw: 0, half: [5, 5, 20], baseZ: 2 };
  const view = createCameraObstacles([HOUSE, tower], FLAT);
  expect(view.sweepTop([90, 50, 3], [110, 50, 3], 1)).toBe(8);
  expect(view.sweepTop([90, 50, 3], [150, 50, 3], 1)).toBe(42);
  expect(view.sweepTop([150, 50, 3], [90, 50, 3], 1)).toBe(42);
  expect(view.sweepTop([90, 62, 3], [110, 62, 3], 1)).toBe(-Infinity);
  expect(view.sweepTop([90, 50, 50], [150, 50, 50], 1)).toBe(-Infinity);
});

test("a point in a box is pushed out through the side or top nearest where it is headed from", () => {
  const view = createCameraObstacles([HOUSE], FLAT);
  const out: [number, number, number] = [0, 0, 0];
  // The house, turned, spans x 95..105 and y 40..60; a metre's clearance.
  const inside: [number, number, number] = [103, 50, 3];
  // The shortest way out: its east wall.
  expect(view.pushOut(out, inside, inside, 1)).toBe(out);
  expect(out[0]).toBeCloseTo(106, 4);
  expect([out[1], out[2]]).toEqual([50, 3]);
  expect(view.clear(out, 1)).toBe(true);
  // Coming from the west, it stays on the west wall; from above, on the roof.
  view.pushOut(out, inside, [90, 50, 3], 1);
  expect(out[0]).toBeCloseTo(94, 4);
  view.pushOut(out, inside, [103, 50, 12], 1);
  expect(out).toEqual([103, 50, expect.closeTo(9, 4)]);
  // Never out through the floor.
  view.pushOut(out, [100, 50, 0.5], [100, 50, 0.5], 1);
  expect(view.clear(out, 1)).toBe(true);
  // A point already clear stays where it is.
  view.pushOut(out, [80, 50, 3], [80, 50, 3], 1);
  expect(out).toEqual([80, 50, 3]);
});

test("a point pushed out of one box into another ends clear of both, over them if wedged", () => {
  // An L of two walls meeting at (0, 0), and its inside corner.
  const north: ObstacleBox = { center: [-20, 5], yaw: 0, half: [25, 5, 6], baseZ: 0 };
  const east: ObstacleBox = { center: [0, -20], yaw: 0, half: [5, 20, 10], baseZ: 0 };
  const corner = createCameraObstacles([north, east], FLAT);
  const out: [number, number, number] = [0, 0, 0];
  corner.pushOut(out, [-5.5, -0.5, 3], [-9, -4, 3], 1);
  expect(out[0]).toBeCloseTo(-6, 4);
  expect(out[1]).toBeCloseTo(-1, 4);
  expect(corner.clear(out, 1)).toBe(true);
  // An alley a metre wide between two houses: no room at a metre's clearance.
  const alley = createCameraObstacles(
    [
      { center: [-5.5, 0], yaw: 0, half: [5, 10, 4], baseZ: 0 },
      { center: [5.5, 0], yaw: 0, half: [5, 10, 6], baseZ: 0 },
      { center: [200, 0], yaw: 0, half: [5, 5, 40], baseZ: 0 },
    ],
    FLAT,
  );
  alley.pushOut(out, [0, 0, 3], [0, 0, 3], 1);
  expect(alley.clear(out, 1)).toBe(true);
  // Over the taller of the two, not over the distant tower.
  expect(out[2]).toBeCloseTo(13, 4);
  // And off the ground: a point under a hillside comes up onto it.
  const hill = createCameraObstacles([], (x) => x * 0.5);
  hill.pushOut(out, [10, 0, 2], [10, 0, 2], 2);
  expect(out).toEqual([10, 0, 7]);
});

test("a query reads the boxes near it, not the whole town", () => {
  // 10,000 houses on a 30 m pitch: a 3 km town.
  const town: ObstacleBox[] = [];
  for (let i = 0; i < 100; i++)
    for (let j = 0; j < 100; j++)
      town.push({ center: [i * 30, j * 30], yaw: 0.3, half: [8, 6, 4], baseZ: 0 });
  const view = createCameraObstacles(town, FLAT);
  // Every answer agrees with asking each box alone.
  const alone = town.map((box) => createCameraObstacles([box], FLAT));
  for (const p of [
    [1500, 1500, 3],
    [1492, 1506.5, 5],
    [15, 14, 9],
    [2985, 2970, 2],
    [-40, 1200, 3],
  ] as [number, number, number][]) {
    const before = view.tested;
    expect(view.clear(p, 2)).toBe(alone.every((one) => one.clear(p, 2)));
    expect(view.tested - before).toBeLessThan(20);
  }
  const before = view.tested;
  expect(view.sweepClear([1500, 1515, 3], [1560, 1515, 3], 1)).toBe(true);
  expect(view.sweepClear([1500, 1515, 3], [1560, 1530, 3], 1)).toBe(false);
  expect(view.tested - before).toBeLessThan(60);
});
