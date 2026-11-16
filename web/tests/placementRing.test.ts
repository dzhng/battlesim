// @vitest-environment node
// A purchase being placed is under the pointer, so its ghost shows the ring
// its unit will show once it stands there: the same circle, pointing the
// same way. A squad's ring waits for the preview that names its area; a
// winged aircraft never stops over a point and has none.
import { expect, test } from "vitest";
import { unitCircle } from "@packages/battle-renderer/src/unitCircle";
import { fixedWing, airborne } from "@packages/scene-assets/src/units";
import { orderView } from "@apps/battle-lab/src/battleOverlay";
import { gameOrderStyle } from "@apps/battle-lab/src/gameOverlay";
import { placementCircle } from "@apps/battle-lab/src/pointerPaint";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import type { PurchaseGhost } from "@web/battle/input/purchasePlacement";
import type { OwnUnitView } from "@web/battle/sim/observationView";
import { TEST_CATALOG, UNITS } from "./catalog";

const AT: [number, number] = [120, 80];
const FACING = 0.7;

const ghost = (variant: string, patch: Partial<PurchaseGhost> = {}): PurchaseGhost => ({
  variant,
  destination: AT,
  facing: FACING,
  valid: true,
  spots: [],
  areaRadius: null,
  ...patch,
});

/** The circle `kind` shows selected once it holds at the ghost's spot. */
function standing(kind: string, patch: Partial<OwnUnitView> = {}) {
  const unit = specimenUnit(TEST_CATALOG, kind, {
    position: [AT[0], AT[1], 0],
    yaw: FACING,
    goal: null,
    route: [],
    steps: [],
    members: [],
    memberOrders: [],
    area: null,
    ...patch,
  } as Partial<OwnUnitView>);
  return unitCircle(orderView(UNITS, unit, true), gameOrderStyle);
}

test("a vehicle's ghost shows the ring the vehicle will show standing there", () => {
  expect(placementCircle(ghost("test_tank"), UNITS)).toEqual(standing("test_tank"));
});

test("a squad's ghost shows its area's ring once the preview names it, and none before", () => {
  const spots: [number, number][] = [
    [AT[0] + 2, AT[1]],
    [AT[0] - 2, AT[1]],
  ];
  expect(placementCircle(ghost("test_rifle"), UNITS), "before the preview").toBeNull();
  expect(placementCircle(ghost("test_rifle", { spots, areaRadius: 9 }), UNITS)).toEqual(
    standing("test_rifle", {
      members: spots.map(([x, y]) => [x, y, 0] as [number, number, number]),
      area: { anchor: AT, radius: 9 },
    }),
  );
});

test("a helicopter's ghost shows its ring; a winged aircraft's shows none", () => {
  const kinds = UNITS.view.units.map((t) => t.id);
  const rotor = kinds.find((k) => airborne(UNITS.type(k)) && !fixedWing(UNITS.type(k)))!;
  const jet = kinds.find((k) => fixedWing(UNITS.type(k)))!;
  expect(placementCircle(ghost(rotor), UNITS)).toEqual(standing(rotor));
  expect(placementCircle(ghost(rotor), UNITS)).not.toBeNull();
  expect(placementCircle(ghost(jet), UNITS)).toBeNull();
});
