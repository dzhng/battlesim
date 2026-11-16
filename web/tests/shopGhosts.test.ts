// @vitest-environment node
import { expect, test } from "vitest";
import type { PurchaseGhost } from "@web/battle/input/purchasePlacement";
import type { OwnUnitView } from "@web/battle/sim/observationView";
import { BOUGHT_FADE_MS, boughtGhosts, type Bought } from "@apps/battle-lab/src/shopGhosts";

const tank: PurchaseGhost = {
  variant: "test_tank",
  destination: [100, 200],
  facing: 0,
  valid: true,
  spots: [],
  areaRadius: null,
};
const bought: Bought = { ghost: tank, at: 1000, after: 7 };
const unit = (id: number, kind: string, at: [number, number]) =>
  ({ id, kind, position: [at[0], at[1], 0] }) as OwnUnitView;
const opacity = (now: number, own: OwnUnitView[] = []) =>
  boughtGhosts([bought], now, own)[0]?.opacity;

test("a bought unit's ghost stays where it was placed, fading until it is gone", () => {
  const [ghost] = boughtGhosts([bought], 1000, []);
  expect(ghost).toMatchObject({ kind: "test_tank", at: [100, 200] });
  expect(opacity(1000)).toBe(1);
  const early = opacity(1000 + BOUGHT_FADE_MS * 0.25)!;
  const late = opacity(1000 + BOUGHT_FADE_MS * 0.75)!;
  expect(early).toBeLessThan(1);
  expect(late).toBeLessThan(early);
  expect(late).toBeGreaterThan(0);
  expect(boughtGhosts([bought], 1000 + BOUGHT_FADE_MS, [])).toEqual([]);
});

test("the ghost goes as soon as the unit it bought reaches its spot", () => {
  const now = 1000 + BOUGHT_FADE_MS * 0.25;
  // On its way, still far off: the ghost stays.
  expect(opacity(now, [unit(8, "test_tank", [100, 400])])).toBeGreaterThan(0);
  expect(boughtGhosts([bought], now, [unit(8, "test_tank", [100, 200])])).toEqual([]);
});

test("a unit already there, or of another kind, does not take the ghost's place", () => {
  const now = 1000 + BOUGHT_FADE_MS * 0.25;
  // Unit 7 was on the field before the purchase; unit 8 is something else.
  const there = [unit(7, "test_tank", [100, 200]), unit(8, "test_scout", [100, 200])];
  expect(opacity(now, there)).toBeGreaterThan(0);
});

test("a squad bought aboard a carrier is ghosted, and arrives, as its carrier", () => {
  const pair: Bought = {
    ...bought,
    ghost: { ...tank, variant: "test_squad", carrier: "test_apc" },
  };
  expect(boughtGhosts([pair], 1000, [])[0]?.kind).toBe("test_apc");
  expect(boughtGhosts([pair], 1000, [unit(9, "test_apc", [100, 200])])).toEqual([]);
});
