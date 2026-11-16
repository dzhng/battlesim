import { shippedRules } from "@web/shippedRules";
import { UNIT_GROUPS } from "@web/battle/present/purchaseFilter";
import { cleanup, fireEvent, render as renderView, within } from "@testing-library/react";
import { WithTestCatalog } from "./catalog";
import { afterEach, expect, test, vi } from "vitest";
import { PurchasePicker } from "@web/battle/present/purchasePicker";
import type { UnitCard, UnitCategory } from "@packages/scene-assets/src/units";
import type { SkirmishView } from "@web/battle/sim/observationView";
import type { PanelRules } from "@web/battle/present/panelRows";

/** A group tab's label, by its id: the labels are data. */
const tab = (id: string) => UNIT_GROUPS.find((g) => g.id === id)!.label;

const RULES = shippedRules as unknown as PanelRules;

/** Views render under the test set, as a lab page provides it. */
const render = (ui: Parameters<typeof renderView>[0]) =>
  renderView(ui, { wrapper: WithTestCatalog });
afterEach(cleanup);
/** A variant of an invented family, named `<family> <variant>`. None is a
 *  catalog type, so each browses under its category's first group. */
const card = (
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
const BASE = card("base", "SEP v2");
const TROPHY = card("trophy", "SEP v2 Trophy", { cost: 520 });
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
/** Renders the picker over `cards` with `player` overrides, opened on the
 *  tank tab; returns the view and its choice spy. */
function openShop(cards: UnitCard[], player: Partial<SkirmishView["players"][0]> = {}) {
  const choose = vi.fn();
  const view = render(
    <PurchasePicker
      cards={cards}
      faction="us"
      match={{ ...match, players: [{ ...match.players[0], ...player }] }}
      onChoose={choose}
      rules={RULES}
    />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  fireEvent.click(view.getByRole("tab", { name: tab("mbt") }));
  return { view, choose };
}

/** The shown cards' names, in grid order. */
const shownNames = (view: ReturnType<typeof render>) =>
  within(view.getByRole("tabpanel"))
    .queryAllByRole("button")
    .map((b) => b.getAttribute("aria-label"));

test("every variant is its own card, and clicking one places exactly that variant", () => {
  const { view, choose } = openShop([BASE, TROPHY]);
  expect(shownNames(view)).toEqual([BASE.name, TROPHY.name]);
  // The dearer sibling, not the family's first affordable one.
  fireEvent.click(view.getByRole("button", { name: TROPHY.name }));
  expect(choose).toHaveBeenCalledExactlyOnceWith("trophy");
});

test("hovering a card shows that variant's info card alone, at once", () => {
  const { view } = openShop([BASE, TROPHY]);
  expect(view.queryByRole("tooltip")).toBeNull();
  fireEvent.mouseEnter(view.getByRole("button", { name: BASE.name }));
  expect(view.getAllByRole("tooltip").map((t) => t.getAttribute("aria-label"))).toEqual([
    BASE.name,
  ]);
  // Straight onto its sibling: no hover-intent wait.
  fireEvent.mouseEnter(view.getByRole("button", { name: TROPHY.name }));
  expect(view.getAllByRole("tooltip").map((t) => t.getAttribute("aria-label"))).toEqual([
    TROPHY.name,
  ]);
  expect(view.getByRole("button", { name: TROPHY.name }).getAttribute("aria-describedby")).toBe(
    view.getByRole("tooltip").id,
  );
  // Leaving the picker puts it away.
  fireEvent.mouseLeave(view.getByRole("region", { name: "Faction units" }));
  expect(view.queryByRole("tooltip")).toBeNull();
});

test("a tab's cards stand by family, each family cheapest first", () => {
  const jeep = { family: "humvee", family_name: "HMMWV" };
  const { view } = openShop([
    card("dear", "Dear", { cost: 600 }),
    card("jeep_hmg", "HMG", { ...jeep, cost: 120 }),
    card("cheap", "Cheap", { cost: 200 }),
    card("jeep", "Base", { ...jeep, cost: 80 }),
  ]);
  expect(shownNames(view)).toEqual([
    "M1 Abrams Cheap",
    "M1 Abrams Dear",
    "HMMWV Base",
    "HMMWV HMG",
  ]);
});

test("a card is chosen only while it can be bought: on sale, affordable, under the cap, battle on", () => {
  const unsold = card("unsold", "Prototype", { reason: "Protection deferred" });
  const cases: [UnitCard, Partial<SkirmishView["players"][0]>, SkirmishView["phase"]?][] = [
    [BASE, { credits: 199 }],
    [BASE, { unitCount: 30 }],
    [BASE, {}, "finished"],
    [unsold, {}],
  ];
  for (const [c, player, phase] of cases) {
    const choose = vi.fn();
    const view = render(
      <PurchasePicker
        cards={[c]}
        faction="us"
        match={{
          ...match,
          phase: phase ?? match.phase,
          players: [{ ...match.players[0], ...player }],
        }}
        onChoose={choose}
        rules={RULES}
      />,
    );
    fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
    fireEvent.click(view.getByRole("tab", { name: tab("mbt") }));
    const shown = view.getByRole("button", { name: c.name });
    // It still shows its info card, to say what it is.
    fireEvent.mouseEnter(shown);
    expect(view.getByRole("tooltip", { name: c.name })).toBeTruthy();
    fireEvent.click(shown);
    expect(choose, JSON.stringify({ id: c.id, player, phase })).not.toHaveBeenCalled();
    cleanup();
  }
  // Exactly enough credits and one slot left buys it.
  const { view, choose } = openShop([BASE], { credits: 200, unitCount: 29 });
  fireEvent.click(view.getByRole("button", { name: BASE.name }));
  expect(choose).toHaveBeenCalledExactlyOnceWith("base");
  expect(view.queryByLabelText("Unit slots")).toBeNull();
});

test("the picker opens fresh, however it was last closed", () => {
  const { view } = openShop([BASE]);
  const toggle = () => fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  const expectFresh = () => {
    expect(view.getByRole("tab", { name: tab("rec") }).getAttribute("aria-selected")).toBe("true");
    expect(view.queryByRole("tooltip")).toBeNull();
  };
  // Closed by buying.
  fireEvent.click(view.getByRole("button", { name: BASE.name }));
  toggle();
  expectFresh();
  // Closed by its button, with an info card open.
  fireEvent.click(view.getByRole("tab", { name: tab("mbt") }));
  fireEvent.mouseEnter(view.getByRole("button", { name: BASE.name }));
  expect(view.getByRole("tooltip")).toBeTruthy();
  toggle();
  toggle();
  expectFresh();
});

test("resting the pointer on a group tab switches to it, without a click", () => {
  const view = render(
    <PurchasePicker cards={[BASE]} faction="us" match={match} onChoose={vi.fn()} rules={RULES} />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  expect(view.queryByRole("button", { name: BASE.name })).toBeNull();
  fireEvent.mouseEnter(view.getByRole("tab", { name: tab("mbt") }));
  expect(view.getByRole("tab", { name: tab("mbt") }).getAttribute("aria-selected")).toBe("true");
  fireEvent.mouseEnter(view.getByRole("button", { name: BASE.name }));
  expect(view.getByRole("tooltip", { name: BASE.name })).toBeTruthy();
  // Passing back over the shown tab keeps the open info card.
  fireEvent.mouseEnter(view.getByRole("tab", { name: tab("mbt") }));
  expect(view.getByRole("tooltip", { name: BASE.name })).toBeTruthy();
});

test("typing a family or variant name narrows the grid to the cards it names", () => {
  const jeep = card("jeep", "Base", { family: "humvee", family_name: "HMMWV", cost: 80 });
  const { view } = openShop([BASE, TROPHY, jeep]);
  const field = view.getByRole("textbox", { name: "Search units" });
  fireEvent.change(field, { target: { value: "hmmwv" } });
  expect(shownNames(view)).toEqual([jeep.name]);
  // A family name finds every variant; a variant's, only the variants it names.
  fireEvent.change(field, { target: { value: "abrams" } });
  expect(shownNames(view)).toEqual([BASE.name, TROPHY.name]);
  fireEvent.change(field, { target: { value: "trophy" } });
  expect(shownNames(view)).toEqual([TROPHY.name]);
});

test("a search spans every group and dims the tabs", () => {
  // A tank, a jeep of the same category but another group, and a squad.
  const cards = [
    card("test_tank", "Base", { family: "tank", family_name: "Tank" }),
    card("test_jeep", "Base", { family: "jeep", family_name: "Jeep" }),
    card("test_rifle", "Base", { family: "rifle", family_name: "Rifle Section", category: "inf" }),
  ];
  const { view } = openShop(cards);
  const names = cards.map((c) => c.name);
  // The test catalog's types place each card under its own group.
  expect(shownNames(view)).toEqual([names[0]]);
  const field = view.getByRole("textbox", { name: "Search units" });
  fireEvent.change(field, { target: { value: "base" } });
  expect(shownNames(view)).toEqual(names);
  expect(view.getByRole("tab", { name: tab("mbt") }).hasAttribute("disabled")).toBe(true);
  // Clearing it browses the shown group again.
  fireEvent.change(field, { target: { value: "" } });
  expect(shownNames(view)).toEqual([names[0]]);
  fireEvent.click(view.getByRole("tab", { name: tab("ifv_apc") }));
  expect(shownNames(view)).toEqual([names[1]]);
});

test("an unmatched search says so, and clearing restores the shown group", () => {
  const rifle = card("rifle", "Base", { family: "rifle", family_name: "Rifle", category: "inf" });
  const { view } = openShop([rifle, BASE]);
  const field = view.getByRole("textbox", { name: "Search units" });
  fireEvent.change(field, { target: { value: "zzz" } });
  expect(view.getByText("NO MATCH")).toBeTruthy();
  expect(shownNames(view)).toEqual([]);
  fireEvent.change(field, { target: { value: "" } });
  expect(view.queryByText("NO MATCH")).toBeNull();
  expect(shownNames(view)).toEqual([BASE.name]);
  expect(view.getByRole("tab", { name: tab("mbt") }).getAttribute("aria-selected")).toBe("true");
});

test("Enter buys the one card a search matches, and only then", () => {
  const unsold = card("unsold", "Prototype", { reason: "Protection deferred" });
  const { view, choose } = openShop([BASE, TROPHY, unsold]);
  const field = view.getByRole("textbox", { name: "Search units" });
  const enter = (query: string) => {
    fireEvent.change(field, { target: { value: query } });
    fireEvent.keyDown(field, { key: "Enter" });
  };
  // Two siblings match: no hidden first choice.
  enter("abrams sep");
  // The one match is never sold; then nothing matches.
  enter("prototype");
  enter("zzz");
  expect(choose).not.toHaveBeenCalled();
  enter("trophy");
  expect(choose).toHaveBeenCalledExactlyOnceWith("trophy");
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

/** A Tab press, as the browser sends it, at `target`; true if not consumed. */
const tabAt = (target: Element, shiftKey = false) =>
  fireEvent.keyDown(target, { key: "Tab", code: "Tab", shiftKey, cancelable: true });

test("Tab opens the shop, Tab again types into its search, a third Tab leaves the field", () => {
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
  // The tooltip carries the key, as the menu button's does; the name stays.
  expect(toggle.getAttribute("aria-keyshortcuts")).toBe("Tab");
  expect(toggle.textContent?.trim()).toBe("Reinforcements");
  expect(toggle.getAttribute("title")).toBe("Reinforcements (Tab)");
  // Opening leaves game keys to the game: the field waits.
  expect(tabAt(document.body)).toBe(false);
  const field = view.getByRole("textbox", { name: "Search units" });
  expect(document.activeElement).not.toBe(field);
  expect(tabAt(document.body)).toBe(false);
  expect(document.activeElement).toBe(field);
  // In the field every letter types, V included: no letter is the shop's.
  expect(fireEvent.keyDown(field, { key: "v", code: "KeyV", cancelable: true })).toBe(true);
  expect(tabAt(field)).toBe(false);
  expect(document.activeElement).not.toBe(field);
  expect(view.getByRole("textbox", { name: "Search units" })).toBeTruthy();
  // The cycle goes on while the shop is open.
  tabAt(document.body);
  expect(document.activeElement).toBe(field);
  // Shift+Tab is the same key: it never walks focus onto a button.
  expect(tabAt(field, true)).toBe(false);
  expect(document.activeElement).not.toBe(field);
  // The button is the close.
  fireEvent.click(toggle);
  expect(view.queryByRole("textbox", { name: "Search units" })).toBeNull();
});

test("Escape outside the field closes the shop ahead of the pause menu", () => {
  const view = render(
    <PurchasePicker
      cards={[card("base", "SEP v2")]}
      faction="us"
      match={match}
      onChoose={vi.fn()}
      rules={RULES}
    />,
  );
  tabAt(document.body);
  expect(fireEvent.keyDown(document.body, { key: "Escape", cancelable: true })).toBe(false);
  expect(view.queryByRole("textbox", { name: "Search units" })).toBeNull();
});

test("the shop's keys do nothing while the battle is covered", () => {
  const props = {
    cards: [card("base", "SEP v2")],
    faction: "us" as const,
    match,
    onChoose: vi.fn(),
    rules: RULES,
  };
  const view = render(<PurchasePicker {...props} />);
  tabAt(document.body);
  expect(view.getByRole("textbox", { name: "Search units" })).toBeTruthy();
  // A menu covers the open shop: Tab and Escape are the menu's.
  view.rerender(<PurchasePicker {...props} enabled={false} />);
  expect(tabAt(document.body)).toBe(true);
  expect(fireEvent.keyDown(document.body, { key: "Escape", cancelable: true })).toBe(true);
  expect(view.getByRole("textbox", { name: "Search units" })).toBeTruthy();
  // Closed and covered, Tab opens nothing.
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  expect(tabAt(document.body)).toBe(true);
  expect(view.queryByRole("textbox", { name: "Search units" })).toBeNull();
});
