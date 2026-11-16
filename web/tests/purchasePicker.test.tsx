import { shippedRules } from "@web/shippedRules";
import { UNIT_GROUPS } from "@web/battle/present/purchaseFilter";
import { act, cleanup, fireEvent, render as renderView, within } from "@testing-library/react";
import { UNITS, WithTestCatalog } from "./catalog";
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
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});
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
  // Straight onto its sibling: no wait.
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

// A squad's ride. Squads and carriers are test catalog types: a card is a
// squad by its soldier slots and a carrier by its seats. Prices are the
// cards' own; the APC is cheaper than the scout car though listed after it,
// and the helicopter is listed before both though its tab comes after.
const squad = (id: string, family_name: string, cost: number) =>
  card(id, "Base", { family: id, family_name, category: "inf", cost });
const ride = (id: string, family_name: string, cost: number, category: UnitCategory = "veh") =>
  card(id, "Base", { family: id, family_name, category, cost });
const RIFLES = squad("test_rifle", "Rifles", 100);
const TEAM = squad("test_at", "AT Team", 150);
const HELI = ride("test_transport_heli", "Lift Heli", 300, "hel");
const SCOUT = ride("test_scout_car", "Scout Car", 90);
const APC = ride("test_apc", "APC", 60);
const TANK = ride("test_tank", "Tank", 200);
/** Never a carrier: a planned card has no type, so no seats. */
const PLANNED = ride("test_planned_ride", "Planned Ride", 40);
const RIDES = [RIFLES, TEAM, HELI, SCOUT, APC, TANK, PLANNED];

test("the test carriers seat the squads these tests say they do", () => {
  // The physical facts the menu's tests rest on, so a change to the test
  // units says which test it moved.
  const seats = (c: UnitCard) => UNITS.seats(c.id);
  const size = (c: UnitCard) => UNITS.squadSize(c.id);
  expect(seats(SCOUT)).toBeLessThan(size(RIFLES));
  expect(seats(SCOUT)).toBeGreaterThanOrEqual(size(TEAM));
  expect(Math.min(seats(APC), seats(HELI))).toBeGreaterThanOrEqual(size(RIFLES));
  expect(seats(TANK)).toBe(0);
  expect(UNITS.has(PLANNED.id)).toBe(false);
});

/** The shop opened on `group`'s tab over the transport cards. */
function openOn(group: string, player: Partial<SkirmishView["players"][0]> = {}) {
  const shop = openShop(RIDES, player);
  fireEvent.click(shop.view.getByRole("tab", { name: tab(group) }));
  return shop;
}

/** The open transport menu's sections, each its label and its cards'
 *  names in order; null with no menu open. */
