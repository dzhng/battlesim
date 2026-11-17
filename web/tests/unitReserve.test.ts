// An army card warns about the same reserve its unit panel shows: the
// emptiest finite ammunition kind, or a supply truck's stock. Unlimited kinds
// are skipped. The test sets its own capacities, so a catalog retune cannot
// move it.
import { expect, test } from "vitest";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { outOfAmmo, ownPanel, unitReserve, type PanelRules } from "../src/battle/present/panelRows";
import type { OwnUnitView } from "../src/battle/sim/observationView";
import { TEST_CATALOG } from "./catalog";

const RULES = {
  tick_hz: 30,
  weapons: {
    tank_ap: { name: "AP", icon: "ap_shell", ammo: 10 },
    tank_he: { name: "HE", icon: "he_shell", ammo: 20 },
    hmg: { name: "HMG", icon: "hmg", ammo: "unlimited" },
    rifle: { name: "Rifle", icon: "rifle", ammo: "unlimited" },
  },
  service: { radius_m: 50 },
} as unknown as PanelRules;

const tank = (ammo: number[]) => specimenUnit(TEST_CATALOG, "test_tank", {}, [{ ammo }]);
const reserveOf = (unit: OwnUnitView, units = TEST_CATALOG.units) =>
  unitReserve(units, unit, RULES);
/** The panel's own out-of-ammo mark, from the same weapons it draws. */
const panelOut = (unit: OwnUnitView) =>
  outOfAmmo(ownPanel(TEST_CATALOG.units, unit, [unit], RULES).weapons);

test("the emptiest finite kind sets the fill, and an empty one is out", () => {
  expect(reserveOf(tank([10, 20]))).toEqual({ resource: "ammo", fill: 1, out: false });
  // AP is the emptiest kind: HE's fuller magazine does not average it away.
  expect(reserveOf(tank([5, 20]))?.fill).toBeCloseTo(0.5);
  expect(reserveOf(tank([0, 20]))).toEqual({ resource: "ammo", fill: 0, out: true });
});

test("the card and the unit's panel agree on when ammunition is out", () => {
  for (const ammo of [
    [10, 20],
    [5, 5],
    [0, 20],
    [0, 0],
  ]) {
    const unit = tank(ammo);
    expect(panelOut(unit), String(ammo)).toBe(reserveOf(unit)!.out);
  }
});

test("a supply truck's stock is its reserve, and empty is out", () => {
  // The stock's full amount is the test's, not the supply type's.
  const supplyUnits = {
    type: () => ({ mounts: [], capabilities: { supply: { stock: 40 } } }),
  } as unknown as UnitCatalog;
  expect(reserveOf(specimenUnit(TEST_CATALOG, "test_supply", { stock: 0 }), supplyUnits)).toEqual({
    resource: "stock",
    fill: 0,
    out: true,
  });
  expect(
    reserveOf(specimenUnit(TEST_CATALOG, "test_supply", { stock: 10 }), supplyUnits)?.fill,
  ).toBeCloseTo(0.25);
});

test("unlimited-only units have no reserve to warn about", () => {
  expect(reserveOf(specimenUnit(TEST_CATALOG, "test_recon"))).toBeNull();
});
