// @vitest-environment node
import { expect, test } from "vitest";
import { TickInterpolator } from "../src/battle/present/interpolate";
import type { ObservationView, OwnUnitView } from "../src/battle/sim/observation";

const unit = (id: number, x: number, yaw = 0): OwnUnitView => ({
  id,
  kind: "test_tank",
  position: [x, 0, 0],
  yaw,
  goal: null,
  policy: null,
  direction: null,
  reversing: false,
  withdrawing: false,
  protection: null,
  state: "idle",
  blocker: null,
  route: [],
  queue: [],
  members: [],
  memberIds: [],
  memberSlots: [],
  memberActiveMounts: [],
  memberOrders: [],
  memberLeans: [],
  area: null,
  finalFacing: yaw,
  sees: [],
  engagement: "fire_at_will",
  mounts: [],
  weaponPoses: [],
  hp: 100,
  memberHp: [],
  suppression: "none",
  concealed: false,
  deployment: null,
  garrison: null,
  stock: null,
  service: "out_of_range",
  sight: { eyes: [[x, 0, 2]], forward: yaw, shape: { front: 1, side: 0.5, rear: 0.3 }, range: 350 },
});
const frame = (tick: number, own: OwnUnitView[]): ObservationView => ({
  tick,
  own,
  identified: [],
  contacts: [],
  audible: [],
  projectiles: [],
  blasts: [],
  corpses: [],
  fallenBodies: [],
  guided: [],
  encounter: null,
  skirmish: null,
  knownProps: [],
  fog: { cellM: 8, nx: 0, ny: 0, bits: new Uint32Array(0) },
  groundPatch: {
    epoch: 1,
    side: "blue",
    baseRevision: 0,
    revision: 0,
    full: true,
    runs: new Float32Array(0),
  },
});

test("poses blend from the previous tick toward the latest and never overshoot", () => {
  const i = new TickInterpolator(100);
  i.push(frame(1, [unit(0, 0)]), 0);
  i.push(frame(2, [unit(0, 10)]), 1000);
  expect(i.frame(1000)!.own[0].position[0]).toBe(0);
  expect(i.frame(1050)!.own[0].position[0]).toBe(5);
  expect(i.frame(5000)!.own[0].position[0]).toBe(10);
});

test("yaw takes the short way round and new units appear without blending", () => {
  const i = new TickInterpolator(100);
  i.push(frame(1, [unit(0, 0, Math.PI - 0.1)]), 0);
  i.push(frame(2, [unit(0, 0, -Math.PI + 0.1), unit(1, 7)]), 0);
  const [a, b] = i.frame(50)!.own;
  expect(Math.abs(Math.abs(a.yaw) - Math.PI)).toBeLessThan(1e-9);
  expect(b.position[0]).toBe(7);
});

test("deployment progress blends between ticks and stays null for units without it", () => {
  const i = new TickInterpolator(100);
  const supply = (progress: number): OwnUnitView => ({
    ...unit(1, 0),
    kind: "test_supply",
    deployment: { progress, target: "deployed" },
  });
  i.push(frame(1, [supply(0.2), unit(0, 0)]), 0);
  i.push(frame(2, [supply(0.4), unit(0, 0)]), 1000);
  expect(i.frame(1050)!.own[0].deployment).toBeCloseTo(0.3);
  expect(i.frame(5000)!.own[0].deployment).toBeCloseTo(0.4);
  expect(i.frame(1050)!.own[1].deployment).toBeNull();
});

test("a drawn frame reads the publication its poses blend toward, mid-blend included", () => {
  const i = new TickInterpolator(100);
  expect(i.frame(0)).toBeNull();
  i.push(frame(1, [unit(0, 0), unit(1, 5)]), 0);
  // Unit 1 is gone in tick 2. Halfway between the ticks the frame draws
  // tick 2's units, and tick 2 is what it reads of the battle.
  const latest = frame(2, [unit(0, 10)]);
  i.push(latest, 1000);
  const mid = i.frame(1050)!;
  expect(mid.observation).toBe(latest);
  expect(mid.own.map((p) => p.id)).toEqual([0]);
  expect(mid.own[0].position[0]).toBe(5);
  expect(mid.time).toBeCloseTo(0.15);
});
