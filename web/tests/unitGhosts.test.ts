// @vitest-environment node
import { expect, test } from "vitest";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { OwnUnitView } from "@web/battle/sim/observation";
import { orderedGhost, pushGhostModels, type UnitGhost } from "@apps/battle-lab/src/unitGhosts";

const COLOUR = [0.2, 0.6, 1, 0.5] as const;
/** An aircraft's cruise height over the ground. */
const CRUISE = 20;
const appearances = {
  appearances: new Map([
    ["rifleman", { bundle: { kind: "static", states: [{ name: "intact" }] } }],
    ["gunner", { bundle: { kind: "static", states: [{ name: "intact" }] } }],
    ["jeep", { bundle: { kind: "static", states: [{ name: "intact" }] } }],
    ["heli", { bundle: { kind: "static", states: [{ name: "intact" }] } }],
  ]),
} as unknown as InstalledAppearances;
const resolve = (kind: string, _side: string, _id: number, slot: number) => ({
  appearance: kind === "jeep" || kind === "heli" ? kind : slot === 0 ? "rifleman" : "gunner",
  tint: [1, 1, 1] as const,
});
const draw = (ghosts: UnitGhost[]) => {
  const out: ModelInstance[] = [];
  pushGhostModels(
    out,
    ghosts,
    "blue",
    resolve as never,
    appearances,
    () => 3,
    (k) => k === "jeep" || k === "heli",
    (k) => (k === "heli" ? CRUISE : 0),
  );
  return out;
};

test("a squad's ghost is each of its soldiers in his own look, a vehicle's its one hull", () => {
  const models = draw([
    {
      kind: "squad",
      at: [0, 0],
      soldiers: [
        { at: [10, 20], slot: 0, id: 7 },
        { at: [12, 21], slot: 1, id: 8 },
      ],
      yaw: 1,
      colour: COLOUR,
    },
    { kind: "jeep", at: [50, 60], soldiers: [], yaw: 2, colour: COLOUR },
  ]);
  expect(models.map((m) => [m.appearance, m.x, m.y, m.z, m.yaw, m.ghost])).toEqual([
    ["rifleman", 10, 20, 3, 1, COLOUR],
    ["gunner", 12, 21, 3, 1, COLOUR],
    ["jeep", 50, 60, 3, 2, COLOUR],
  ]);
});

test("an aircraft's ghost flies at its cruise height over the ground, not landed", () => {
  const models = draw([
    { kind: "heli", at: [50, 60], soldiers: [], yaw: 2, colour: COLOUR },
    { kind: "jeep", at: [70, 60], soldiers: [], yaw: 2, colour: COLOUR },
  ]);
  expect(models.map((m) => [m.appearance, m.z])).toEqual([
    ["heli", 3 + CRUISE],
    ["jeep", 3],
  ]);
});

test("a squad with no soldiers placed yet draws nothing rather than one stand-in", () => {
  expect(draw([{ kind: "squad", at: [0, 0], soldiers: [], yaw: 0, colour: COLOUR }])).toEqual([]);
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
  expect(orderedGhost(squad({}), COLOUR)).toEqual({
    kind: "squad",
    at: [100, 100],
    soldiers: [
      { at: [99, 100], slot: 0, id: 7 },
      { at: [101, 102], slot: 1, id: 8 },
    ],
    yaw: 0.5,
    colour: COLOUR,
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
      COLOUR,
    )?.soldiers,
  ).toEqual([
    { at: [199, 120], slot: 0, id: 7 },
    { at: [201, 122], slot: 1, id: 8 },
  ]);
});

test("a unit holding where it stands has no ghost", () => {
  expect(orderedGhost(squad({ goal: null }), COLOUR)).toBeNull();
});
