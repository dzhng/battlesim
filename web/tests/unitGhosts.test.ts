import { shippedRules } from "@web/shippedRules";
import { loadWasm } from "@web/battle/sim/module";
beforeAll(() => loadWasm());
// @vitest-environment node
import { beforeAll, expect, test } from "vitest";
import type { OrderStepView, OwnUnitView } from "@web/battle/sim/observationView";
import {
  GHOST_SELECTED,
  ghostBodies,
  orderedGhost,
  aircraftOrderReadouts,
  ghostAloft,
  pointAloftOnRay,
  type UnitGhost,
} from "@apps/battle-lab/src/unitGhosts";

const LOOK = GHOST_SELECTED;
const draw = (ghosts: UnitGhost[]) =>
  ghostBodies(
    ghosts,
    (k) => k === "jeep" || k === "heli",
    () => 3,
    () => 0,
  );

test("a squad's ghost is each of its soldiers where he stands, a vehicle's its one hull", () => {
  const bodies = draw([
    {
      kind: "squad",
      at: [0, 0],
      soldiers: [
        { at: [10, 20], slot: 0, id: 7 },
        { at: [12, 21], slot: 1, id: 8 },
      ],
      yaw: 1,
      look: LOOK,
    },
    { kind: "jeep", at: [50, 60], soldiers: [], yaw: 2, look: LOOK },
  ]);
  expect(bodies.map((b) => [b.kind, b.slot, b.id, b.x, b.y, b.z, b.yaw, b.look])).toEqual([
    ["squad", 0, 7, 10, 20, 3, 1, LOOK],
    ["squad", 1, 8, 12, 21, 3, 1, LOOK],
    ["jeep", 0, 0, 50, 60, 3, 2, LOOK],
  ]);
});

test("a squad with no soldiers placed yet draws nothing rather than one stand-in", () => {
  expect(draw([{ kind: "squad", at: [0, 0], soldiers: [], yaw: 0, look: LOOK }])).toEqual([]);
});

const squad = (over: Partial<OwnUnitView>) =>
  ({
    kind: "squad",
    goal: [100, 100],
    steps: [{ kind: "move", point: [100, 100] }],
    finalFacing: 0.5,
    memberSlots: [0, 1],
    memberIds: [7, 8],
    memberOrders: [
      { spot: [99, 100], coverNow: null, coverThere: null },
      { spot: [101, 102], coverNow: null, coverThere: null },
    ],
    ...over,
  }) as OwnUnitView;

test("with Space held a moving squad's soldiers stand where its orders end, facing their end", () => {
  expect(orderedGhost(squad({}), LOOK)).toEqual({
    kind: "squad",
    at: [100, 100],
    soldiers: [
      { at: [99, 100], slot: 0, id: 7 },
      { at: [101, 102], slot: 1, id: 8 },
    ],
    yaw: 0.5,
    look: LOOK,
  });
  // Queued legs: the squad's shape is carried to the last queued point.
  const legs = (...steps: OrderStepView[]) =>
    orderedGhost(squad({ steps: [{ kind: "move", point: [100, 100] }, ...steps] }), LOOK);
  expect(
    legs({ kind: "move", point: [150, 100] }, { kind: "move", point: [200, 120] })?.soldiers,
  ).toEqual([
    { at: [199, 120], slot: 0, id: 7 },
    { at: [201, 122], slot: 1, id: 8 },
  ]);
  // An attack only aims: a queued one never moves where the orders end.
  const fire: OrderStepView = {
    kind: "attack",
    point: [300, 300],
    target: { kind: "identified", id: 4 },
  };
  expect(legs({ kind: "move", point: [150, 100] }, fire)?.at).toEqual([150, 100]);
  expect(legs(fire)?.at).toEqual([100, 100]);
  // Any other step that takes it somewhere does: here, into a building.
  const enter: OrderStepView = {
    kind: "garrison",
    point: [180, 90],
    host: { kind: "building", id: 2 },
  };
  expect(legs(fire, enter)?.at).toEqual([180, 90]);
});

test("a unit holding where it stands has no ghost", () => {
  expect(orderedGhost(squad({ goal: null }), LOOK)).toBeNull();
});

