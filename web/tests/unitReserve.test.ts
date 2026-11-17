// An army card warns about the reserve that can actually run out: finite
// rounds, or a supply truck's stock. Unlimited kinds are skipped, and a unit
// that keeps an unlimited weapon is never "out" in the red sense. The test
// sets its own capacities, so a catalog retune cannot move it.
import { expect, test } from "vitest";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { unitReserve, type PanelRules } from "../src/battle/present/panelRows";
import type { OwnUnitView } from "../src/battle/sim/observationView";
import { TEST_CATALOG } from "./catalog";

const RULES = {
  weapons: {
    tank_ap: { name: "AP", icon: "ap_shell", ammo: 10 },
    tank_he: { name: "HE", icon: "he_shell", ammo: 20 },
    hmg: { name: "HMG", icon: "hmg", ammo: "unlimited" },
    rifle: { name: "Rifle", icon: "rifle", ammo: "unlimited" },
  },
} as unknown as PanelRules;

const tank = (ammo: number[]) => specimenUnit(TEST_CATALOG, "test_tank", {}, [{ ammo }]);
const reserveOf = (unit: OwnUnitView, units = TEST_CATALOG.units) =>
  unitReserve(units, unit, RULES);

test("finite kinds count once each and unlimited kinds are skipped", () => {
  expect(reserveOf(tank([10, 20]))).toEqual({
    resource: "ammo",
    fill: 1,
    out: false,
    stillArmed: true,
  });
  // AP and HE weigh the same, and the unlimited HMG makes no third kind.
  expect(reserveOf(tank([5, 5]))?.fill).toBeCloseTo(0.375);
});

test("an empty finite reserve is still only a warning while an unlimited gun remains", () => {
  expect(reserveOf(tank([0, 0]))).toEqual({
    resource: "ammo",
    fill: 0,
    out: true,
    stillArmed: true,
  });
});

test("a unit with no unlimited weapon is useless once its reserve is empty", () => {
  // The stock's full amount is the test's, not the supply type's.
  const supplyUnits = {
    type: () => ({ mounts: [], capabilities: { supply: { stock: 40 } } }),
  } as unknown as UnitCatalog;
  expect(reserveOf(specimenUnit(TEST_CATALOG, "test_supply", { stock: 0 }), supplyUnits)).toEqual({
    resource: "stock",
    fill: 0,
    out: true,
    stillArmed: false,
  });
  expect(
    reserveOf(specimenUnit(TEST_CATALOG, "test_supply", { stock: 10 }), supplyUnits)?.fill,
  ).toBeCloseTo(0.25);
});

test("unlimited-only units have no reserve to warn about", () => {
  expect(reserveOf(specimenUnit(TEST_CATALOG, "test_recon"))).toBeNull();
});
