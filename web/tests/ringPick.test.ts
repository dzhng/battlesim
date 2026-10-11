// @vitest-environment node
// A click on the ground inside a unit's marker circle picks that unit, as a
// click on its body does: every kind of unit, the circle where this frame
// draws it.
import { expect, test } from "vitest";
import type { ObservationView, OwnUnitView, IdentifiedView } from "@web/battle/sim/observation";
import type { Pose } from "@web/battle/present/interpolate";
import { ringUnder, sideInstances } from "@apps/battle-lab/src/sideInstances";
import { gameOrderStyle as STYLE } from "@apps/battle-lab/src/gameOverlay";
import { UNITS } from "./catalog";
import game from "../../fixtures/game.json";

const own = (u: Partial<OwnUnitView>) =>
  ({
    id: 1,
    kind: "test_heli",
    position: [100, 50, 20],
    yaw: 0,
    goal: null,
    state: "idle",
    route: [],
    queue: [],
    members: [],
    memberOrders: [],
    area: null,
    garrison: null,
    finalFacing: 0,
    direction: null,
    ...u,
  }) as OwnUnitView;
const seen = (u: Partial<IdentifiedView>) =>
  ({
    id: 9,
    kind: "test_heli",
    position: [300, 50, 20],
    yaw: 0,
    members: [],
    ...u,
  }) as IdentifiedView;
const pose = (u: { id: number; position: readonly number[] }): Pose => ({
  id: u.id,
  position: [u.position[0], u.position[1], u.position[2]],
  yaw: 0,
  members: [],
  memberIds: [],
  deployment: null,
});
const drawnFor = (mine: OwnUnitView[], theirs: IdentifiedView[] = []) =>
  sideInstances(
    mine.map(pose),
    theirs.map(pose),
    { own: mine, identified: theirs } as unknown as ObservationView,
    game.physics,
    UNITS,
  );
/** A vehicle's marker radius: its hull's half length plus the margin. */
const ringOf = (kind: string) =>
  UNITS.hull(kind)!.half_extents_m[0] + STYLE.vehicle_marker_margin_m;

test("a click in a helicopter's ring, on the ground under it, selects it", () => {
  const drawn = drawnFor([own({})]);
  const r = ringOf("test_heli");
  // Well clear of its airframe's box, which hangs 20 m up.
  expect(ringUnder(drawn.rings, [100 + r * 0.9, 50], true)).toEqual({ unit: 1, enemy: null });
  expect(ringUnder(drawn.rings, [100 + r * 1.1, 50], true)).toBeNull();
});

test("every kind of unit's ring picks it: a tank's, not only a squad's", () => {
  const drawn = drawnFor([own({ id: 2, kind: "test_tank", position: [40, 40, 0] })]);
  expect(ringUnder(drawn.rings, [40 + ringOf("test_tank") * 0.95, 40], true)).toEqual({
    unit: 2,
    enemy: null,
  });
});

test("a right click in an own unit's ring still moves: only a left click picks it", () => {
  const drawn = drawnFor([own({})]);
  expect(ringUnder(drawn.rings, [100, 50], false)).toBeNull();
});

test("an enemy helicopter's ring picks it for either button", () => {
  const drawn = drawnFor([], [seen({})]);
  for (const left of [true, false])
    expect(ringUnder(drawn.rings, [300 + ringOf("test_heli") * 0.5, 50], left)).toEqual({
      unit: null,
      enemy: 9,
    });
});

test("where rings overlap, the nearest centre wins", () => {
  const drawn = drawnFor([own({ id: 1 }), own({ id: 2, position: [104, 50, 20] })]);
  expect(ringUnder(drawn.rings, [101, 50], true)?.unit).toBe(1);
  expect(ringUnder(drawn.rings, [103, 50], true)?.unit).toBe(2);
});
