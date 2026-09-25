// @vitest-environment node
import { expect, test } from "vitest";
import { TickInterpolator } from "../src/battle/present/interpolate";
import type { ObservationView, OwnUnitView } from "../src/battle/sim/observation";

const unit = (id: number, x: number, yaw = 0): OwnUnitView => ({
  id,
  kind: "tank",
  position: [x, 0, 0],
  yaw,
  goal: null,
  policy: null,
  state: "idle",
  blocker: null,
  route: [],
  queue: [],
  members: [],
});
const frame = (tick: number, own: OwnUnitView[]): ObservationView => ({ tick, own });

test("poses blend from the previous tick toward the latest and never overshoot", () => {
  const i = new TickInterpolator(100);
  i.push(frame(1, [unit(0, 0)]), 0);
  i.push(frame(2, [unit(0, 10)]), 1000);
  expect(i.sample(1000)[0].position[0]).toBe(0);
  expect(i.sample(1050)[0].position[0]).toBe(5);
  expect(i.sample(5000)[0].position[0]).toBe(10);
});

test("yaw takes the short way round and new units appear without blending", () => {
  const i = new TickInterpolator(100);
  i.push(frame(1, [unit(0, 0, Math.PI - 0.1)]), 0);
  i.push(frame(2, [unit(0, 0, -Math.PI + 0.1), unit(1, 7)]), 0);
  const [a, b] = i.sample(50);
  expect(Math.abs(Math.abs(a.yaw) - Math.PI)).toBeLessThan(1e-9);
  expect(b.position[0]).toBe(7);
});
