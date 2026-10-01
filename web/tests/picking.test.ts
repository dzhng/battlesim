// @vitest-environment node
// Picking by the simulation's boxes: a vehicle is picked by its hull box and a
// soldier by his body's, whatever the drawn model's overhang (the tank's gun,
// its antenna), so a click selects what the rules say is there.
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import type { Vec3 } from "math";
import { expect, test } from "vitest";
import game from "@fixtures/game.json";
import {
  bodyBox,
  pickBox,
  rayBoxDistance,
  type PickBox,
} from "@packages/battle-renderer/src/picking.ts";

const bodies = game.physics;
const at = (kind: string, x: number, yaw = 0): PickBox => ({
  x,
  y: 0,
  z: 0,
  yaw,
  ...bodyBox(bodies, UNITS.hull(kind)),
});
const down = (x: number, y: number) => ({ origin: [x, y, 50] as Vec3, dir: [0, 0, -1] as Vec3 });

test("a vehicle is picked by its hull box: its roof, not beside its narrow side", () => {
  // Yawed 90°, the tank's long axis runs along world Y: y=3 is on the hull, x=12.5 is beside it.
  const tank = at("tank", 10, Math.PI / 2);
  const [, , hz] = UNITS.hull("tank")!.half_extents_m;
  expect(rayBoxDistance(down(10, 3), tank)).toBeCloseTo(50 - 2 * hz, 5);
  expect(rayBoxDistance(down(12.5, 0), tank)).toBe(Infinity);
  // The gun reaches 5.9 m ahead of the hull centre, past the box: not the tank.
  expect(rayBoxDistance(down(10, 5.5), tank)).toBe(Infinity);
  const truck = at("supply", 0);
  expect(rayBoxDistance(down(0, 0), truck)).toBeCloseTo(
    50 - 2 * UNITS.hull("supply")!.half_extents_m[2],
    5,
  );
});

test("a soldier is picked by his body, head to foot", () => {
  const soldier = at("rifle", 0);
  expect(rayBoxDistance(down(0, 0), soldier)).toBeCloseTo(50 - bodies.soldier_height_m, 5);
  expect(rayBoxDistance(down(bodies.soldier_radius_m + 0.05, 0), soldier)).toBe(Infinity);
});

test("the nearest hit along the ray wins", () => {
  const far = at("tank", 10);
  const near = at("tank", 0);
  const ray = { origin: [-50, 0, 1] as Vec3, dir: [1, 0, 0] as Vec3 };
  expect(pickBox(ray, [far, near])).toBe(1);
  expect(pickBox({ origin: [0, 0, 50], dir: [0, 0, 1] }, [far])).toBe(-1);
});
