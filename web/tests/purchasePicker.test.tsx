import { shippedRules } from "@web/shippedRules";
import { cleanup, fireEvent, render as renderView } from "@testing-library/react";
import { WithTestCatalog } from "./catalog";
import { afterEach, expect, test, vi } from "vitest";
import { PurchasePicker } from "@web/battle/present/purchasePicker";
import type { UnitCard, UnitCategory } from "@packages/scene-assets/src/units";
import type { SkirmishView } from "@web/battle/sim/observationView";
import type { PanelRules } from "@web/battle/present/panelRows";

const RULES = shippedRules as unknown as PanelRules;

/** Views render under the test set, as a lab page provides it. */
const render = (ui: Parameters<typeof renderView>[0]) =>
  renderView(ui, { wrapper: WithTestCatalog });
afterEach(cleanup);
const card = (id: string, variant: string, reason: string | null = null): UnitCard => ({
  id,
  name: variant,
  description: "",
  family: "abrams",
  cost: 200,
  roster: { factions: ["us"], family_name: "M1 Abrams", category: "veh", variant },
  disabled_reason: reason,
  planned_weapons: [],
});
const match: SkirmishView = {
  phase: "preparation",
  players: [
    {
      index: 0,
      id: "blue-1",
      controller: "human",
      ready: false,
      credits: 1000,
      unitCount: 0,
      cap: 30,
      pending: [],
    },
  ],
  preparationRemainingS: 60,
  scores: [0, 0],
  result: null,
  objectives: [],
  report: null,
};
test("an empty army can inspect one family and purchase its enabled concrete variant", () => {
  const choose = vi.fn();
  const view = render(
    <PurchasePicker
      cards={[card("base", "SEP v2"), card("trophy", "SEP v2 Trophy", "Protection deferred")]}
      faction="us"
      match={match}
      onChoose={choose}
      rules={RULES}
    />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  expect(view.getAllByRole("button", { name: "M1 Abrams" })).toHaveLength(1);
  fireEvent.click(view.getByRole("button", { name: "M1 Abrams" }));
  expect(choose).toHaveBeenCalledExactlyOnceWith("base");
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  fireEvent.mouseEnter(view.getByRole("button", { name: "M1 Abrams" }));
  const unavailable = view.getByRole("button", {
    name: "SEP v2 Trophy — 200 credits — Unavailable",
  });
  expect(unavailable.hasAttribute("disabled")).toBe(true);
  fireEvent.click(unavailable);
  expect(choose).toHaveBeenCalledTimes(1);
  expect(view.queryByLabelText("Unit slots")).toBeNull();
});

test("the picker opens fresh, however it was last closed", () => {
  const view = render(
    <PurchasePicker
      cards={[card("base", "SEP v2")]}
      faction="us"
      match={match}
      onChoose={vi.fn()}
      rules={RULES}
    />,
  );
  const toggle = () => fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  const expectFresh = () => {
    expect(view.getByRole("tab", { name: "REC" }).getAttribute("aria-selected")).toBe("true");
    expect(view.queryByRole("group", { name: "M1 Abrams variants" })).toBeNull();
  };
  // Closed by buying.
  toggle();
  fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  fireEvent.click(view.getByRole("button", { name: "M1 Abrams" }));
  toggle();
  expectFresh();
  // Closed by its button, with a family's info cards open.
  fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  fireEvent.mouseEnter(view.getByRole("button", { name: "M1 Abrams" }));
  expect(view.getByRole("group", { name: "M1 Abrams variants" })).toBeTruthy();
  toggle();
  toggle();
  expectFresh();
});

test("resting the pointer on a category tab switches to it, without a click", () => {
  const view = render(
    <PurchasePicker
      cards={[card("base", "SEP v2")]}
      faction="us"
      match={match}
      onChoose={vi.fn()}
      rules={RULES}
    />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  expect(view.queryByRole("button", { name: "M1 Abrams" })).toBeNull();
  fireEvent.mouseEnter(view.getByRole("tab", { name: "VEH" }));
  expect(view.getByRole("tab", { name: "VEH" }).getAttribute("aria-selected")).toBe("true");
  fireEvent.mouseEnter(view.getByRole("button", { name: "M1 Abrams" }));
  expect(view.getByRole("group", { name: "M1 Abrams variants" })).toBeTruthy();
  // Passing back over the shown tab keeps the open family's info cards.
  fireEvent.mouseEnter(view.getByRole("tab", { name: "VEH" }));
  expect(view.getByRole("group", { name: "M1 Abrams variants" })).toBeTruthy();
});

test("purchase availability follows observed credits and the unit cap's reserved room", () => {
  const choose = vi.fn();
  const props = {
    cards: [card("base", "SEP v2")],
    faction: "us" as const,
    onChoose: choose,
    rules: RULES,
  };
  // A one-variant family is bought by clicking it, once it is affordable
  // and the unit cap has room.
  const view = render(
    <PurchasePicker
      {...props}
      match={{ ...match, players: [{ ...match.players[0], credits: 199 }] }}
    />,
  );
  const family = () => view.getByRole("button", { name: "M1 Abrams" });
  const open = () => {
    fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
    fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  };
  open();
  fireEvent.click(family());
  expect(choose).not.toHaveBeenCalled();
  view.rerender(
    <PurchasePicker
      {...props}
      match={{ ...match, players: [{ ...match.players[0], credits: 200, unitCount: 30 }] }}
    />,
  );
  fireEvent.click(family());
  expect(choose).not.toHaveBeenCalled();
  view.rerender(
    <PurchasePicker
      {...props}
      match={{ ...match, players: [{ ...match.players[0], credits: 200, unitCount: 29 }] }}
    />,
  );
  fireEvent.click(family());
  expect(choose).toHaveBeenCalledExactlyOnceWith("base");
});

test("clicking a unit family immediately arms its first available variant", () => {
  const choose = vi.fn();
  const view = render(
    <PurchasePicker
      cards={[card("base", "SEP v2"), card("trophy", "SEP v2 Trophy", "Protection deferred")]}
      faction="us"
      match={match}
      onChoose={choose}
      rules={RULES}
    />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  fireEvent.click(view.getByRole("button", { name: "M1 Abrams" }));
  expect(choose).toHaveBeenCalledExactlyOnceWith("base");
});

test("an unavailable family shows its card's silhouette, marked unavailable and unpickable", () => {
  const choose = vi.fn();
  const jet: UnitCard = {
    ...card("f_16c_block_50", "F-16C Block 50", "Aircraft mechanics deferred"),
    family: "f_16_fighting_falcon",
    roster: { factions: ["us"], family_name: "F-16", category: "air", variant: "F-16C Block 50" },
  };
  const view = render(
    <PurchasePicker cards={[jet]} faction="us" match={match} onChoose={choose} rules={RULES} />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  fireEvent.click(view.getByRole("tab", { name: "AIR" }));
  const family = view.getByRole("button", { name: "F-16" });
  expect(family.querySelector(".ro-icon svg")).not.toBeNull();
  expect(family.classList.contains("unavailable")).toBe(true);
  fireEvent.click(family);
  expect(choose).not.toHaveBeenCalled();
});

test("a one-variant family's info card buys it, as its card does", () => {
  const choose = vi.fn();
  const view = render(
    <PurchasePicker
      cards={[card("base", "SEP v2")]}
      faction="us"
      match={match}
      onChoose={choose}
      rules={RULES}
    />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  fireEvent.mouseEnter(view.getByRole("button", { name: "M1 Abrams" }));
  fireEvent.click(view.getByRole("button", { name: "SEP v2 — 200 credits" }));
  expect(choose).toHaveBeenCalledExactlyOnceWith("base");
});

/** A card for an invented family, so tests can span categories. */
const familyCard = (
  id: string,
  family: string,
  family_name: string,
  variant: string,
  category: UnitCategory,
): UnitCard => ({
  ...card(id, variant),
  family,
  roster: { factions: ["us"], family_name, category, variant },
});

test("typing a family or variant name narrows the grid to that family", () => {
  const cards = [
    familyCard("base", "abrams", "M1 Abrams", "SEP v2", "veh"),
    familyCard("jeep", "humvee", "HMMWV", "Base", "veh"),
  ];
  const view = render(
    <PurchasePicker cards={cards} faction="us" match={match} onChoose={vi.fn()} rules={RULES} />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  const field = view.getByRole("textbox", { name: "Search units" });
  fireEvent.change(field, { target: { value: "hmmwv" } });
  expect(view.queryByRole("button", { name: "M1 Abrams" })).toBeNull();
  expect(view.getByRole("button", { name: "HMMWV" })).toBeTruthy();
  // A variant name finds its family too.
  fireEvent.change(field, { target: { value: "sep v2" } });
  expect(view.getByRole("button", { name: "M1 Abrams" })).toBeTruthy();
  expect(view.queryByRole("button", { name: "HMMWV" })).toBeNull();
});

test("a search spans every category and dims the tabs", () => {
  const cards = [
    familyCard("rifle", "rifle_section", "Rifle Section", "Base", "inf"),
    familyCard("base", "abrams", "M1 Abrams", "SEP v2", "veh"),
  ];
  const view = render(
    <PurchasePicker cards={cards} faction="us" match={match} onChoose={vi.fn()} rules={RULES} />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  expect(view.queryByRole("button", { name: "Rifle Section" })).toBeNull();
  fireEvent.change(view.getByRole("textbox", { name: "Search units" }), {
    target: { value: "rifle" },
  });
  // The infantry family shows although the VEH tab is the one shown.
  expect(view.getByRole("button", { name: "Rifle Section" })).toBeTruthy();
  expect(view.queryByRole("button", { name: "M1 Abrams" })).toBeNull();
  expect(view.getByRole("tab", { name: "VEH" }).hasAttribute("disabled")).toBe(true);
});

test("an unmatched search says so, and clearing restores the shown category", () => {
  const cards = [
    familyCard("rifle", "rifle_section", "Rifle Section", "Base", "inf"),
    familyCard("base", "abrams", "M1 Abrams", "SEP v2", "veh"),
  ];
  const view = render(
    <PurchasePicker cards={cards} faction="us" match={match} onChoose={vi.fn()} rules={RULES} />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  const field = view.getByRole("textbox", { name: "Search units" });
  fireEvent.change(field, { target: { value: "zzz" } });
  expect(view.getByText("NO MATCH")).toBeTruthy();
  expect(view.queryByRole("button", { name: "M1 Abrams" })).toBeNull();
  fireEvent.change(field, { target: { value: "" } });
  expect(view.queryByText("NO MATCH")).toBeNull();
  expect(view.getByRole("button", { name: "M1 Abrams" })).toBeTruthy();
  expect(view.getByRole("tab", { name: "VEH" }).getAttribute("aria-selected")).toBe("true");
});

test("Enter buys the sole matching family's first available variant", () => {
  const choose = vi.fn();
  const view = render(
    <PurchasePicker
      cards={[card("base", "SEP v2")]}
      faction="us"
      match={match}
      onChoose={choose}
      rules={RULES}
    />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  const field = view.getByRole("textbox", { name: "Search units" });
  fireEvent.change(field, { target: { value: "abrams" } });
  fireEvent.keyDown(field, { key: "Enter" });
  expect(choose).toHaveBeenCalledExactlyOnceWith("base");
});

test("Enter buys the exact variant a variant-name search found", () => {
  const choose = vi.fn();
  const cards = [
    familyCard("base", "humvee", "HMMWV", "Base", "veh"),
    familyCard("hmg", "humvee", "HMMWV", "HMG", "veh"),
  ];
  const view = render(
    <PurchasePicker cards={cards} faction="us" match={match} onChoose={choose} rules={RULES} />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  const field = view.getByRole("textbox", { name: "Search units" });
  fireEvent.change(field, { target: { value: "hmg" } });
  fireEvent.keyDown(field, { key: "Enter" });
  // The HMG variant, not the family's first available (Base).
  expect(choose).toHaveBeenCalledExactlyOnceWith("hmg");
});

test("Enter stays out of the way with several families or nothing to buy", () => {
  const cards = [
    familyCard("base", "abrams", "M1 Abrams", "SEP v2", "veh"),
    {
      ...familyCard("jeep", "humvee", "HMMWV", "Base", "veh"),
      disabled_reason: "Protection deferred",
    },
  ];
  const choose = vi.fn();
  const view = render(
    <PurchasePicker cards={cards} faction="us" match={match} onChoose={choose} rules={RULES} />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  const field = view.getByRole("textbox", { name: "Search units" });
  fireEvent.change(field, { target: { value: "m" } });
  fireEvent.keyDown(field, { key: "Enter" });
  expect(choose).not.toHaveBeenCalled();
  // One family, but its only variant is unavailable.
  fireEvent.change(field, { target: { value: "hmmwv" } });
  fireEvent.keyDown(field, { key: "Enter" });
  expect(choose).not.toHaveBeenCalled();
});

test("Escape clears the filter first, then closes and returns focus to its button", () => {
  const view = render(
    <PurchasePicker
      cards={[card("base", "SEP v2")]}
      faction="us"
      match={match}
      onChoose={vi.fn()}
      rules={RULES}
    />,
  );
  const toggle = view.getByRole("button", { name: "Reinforcements" });
  fireEvent.click(toggle);
  const field = () => view.getByRole("textbox", { name: "Search units" });
  fireEvent.change(field(), { target: { value: "abrams" } });
  // The pause menu owns Escape unless the field cancels it.
  expect(fireEvent.keyDown(field(), { key: "Escape" })).toBe(false);
  expect((field() as HTMLInputElement).value).toBe("");
  expect(view.getByRole("button", { name: "Reinforcements" })).toBeTruthy();
  expect(fireEvent.keyDown(field(), { key: "Escape" })).toBe(false);
  expect(view.queryByRole("textbox", { name: "Search units" })).toBeNull();
  expect(document.activeElement).toBe(toggle);
});

test("V opens the shop and stays a search character in the field", () => {
  const view = render(
    <PurchasePicker
      cards={[card("base", "SEP v2")]}
      faction="us"
      match={match}
      onChoose={vi.fn()}
      rules={RULES}
    />,
  );
  const toggle = view.getByRole("button", { name: "Reinforcements" });
  expect(toggle.getAttribute("aria-keyshortcuts")).toBe("V");
  // The visible label stays the name; the tooltip carries the key, as the
  // menu button's does, so no standalone control spells its shortcut.
  expect(toggle.textContent?.trim()).toBe("Reinforcements");
  expect(toggle.getAttribute("title")).toBe("Reinforcements (V)");
  fireEvent.keyDown(window, { code: "KeyV" });
  const field = view.getByRole("textbox", { name: "Search units" });
  // The field owns its typing: the press is not consumed and the shop stays open.
  expect(fireEvent.keyDown(field, { code: "KeyV", cancelable: true })).toBe(true);
  expect(view.getByRole("textbox", { name: "Search units" })).toBeTruthy();
  // The button is the close, as it was before the key.
  fireEvent.click(toggle);
  expect(view.queryByRole("textbox", { name: "Search units" })).toBeNull();
});

test("V does nothing while the battle is covered", () => {
  const view = render(
    <PurchasePicker
      cards={[card("base", "SEP v2")]}
      faction="us"
      match={match}
      onChoose={vi.fn()}
      rules={RULES}
      enabled={false}
    />,
  );
  fireEvent.keyDown(window, { code: "KeyV" });
  expect(view.queryByRole("textbox", { name: "Search units" })).toBeNull();
});