test("a unit firing where it stands with a move queued behind has its ghost at the move", () => {
  const fire: OrderStepView = { kind: "attack", point: [140, 60], target: null };
  const onward: OrderStepView = { kind: "move", point: [160, 80], attackMove: false };
  const unit = squad({ goal: null, position: [150, 70, 0], steps: [fire, onward] });
  expect(orderedGhost(unit, LOOK)?.at).toEqual([160, 80]);
});

test("Space aircraft cards follow the final order at flight altitude with the actual unit info", () => {
  const jet = squad({
    id: 7,
    kind: "jet",
    steps: [
      { kind: "move", point: [100, 100] },
      { kind: "move", point: [250, 180] },
    ],
    fuelS: 95,
  });
  const heli = squad({ id: 8, kind: "heli" });
  const holding = squad({ id: 9, kind: "jet", goal: null });
  const isHighAir = (kind: string) => kind === "jet";
  const elevation = () => 64;
  const altitude = (kind: string) => (kind === "jet" ? 1000 : 20);
  expect(
    aircraftOrderReadouts(
      [jet, heli, holding],
      false,
      new Map([[7, 1]]),
      isHighAir,
      elevation,
      altitude,
    ),
  ).toEqual([]);
  const cards = aircraftOrderReadouts(
    [jet, heli, holding],
    true,
    new Map([
      [7, 1],
      [8, 1],
      [9, 1],
    ]),
    isHighAir,
    elevation,
    altitude,
  );
  expect(cards).toEqual([{ unit: jet, position: [250, 180, 1064] }]);
  expect(cards[0].unit).toBe(jet);
});

test("the ghost stands at its kind's cruise height: its own, else its layer's, never under the floor", () => {
  const rows: Record<string, { layer: "low_air" | "high_air"; cruise_agl_m?: number }> = {
    heli: { layer: "low_air" },
    jet: { layer: "high_air" },
    low_jet: { layer: "high_air", cruise_agl_m: 60 },
    high_jet: { layer: "high_air", cruise_agl_m: 400 },
    tall_heli: { layer: "low_air", cruise_agl_m: 300 },
  };
  const units = {
    type: (kind: string) => ({
      mobility: {
        air: {
          cruise_kmh: 200,
          turn_deg_s: 30,
          climb_mps: 8,
          flight: rows[kind].layer === "high_air" ? "fixed_wing" : "rotor",
          ...rows[kind],
        },
      },
    }),
  } as any;
  const air = {
    ...shippedRules.air,
    low_air: { cruise_agl_m: 30 },
    high_air: { cruise_agl_m: 250, floor_agl_m: 100 },
  };
  const altitude = ghostAloft(units, air);
  expect(altitude("heli")).toBe(30);
  expect(altitude("jet")).toBe(250);
  expect(altitude("high_jet")).toBe(400);
  expect(altitude("low_jet")).toBe(100);
  expect(altitude("tall_heli")).toBe(300);
});

test("an aircraft ghost placed aloft lies on the pointer's ray at its height over the ground", () => {
  // Looking down at 45° from 1 km up, toward +x.
  const ray = { origin: [0, 0, 1000], dir: [Math.SQRT1_2, 0, -Math.SQRT1_2] };
  const flat = pointAloftOnRay(ray, () => 0, 250);
  expect(flat![0]).toBeCloseTo(750, 6);
  expect(flat![1]).toBeCloseTo(0, 6);

  // Over a slope it still stands at its height over the ground under it,
  // on the ray: so it draws under the pointer.
  const slope = (x: number) => x * 0.1;
  const [x, y] = pointAloftOnRay(ray, slope, 250)!;
  const t = x / Math.SQRT1_2;
  expect(y).toBeCloseTo(0, 6);
  expect(1000 - t * Math.SQRT1_2).toBeCloseTo(slope(x) + 250, 1);
});

test("a ray that never climbs down to the height has no point aloft", () => {
  expect(pointAloftOnRay({ origin: [0, 0, 200], dir: [0.6, 0, -0.8] }, () => 0, 250)).toBeNull();
  expect(pointAloftOnRay({ origin: [0, 0, 900], dir: [1, 0, 0] }, () => 0, 250)).toBeNull();
});