const transportMenu = (view: ReturnType<typeof render>) => {
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

/** The info cards on show, top to bottom. */
const infoCards = (view: ReturnType<typeof render>) =>
  view.queryAllByRole("tooltip").map((t) => t.getAttribute("aria-label"));

/** The menu's highlighted options. */
const highlighted = (view: ReturnType<typeof render>) =>
  within(view.getByRole("region", { name: "Transport" }))
    .queryAllByRole("button", { pressed: true })
    .map((b) => b.getAttribute("aria-label"));

test("hovering a squad opens its transport menu: on foot, then each tab's carriers that seat it, cheapest first", () => {
  vi.useFakeTimers();
  const { view } = openOn("inf");
  expect(transportMenu(view)).toBeNull();
  fireEvent.mouseEnter(view.getByRole("button", { name: TEAM.name }));
  expect(transportMenu(view)).toEqual([
    ["On foot", ["On foot"]],
    [tab("ifv_apc"), [APC.name, SCOUT.name]],
    [tab("hel"), [HELI.name]],
  ]);
  // The base card previews the same purchase as the On foot option.
  expect(highlighted(view)).toEqual(["On foot"]);
  expect(infoCards(view)).toEqual([TEAM.name]);
  // A bigger squad, once the pointer rests on it: the scout car can't seat it.
  fireEvent.mouseEnter(view.getByRole("button", { name: RIFLES.name }));
  act(() => vi.advanceTimersByTime(1000));
  expect(transportMenu(view)).toEqual([
    ["On foot", ["On foot"]],
    [tab("ifv_apc"), [APC.name]],
    [tab("hel"), [HELI.name]],
  ]);
  // A ride's price reads as what it adds.
  expect(view.getByRole("button", { name: APC.name }).textContent).toContain(`+${APC.cost} CR`);
});

test("a vehicle's card shows its info without a transport menu", () => {
  const { view } = openOn("ifv_apc");
  fireEvent.mouseEnter(view.getByRole("button", { name: APC.name }));
  expect(transportMenu(view)).toBeNull();
  expect(infoCards(view)).toEqual([APC.name]);
});

test("pointing at an option shows what it buys: on foot the squad, a ride its card with the squad aboard and the total", () => {
  const { view } = openOn("inf");
  fireEvent.mouseEnter(view.getByRole("button", { name: TEAM.name }));
  fireEvent.mouseEnter(view.getByRole("button", { name: "On foot" }));
  expect(infoCards(view)).toEqual([TEAM.name]);
  expect(highlighted(view)).toEqual(["On foot"]);
  fireEvent.mouseEnter(view.getByRole("button", { name: SCOUT.name }));
  expect(infoCards(view)).toEqual([SCOUT.name, TEAM.name]);
  expect(highlighted(view)).toEqual([SCOUT.name]);
  // The squad rides in the carrier: its seats count the squad's soldiers.
  const [carrier, aboard] = view.getAllByRole("tooltip");
  const seated = `${UNITS.squadSize(TEAM.id)}/${UNITS.seats(SCOUT.id)}`;
  expect(carrier.textContent).toContain(seated);
  expect(view.getByLabelText("Total").textContent).toContain(`${TEAM.cost + SCOUT.cost} CR`);
  // The hovered ride is described by everything it buys.
  const described = view.getByRole("button", { name: SCOUT.name }).getAttribute("aria-describedby");
  expect(described?.split(" ")).toContain(aboard.id);
  // Leaving and coming back resets the menu to the squad on foot.
  fireEvent.mouseLeave(view.getByRole("region", { name: "Faction units" }));
  fireEvent.mouseEnter(view.getByRole("button", { name: TEAM.name }));
  expect(highlighted(view)).toEqual(["On foot"]);
  expect(infoCards(view)).toEqual([TEAM.name]);
});

test("clicking a ride buys the squad in it; on foot or the squad's own card buys it alone", () => {
  for (const [target, expected] of [
    [SCOUT.name, ["test_at", "test_scout_car"]],
    ["On foot", ["test_at"]],
    [TEAM.name, ["test_at"]],
  ] as const) {
    const { view, choose } = openOn("inf");
    fireEvent.mouseEnter(view.getByRole("button", { name: TEAM.name }));
    // A previously inspected ride never changes what the base card buys.
    fireEvent.mouseEnter(view.getByRole("button", { name: SCOUT.name }));
    fireEvent.click(view.getByRole("button", { name: target }));
    expect(choose, target).toHaveBeenCalledExactlyOnceWith(...expected);
    cleanup();
  }
});

test("returning from a ride to its base card previews on foot again", () => {
  const { view } = openOn("inf");
  const base = view.getByRole("button", { name: TEAM.name });
  fireEvent.mouseEnter(base);
  fireEvent.mouseEnter(view.getByRole("button", { name: SCOUT.name }));
  expect(infoCards(view)).toEqual([SCOUT.name, TEAM.name]);
  fireEvent.mouseEnter(base);
  expect(highlighted(view)).toEqual(["On foot"]);
  expect(infoCards(view)).toEqual([TEAM.name]);
});

test("a ride is refused when the credits or the unit cap can't take the pair, while on foot still buys", () => {
  const pair = TEAM.cost + SCOUT.cost;
  for (const player of [
    { credits: pair - 1 },
    { unitCount: 29 },
    { credits: pair, unitCount: 28 },
  ]) {
    const { view, choose } = openOn("inf", player);
    fireEvent.mouseEnter(view.getByRole("button", { name: TEAM.name }));
    const scout = view.getByRole("button", { name: SCOUT.name });
    const refused = player.unitCount !== 28;
    expect(scout.getAttribute("aria-disabled"), JSON.stringify(player)).toBe(String(refused));
    fireEvent.click(scout);
    if (refused) {
      expect(choose).not.toHaveBeenCalled();
      // The player can still read what it would have bought.
      expect(infoCards(view)).toEqual([SCOUT.name, TEAM.name]);
      fireEvent.click(view.getByRole("button", { name: "On foot" }));
      expect(choose).toHaveBeenCalledExactlyOnceWith("test_at");
    } else expect(choose).toHaveBeenCalledExactlyOnceWith("test_at", "test_scout_car");
    cleanup();
  }
});

test("an open menu holds while the pointer heads into it across other cards; turning away or stopping switches", () => {
  vi.useFakeTimers();
  // The menu stands over the card row: 400 px wide, its foot at y = 100.
  const box = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect");
  box.mockImplementation(function (this: HTMLElement) {
    const menu = this.getAttribute("aria-label") === "Transport";
    return DOMRect.fromRect(menu ? { x: 0, y: 0, width: 400, height: 100 } : {});
  });
  const { view } = openOn("inf");
  const picker = view.getByRole("region", { name: "Faction units" });
  const moves = (...path: [number, number][]) =>
    path.forEach(([clientX, clientY]) => fireEvent.mouseMove(picker, { clientX, clientY }));
  const scoutListed = () => JSON.stringify(transportMenu(view)).includes(SCOUT.name);
  fireEvent.mouseEnter(view.getByRole("button", { name: RIFLES.name }));
  expect(scoutListed()).toBe(false);
  // Up and right across the team's card, into the rifles' menu.
  moves([30, 150], [60, 140], [90, 130]);
  fireEvent.mouseEnter(view.getByRole("button", { name: TEAM.name }));
  moves([100, 126]);
  expect(scoutListed()).toBe(false);
  fireEvent.mouseEnter(view.getByRole("button", { name: APC.name }));
  act(() => vi.advanceTimersByTime(1000));
  expect(scoutListed()).toBe(false);
  expect(highlighted(view)).toEqual([APC.name]);
  // Along the row, the team's card takes over at once, previewing on foot.
  moves([100, 150], [130, 150], [160, 150]);
  fireEvent.mouseEnter(view.getByRole("button", { name: TEAM.name }));
  moves([170, 150]);
  expect(scoutListed()).toBe(true);
  expect(highlighted(view)).toEqual(["On foot"]);
  // Heading in, then turning away on the rifles' card: it takes over.
  moves([200, 200], [210, 185], [220, 170]);
  fireEvent.mouseEnter(view.getByRole("button", { name: RIFLES.name }));
  moves([225, 162]);
  expect(scoutListed()).toBe(true);
  moves([225, 190]);
  expect(scoutListed()).toBe(false);
  // Heading in and stopping on the team's card: it takes over once the wait ends.
  moves([200, 200], [210, 185], [220, 170]);
  fireEvent.mouseEnter(view.getByRole("button", { name: TEAM.name }));
  expect(scoutListed()).toBe(false);
  act(() => vi.advanceTimersByTime(1000));
  expect(scoutListed()).toBe(true);
});

test("a search down to one squad opens its menu; Up and Down step through it, Enter buys", () => {
  const camera = vi.fn();
  window.addEventListener("keydown", camera);
  try {
    const search = (query: string) => {
      const shop = openShop(RIDES);
      const field = shop.view.getByRole("textbox", { name: "Search units" });
      fireEvent.change(field, { target: { value: query } });
      const key = (key: string) => fireEvent.keyDown(field, { key, cancelable: true });
      return { ...shop, key };
    };
    // Enter with nothing highlighted buys on foot.
    let shop = search("at team");
    expect(transportMenu(shop.view)?.[0]).toEqual(["On foot", ["On foot"]]);
    expect(highlighted(shop.view)).toEqual([]);
    shop.key("Enter");
    expect(shop.choose).toHaveBeenCalledExactlyOnceWith("test_at");
    cleanup();
    // Down steps on foot, then each section cheapest first; Up steps back.
    shop = search("at team");
    camera.mockClear();
    const steps = ["ArrowDown", "ArrowDown", "ArrowDown", "ArrowUp"].map((k) => {
      // The arrows are the menu's: the camera never sees them.
      expect(shop.key(k)).toBe(false);
      return [highlighted(shop.view), infoCards(shop.view)];
    });
    expect(steps).toEqual([
      [["On foot"], [TEAM.name]],
      [[APC.name], [APC.name, TEAM.name]],
      [[SCOUT.name], [SCOUT.name, TEAM.name]],
      [[APC.name], [APC.name, TEAM.name]],
    ]);
    expect(camera).not.toHaveBeenCalled();
    shop.key("Enter");
    expect(shop.choose).toHaveBeenCalledExactlyOnceWith("test_at", "test_apc");
    cleanup();
    // With no menu open, the arrows stay the camera's.
    shop = search("tank");
    expect(shop.key("ArrowUp")).toBe(true);
    expect(camera).toHaveBeenCalled();
  } finally {
    window.removeEventListener("keydown", camera);
  }
});

test("a unit card's lookup opens on that exact variant, with its info card", () => {
  const choose = vi.fn();
  const view = render(
    <PurchasePicker
      cards={[BASE, TROPHY]}
      faction="us"
      match={match}
      onChoose={choose}
      rules={RULES}
      lookup={{ card: "base", serial: 1 }}
    />,
  );
  const field = view.getByRole("textbox", { name: "Search units" }) as HTMLInputElement;
  expect(field.value).toBe("M1 Abrams SEP v2");
  // The shop leaves the keyboard to the game; the open card is the next click.
  expect(document.activeElement).not.toBe(field);
  expect(view.getByRole("button", { name: "M1 Abrams SEP v2" })).toBeTruthy();
  expect(view.queryByRole("button", { name: "M1 Abrams SEP v2 Trophy" })).toBeNull();
  expect(view.getByRole("tooltip", { name: "M1 Abrams SEP v2" })).toBeTruthy();
  fireEvent.click(view.getByRole("button", { name: "M1 Abrams SEP v2" }));
  expect(choose).toHaveBeenCalledExactlyOnceWith("base");
});

test("a lookup is one shot: closing it and opening the shop starts fresh", () => {
  const view = render(
    <PurchasePicker
      cards={[BASE]}
      faction="us"
      match={match}
      onChoose={vi.fn()}
      rules={RULES}
      lookup={{ card: "base", serial: 1 }}
    />,
  );
  const toggle = view.getByRole("button", { name: "Reinforcements" });
  fireEvent.click(toggle);
  expect(view.queryByRole("textbox", { name: "Search units" })).toBeNull();
  fireEvent.click(toggle);
  expect(view.getByRole("tab", { name: tab("rec") }).getAttribute("aria-selected")).toBe("true");
  expect((view.getByRole("textbox", { name: "Search units" }) as HTMLInputElement).value).toBe("");
});

test("a second lookup moves the open store to the new exact card", () => {
  const view = render(
    <PurchasePicker
      cards={[BASE, TROPHY]}
      faction="us"
      match={match}
      onChoose={vi.fn()}
      rules={RULES}
      lookup={{ card: "base", serial: 1 }}
    />,
  );
  const field = () => view.getByRole("textbox", { name: "Search units" }) as HTMLInputElement;
  expect(field().value).toBe("M1 Abrams SEP v2");
  view.rerender(
    <PurchasePicker
      cards={[BASE, TROPHY]}
      faction="us"
      match={match}
      onChoose={vi.fn()}
      rules={RULES}
      lookup={{ card: "trophy", serial: 2 }}
    />,
  );
  expect(field().value).toBe("M1 Abrams SEP v2 Trophy");
  expect(view.getByRole("tooltip", { name: "M1 Abrams SEP v2 Trophy" })).toBeTruthy();
});

test("a taken lookup is cleared, so a remount cannot replay it", () => {
  const props = {
    cards: [BASE],
    faction: "us" as const,
    match,
    onChoose: vi.fn(),
    rules: RULES,
  };
  const handled = vi.fn();
  const view = render(
    <PurchasePicker {...props} lookup={{ card: "base", serial: 1 }} onLookupHandled={handled} />,
  );
  expect(handled).toHaveBeenCalledOnce();
  expect(view.getByRole("textbox", { name: "Search units" })).toBeTruthy();
  view.unmount();
  const remounted = render(<PurchasePicker {...props} lookup={null} />);
  expect(remounted.queryByRole("textbox", { name: "Search units" })).toBeNull();
});

test("a squad lookup opens its transport menu on the exact squad", () => {
  const view = render(
    <PurchasePicker
      cards={[RIFLES, APC]}
      faction="us"
      match={match}
      onChoose={vi.fn()}
      rules={RULES}
      lookup={{ card: "test_rifle", serial: 1 }}
    />,
  );
  expect(view.getByRole("button", { name: "Rifles Base" })).toBeTruthy();
  expect(view.getByRole("region", { name: "Transport" })).toBeTruthy();
  expect(view.getByRole("group", { name: "On foot" })).toBeTruthy();
  expect(infoCards(view)).toEqual([RIFLES.name]);
});
