// @vitest-environment node
import { expect, test } from "vitest";
import type { OwnUnitView } from "@web/battle/sim/observationView";
import {
  GHOST_SELECTED,
  ghostBodies,
  orderedGhost,
  type UnitGhost,
} from "@apps/battle-lab/src/unitGhosts";

const LOOK = GHOST_SELECTED;
const draw = (ghosts: UnitGhost[]) =>
  ghostBodies(
    ghosts,
    (k) => k === "jeep" || k === "heli",
    () => 3,
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
