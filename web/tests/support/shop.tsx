/** The reinforcements shop's test fixtures: invented cards and the test
 *  catalog's squads and carriers, a skirmish view, the shop rendered under
 *  the test set, and readers of what it shows. */
import { render as renderView, within } from "@testing-library/react";
import { UNIT_GROUPS } from "@web/battle/present/purchaseFilter";
import type { UnitCard, UnitCategory } from "@packages/scene-assets/src/units";
import type { SkirmishView } from "@web/battle/sim/observationView";
import type { PanelRules } from "@web/battle/present/panelRows";
import { WithTestCatalog, TEST_RULES } from "../catalog";

/** A group tab's label, by its id: the labels are data. */
export const tab = (id: string) => UNIT_GROUPS.find((g) => g.id === id)!.label;

export const RULES = TEST_RULES as unknown as PanelRules;

/** Views render under the test set, as a lab page provides it. */
export const render = (ui: Parameters<typeof renderView>[0]) =>
  renderView(ui, { wrapper: WithTestCatalog });
/** A variant of an invented family, named `<family> <variant>`. None is a
 *  catalog type, so each browses under its category's first group. */
export const card = (
  id: string,
  variant: string,
  {
    cost = 200,
    reason = null,
    family = "abrams",
    family_name = "M1 Abrams",
    category = "veh",
  }: {
    cost?: number;
    reason?: string | null;
    family?: string;
    family_name?: string;
    category?: UnitCategory;
  } = {},
): UnitCard => ({
  id,
  name: `${family_name} ${variant}`,
  description: "",
  family,
  cost,
  roster: { factions: ["us"], family_name, category, variant },
  disabled_reason: reason,
  planned_weapons: [],
});
export const match: SkirmishView = {
  phase: "preparation",
  players: [
    {
      index: 0,
      id: "blue-1",
      controller: "human",
      ready: false,
      credits: 1000,
      incomePerMinute: 200,
      unitCount: 0,
      cap: 30,
      pending: [],
      offered: null,
    },
  ],
  preparationRemainingS: 60,
  scores: [0, 0],
  result: null,
  objectives: [],
  report: null,
};
/** The shown cards' names, in grid order. */
export const shownNames = (view: ReturnType<typeof render>) =>
  within(view.getByRole("tabpanel"))
    .queryAllByRole("button")
    .map((b) => b.getAttribute("aria-label"));

// A squad's ride. Squads and carriers are test catalog types: a card is a
// squad by its soldier slots and a carrier by its seats. Prices are the
// cards' own; the APC is cheaper than the scout car though listed after it,
// and the helicopter is listed before both though its tab comes after.
export const squad = (id: string, family_name: string, cost: number) =>
  card(id, "Base", { family: id, family_name, category: "inf", cost });
export const ride = (
  id: string,
  family_name: string,
  cost: number,
  category: UnitCategory = "veh",
) => card(id, "Base", { family: id, family_name, category, cost });
export const RIFLES = squad("test_rifle", "Rifles", 100);
export const TEAM = squad("test_at", "AT Team", 150);
export const HELI = ride("test_transport_heli", "Lift Heli", 300, "hel");
export const SCOUT = ride("test_scout_car", "Scout Car", 90);
export const APC = ride("test_apc", "APC", 60);
export const TANK = ride("test_tank", "Tank", 200);
/** Never a carrier: a planned card has no type, so no seats. */
export const PLANNED = ride("test_planned_ride", "Planned Ride", 40);
export const RIDES = [RIFLES, TEAM, HELI, SCOUT, APC, TANK, PLANNED];
/** The open transport menu's sections, each its label and its cards'
 *  names in order; null with no menu open. */
export const transportMenu = (view: ReturnType<typeof render>) => {
  const menu = view.queryByRole("region", { name: "Transport" });
  return (
    menu &&
    within(menu)
      .getAllByRole("group")
      .map((g) => [
        g.getAttribute("aria-label"),
        within(g)
          .getAllByRole("button")
          .map((b) => b.getAttribute("aria-label")),
      ])
  );
};

/** The menu's highlighted options. */
export const highlighted = (view: ReturnType<typeof render>) =>
  within(view.getByRole("region", { name: "Transport" }))
    .queryAllByRole("button", { pressed: true })
    .map((b) => b.getAttribute("aria-label"));
