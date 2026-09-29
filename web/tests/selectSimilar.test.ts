// Select similar and a mixed selection's commands (the user's calls):
// a double-click selects the unit's type, a second one (or Ctrl) its role;
// a command is lit when any selected unit can carry it out, and reaches only
// those that can.
import { expect, test } from "vitest";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { reach } from "@web/battle/input/commandReach";
import type { OwnUnitView } from "@web/battle/sim/observation";
import { SelectClicks, similarUnits } from "@web/battle/input/selectSimilar";
import { syntheticUnits } from "./sceneAssets/synthetic";

const click = (unit: number | null, kind: string | null, time: number, ctrl = false) => ({
  unit,
  kind,
  ctrl,
  x: 100,
  y: 100,
  time,
});

test("clicks too slow, too far apart or on different units are not a double-click", () => {
  const clicks = new SelectClicks();
  clicks.click(click(1, "tank", 0));
  expect(clicks.click(click(1, "tank", 351))).toBeNull();
  expect(clicks.click({ ...click(1, "tank", 400), x: 110 })).toBeNull();
  expect(clicks.click(click(2, "tank", 500))).toBeNull();
  // Three quick clicks are one double-click, then a fresh first click.
  const triple = new SelectClicks();
  triple.click(click(1, "tank", 0));
  expect(triple.click(click(1, "tank", 100))).toBe("type");
  expect(triple.click(click(1, "tank", 200))).toBeNull();
  // A box selection in between starts over.
  const boxed = new SelectClicks();
  boxed.click(click(1, "tank", 0));
  boxed.click(click(1, "tank", 100));
  boxed.reset();
  boxed.click(click(1, "tank", 1000));
  expect(boxed.click(click(1, "tank", 1100))).toBe("type");
});

test("similar units are the own units of the type, or of the type's first role", () => {
  const units = syntheticUnits();
  const own = [
    { id: 1, kind: "tank" },
    { id: 2, kind: "supply" },
    { id: 3, kind: "tank" },
    { id: 4, kind: "rifle" },
  ];
  expect(similarUnits(own, "tank", "type", units)).toEqual([1, 3]);
  // Every synthetic type is in role "test".
  expect(similarUnits(own, "tank", "role", units)).toEqual([1, 2, 3, 4]);
  // The shipped roles map one to one onto today's types.
  expect(similarUnits(own, "tank", "role", UNITS)).toEqual([1, 3]);
});

test("a mixed selection's deploy reaches only the units that deploy", () => {
  const selection = [
    { id: 0, kind: "tank", garrison: null },
    { id: 7, kind: "supply", garrison: null },
    { id: 9, kind: "rifle", garrison: { building: 3, soldiers: 4, ports: 2 } },
  ] as unknown as OwnUnitView[];
  expect(reach("deploy", selection, UNITS).map((u) => u.id)).toEqual([7]);
  expect(reach("garrison", selection, UNITS).map((u) => u.id)).toEqual([9]);
  expect(reach("exit_building", selection, UNITS).map((u) => u.id)).toEqual([9]);
  expect(reach("deploy", selection.slice(0, 1), UNITS)).toEqual([]);
});

test("an attack reaches only the units whose type has mounts", () => {
  // The simulation pursues through the weapons pass, which never runs for a
  // unit with no mounts: an attack given to a supply truck froze it.
  const selection = [
    { id: 0, kind: "tank", garrison: null },
    { id: 7, kind: "supply", garrison: null },
    { id: 9, kind: "rifle", garrison: null },
  ] as unknown as OwnUnitView[];
  expect(reach("attack", selection, UNITS).map((u) => u.id)).toEqual([0, 9]);
  expect(reach("attack", selection.slice(1, 2), UNITS)).toEqual([]);
});
