import { shippedRules } from "@web/shippedRules";
import { loadWasm } from "@web/battle/sim/module";
beforeAll(() => loadWasm());
// @vitest-environment node
import { beforeAll, expect, test } from "vitest";
import type { OwnUnitView } from "@web/battle/sim/observationView";
import {
  GHOST_SELECTED,
  ghostBodies,
  orderedGhost,
  aircraftOrderReadouts,
  ghostAloft,
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
    queue: [],
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
  expect(
    orderedGhost(
      squad({
        queue: [
          [150, 100],
          [200, 120],
        ],
      }),
      LOOK,
    )?.soldiers,
  ).toEqual([
    { at: [199, 120], slot: 0, id: 7 },
    { at: [201, 122], slot: 1, id: 8 },
  ]);
});

test("a unit holding where it stands has no ghost", () => {
  expect(orderedGhost(squad({ goal: null }), LOOK)).toBeNull();
});

test("Space aircraft cards follow the final order at flight altitude with the actual unit info", () => {
  const jet = squad({ id: 7, kind: "jet", queue: [[250, 180]], fuelS: 95 });
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
