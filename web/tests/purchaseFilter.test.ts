// The picker's filter core: which cards a query matches, whether a card can be
// bought now, and which card Enter chooses. Pure, so it is tested without a
// page. Fakes only, never the roster.
import { expect, test } from "vitest";
import type { UnitCard } from "@packages/scene-assets/src/units";
import {
  cardPurchasable,
  enterChoice,
  filterCards,
  type BuyFacts,
} from "@web/battle/present/purchaseFilter";

const card = (
  id: string,
  family: string,
  family_name: string,
  variant: string,
  cost: number,
  disabled_reason: string | null = null,
): UnitCard => ({
  id,
  name: `${family_name} ${variant}`,
  description: "",
  family,
  cost,
  roster: { factions: ["us"], family_name, category: "veh", variant },
  disabled_reason,
  planned_weapons: [],
});

const TANK_BASE = card("base", "abrams", "M1 Abrams", "SEP v2", 200);
const TANK_TROPHY = card(
  "trophy",
  "abrams",
  "M1 Abrams",
  "SEP v2 Trophy",
  520,
  "Protection deferred",
);
const JEEP = card("jeep", "humvee", "HMMWV", "Base", 80);

const facts = (over: Partial<BuyFacts> = {}): BuyFacts => ({
  credits: 1000,
  occupiedSlots: 0,
  maxUnits: 30,
  finished: false,
  ...over,
});

const CARDS = [TANK_BASE, TANK_TROPHY, JEEP] as const;

test("a family name matches every variant of that family", () => {
  expect(filterCards(CARDS, "abrams")).toEqual([TANK_BASE, TANK_TROPHY]);
});

test("a variant name matches only that variant", () => {
  expect(filterCards(CARDS, "trophy")).toEqual([TANK_TROPHY]);
});

test("every whitespace token must match, and case is ignored", () => {
  expect(filterCards(CARDS, "SEP TROPHY")).toEqual([TANK_TROPHY]);
  expect(filterCards(CARDS, "sep v2")).toEqual([TANK_BASE, TANK_TROPHY]);
  expect(filterCards(CARDS, "hmmwv base")).toEqual([JEEP]);
});

test("an unmatched query returns nothing, and a blank one returns every card in order", () => {
  expect(filterCards(CARDS, "zzz")).toEqual([]);
  expect(filterCards(CARDS, "")).toEqual([...CARDS]);
  expect(filterCards(CARDS, "   ")).toEqual([...CARDS]);
});

test("a card is purchasable only when nothing blocks it", () => {
  expect(cardPurchasable(TANK_BASE, facts())).toBe(true);
  expect(cardPurchasable(TANK_BASE, facts({ credits: 199 }))).toBe(false);
  expect(cardPurchasable(TANK_BASE, facts({ credits: 200 }))).toBe(true);
  expect(cardPurchasable(TANK_BASE, facts({ occupiedSlots: 30 }))).toBe(false);
  expect(cardPurchasable(TANK_BASE, facts({ occupiedSlots: 29 }))).toBe(true);
  expect(cardPurchasable(TANK_BASE, facts({ finished: true }))).toBe(false);
  expect(cardPurchasable(TANK_TROPHY, facts())).toBe(false);
});

test("Enter buys the sole family's first purchasable variant", () => {
  const dearer = [
    card("base", "abrams", "M1 Abrams", "SEP v2", 600),
    card("trophy", "abrams", "M1 Abrams", "SEP v2 Trophy", 520),
  ];
  // The first variant is beyond the credits; the second is the first available.
  expect(enterChoice(filterCards(dearer, "abrams"), facts({ credits: 550 }))).toBe(dearer[1]);
  // With both affordable, the first in catalog order wins.
  expect(enterChoice(filterCards([TANK_BASE, TANK_TROPHY], "abrams"), facts())).toBe(TANK_BASE);
});

test("Enter does nothing with more than one family, no purchasable card, or no match", () => {
  expect(enterChoice([TANK_BASE, JEEP], facts())).toBeNull();
  expect(enterChoice(filterCards(CARDS, "trophy"), facts())).toBeNull();
  expect(enterChoice(filterCards(CARDS, "zzz"), facts())).toBeNull();
  expect(enterChoice([], facts())).toBeNull();
});
