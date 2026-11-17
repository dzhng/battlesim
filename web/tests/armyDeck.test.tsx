import { shippedRules } from "@web/shippedRules";
import { act, cleanup, fireEvent, render as renderView } from "@testing-library/react";
import { Feed, useFeed } from "@web/feed";
import { WithTestCatalog } from "./catalog";
import { TEST_CATALOG } from "./catalog";
import { afterAll, afterEach, beforeAll, expect, test, vi } from "vitest";
import { useLayoutEffect, useState } from "react";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { ownPanel, reserveRow, type PanelRules } from "../src/battle/present/panelRows";
import { iconSvg } from "../src/battle/present/icons";
import type { CommandControl } from "../src/battle/present/commandBar";
import { ArmyDeck } from "../src/battle/present/armyDeck";
import type { CardBlink } from "../src/battle/present/fireBlink";

/** Views render under the test set, as a lab page provides it. */
const render = (ui: Parameters<typeof renderView>[0]) =>
  renderView(ui, { wrapper: WithTestCatalog });

beforeAll(() => {
  HTMLElement.prototype.scrollIntoView = vi.fn();
});
afterAll(() => {
  delete (HTMLElement.prototype as Partial<HTMLElement>).scrollIntoView;
});
afterEach(cleanup);

/** What a card says on its face, whitespace collapsed. */
const words = (node: HTMLElement) => (node.textContent ?? "").replace(/\s+/g, " ").trim();

test("army observation and selection read the same publication before paint", () => {
  const beforePaint: number[][] = [];
  function Harness() {
    const [own, setOwn] = useState([specimenUnit(TEST_CATALOG, "test_tank", { id: 11 })]);
    const state = useFeed({ own });
    useLayoutEffect(() => {
      beforePaint.push(state.current.own.map((unit) => unit.id));
    }, [state, own]);
    return (
      <>
        <button onClick={() => setOwn([specimenUnit(TEST_CATALOG, "test_rifle", { id: 22 })])}>
          Replace army
        </button>
        <ArmyDeck
          state={state}
          selected={own.map((unit) => unit.id)}
          onSelect={vi.fn()}
          rules={shippedRules as unknown as PanelRules}
          contactLines={null}
        />
      </>
    );
  }
  const view = render(<Harness />);
  fireEvent.click(view.getByRole("button", { name: "Replace army" }));
  expect(beforePaint).toEqual([[11], [22]]);
  expect(view.getByRole("button", { name: "Rifle Section #22" }).getAttribute("aria-pressed")).toBe(
    "true",
  );
});

test("live army publications refresh cards and command availability without replacing the feed", () => {
  const tank = specimenUnit(TEST_CATALOG, "test_tank", { id: 11 });
  const state = new Feed({
    own: [tank],
    control: {
      selectedUnits: [tank],
      mode: "move" as const,
      setMode: vi.fn(),
      stop: vi.fn(),
      togglePolicy: vi.fn(),
      toggleDeployment: vi.fn(),
      exitBuilding: vi.fn(),
      pressUnload: vi.fn(),
      popSmoke: vi.fn(),
    },
  });
  const view = render(
    <ArmyDeck
      state={state}
      selected={[11]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
    />,
  );
  expect(view.getByRole("button", { name: "Tank #11" })).toBeTruthy();
  expect(view.getByRole("button", { name: /^Stop/ }).hasAttribute("disabled")).toBe(false);
  const rifle = specimenUnit(TEST_CATALOG, "test_rifle", { id: 22 });
  act(() => state.set({ own: [rifle], control: { ...state.current.control, selectedUnits: [] } }));
  expect(view.queryByRole("button", { name: "Tank #11" })).toBeNull();
  expect(view.getByRole("button", { name: "Rifle Section #22" })).toBeTruthy();
  expect(view.getByRole("button", { name: /^Stop/ }).hasAttribute("disabled")).toBe(true);
  act(() =>
    state.set({ own: [rifle], control: { ...state.current.control, selectedUnits: [rifle] } }),
  );
  fireEvent.click(view.getByRole("button", { name: /^Stop/ }));
  expect(state.current.control.stop).toHaveBeenCalledOnce();
});

test("the army row includes unselected own units and stays visible without a selection", () => {
  const own = [
    specimenUnit(TEST_CATALOG, "test_tank", { id: 11 }),
    specimenUnit(TEST_CATALOG, "test_rifle", { id: 22 }),
  ];
  const view = render(
    <ArmyDeck
      state={new Feed({ own })}
      selected={[]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
    />,
  );
  expect(view.getByRole("button", { name: "Tank #11" }).getAttribute("aria-pressed")).toBe("false");
  expect(view.getByRole("button", { name: "Rifle Section #22" }).getAttribute("aria-pressed")).toBe(
    "false",
  );
  expect(view.queryByRole("toolbar", { name: "Commands" })).toBeNull();
  expect(view.queryByRole("tooltip")).toBeNull();
});

test("a click selects one card, Shift-click the run from the last clicked card, Cmd or Ctrl one more", () => {
  const apc = specimenUnit(TEST_CATALOG, "test_apc", { id: 4 });
  const rider = {
    ...specimenUnit(TEST_CATALOG, "test_rifle", { id: 7 }),
    hold: { host: { kind: "carrier" as const, id: 4 }, phase: "inside", progress: 1 },
  };
  const own = [
    specimenUnit(TEST_CATALOG, "test_tank", { id: 11 }),
    specimenUnit(TEST_CATALOG, "test_rifle", { id: 22 }),
    rider,
    specimenUnit(TEST_CATALOG, "test_supply", { id: 33 }),
    apc,
  ];
  function Harness() {
    const [selected, setSelected] = useState<number[]>([]);
    return (
      <ArmyDeck
        state={new Feed({ own })}
        selected={selected}
        onSelect={setSelected}
        rules={shippedRules as unknown as PanelRules}
        contactLines={null}
      />
    );
  }
  const view = render(<Harness />);
  const card = (name: RegExp) => view.getByRole("button", { name });
  const pressed = () =>
    view
      .getAllByRole("button", { pressed: true })
      .map((b) => b.getAttribute("aria-label")!.split(",")[0]);
  fireEvent.click(card(/ #22$/));
  expect(pressed()).toEqual(["Rifle Section #22"]);
  // The run replaces the selection; a rider in it picks its carrier.
  fireEvent.click(card(/ #33$/), { shiftKey: true });
  expect(pressed()).toEqual([
    "Rifle Section #22",
    "Rifle Section #7",
    "Supply Truck #33",
    "Test APC #4",
  ]);
  // A run backwards from the same anchor.
  fireEvent.click(card(/ #11$/), { shiftKey: true });
  expect(pressed()).toEqual(["Tank #11", "Rifle Section #22"]);
  fireEvent.click(card(/ #33$/), { metaKey: true });
  expect(pressed()).toEqual(["Tank #11", "Rifle Section #22", "Supply Truck #33"]);
  fireEvent.click(card(/ #22$/), { ctrlKey: true });
  expect(pressed()).toEqual(["Tank #11", "Supply Truck #33"]);
  // The toggled card is the new anchor.
  fireEvent.click(card(/^Test APC #4$/), { shiftKey: true });
  expect(pressed()).toEqual([
    "Rifle Section #22",
    "Rifle Section #7",
    "Supply Truck #33",
    "Test APC #4",
  ]);
});

test("Shift-click after the selection changed elsewhere starts a new run at the clicked card", () => {
  const own = [
    specimenUnit(TEST_CATALOG, "test_tank", { id: 11 }),
    specimenUnit(TEST_CATALOG, "test_rifle", { id: 22 }),
    specimenUnit(TEST_CATALOG, "test_supply", { id: 33 }),
  ];
  function Harness() {
    const [selected, setSelected] = useState<number[]>([]);
    return (
      <>
        <button onClick={() => setSelected([])}>Clear</button>
        <ArmyDeck
          state={new Feed({ own })}
          selected={selected}
          onSelect={setSelected}
          rules={shippedRules as unknown as PanelRules}
          contactLines={null}
        />
      </>
    );
  }
  const view = render(<Harness />);
  fireEvent.click(view.getByRole("button", { name: "Tank #11" }));
  fireEvent.click(view.getByRole("button", { name: "Clear" }));
  fireEvent.click(view.getByRole("button", { name: "Supply Truck #33" }), { shiftKey: true });
  expect(
    view.getAllByRole("button", { pressed: true }).map((b) => b.getAttribute("aria-label")),
  ).toEqual(["Supply Truck #33"]);
});

test("a selected card asks the camera to centre while an unselected card only selects", () => {
  const tank = specimenUnit(TEST_CATALOG, "test_tank", { id: 11 });
  const rifle = specimenUnit(TEST_CATALOG, "test_rifle", { id: 22 });
  const onCenter = vi.fn();
  function Harness() {
    const [selected, setSelected] = useState([11]);
    return (
      <ArmyDeck
        state={new Feed({ own: [tank, rifle] })}
        selected={selected}
        onSelect={setSelected}
        onCenter={onCenter}
        rules={shippedRules as unknown as PanelRules}
        contactLines={null}
      />
    );
  }
  const view = render(<Harness />);
  const tankCard = view.getByRole("button", { name: "Tank #11" });
  const rifleCard = view.getByRole("button", { name: "Rifle Section #22" });
  fireEvent.click(tankCard);
  expect(onCenter).toHaveBeenCalledOnce();
  expect(onCenter).toHaveBeenCalledWith(tank);
  onCenter.mockClear();
  fireEvent.click(rifleCard);
  expect(rifleCard.getAttribute("aria-pressed")).toBe("true");
  expect(onCenter).not.toHaveBeenCalled();
  fireEvent.click(rifleCard);
  expect(onCenter).toHaveBeenCalledOnce();
  expect(onCenter).toHaveBeenCalledWith(rifle);
  onCenter.mockClear();
  fireEvent.click(rifleCard, { metaKey: true });
  expect(rifleCard.getAttribute("aria-pressed")).toBe("false");
  expect(onCenter).not.toHaveBeenCalled();
});

test("hover exposes live unit facts, health and casualty removal", () => {
  const tank = specimenUnit(TEST_CATALOG, "test_tank", { id: 11 });
  const rifle = specimenUnit(TEST_CATALOG, "test_rifle", {
    id: 22,
    suppression: "suppressed",
    service: "serving",
  });
  const props = {
    selected: [],
    onSelect: vi.fn(),
    rules: shippedRules as unknown as PanelRules,
    contactLines: null,
  };
  const view = render(<ArmyDeck state={new Feed({ own: [tank, rifle] })} {...props} />);
  const card = view.getByRole("button", { name: "Rifle Section #22" });
  fireEvent.mouseEnter(card);
  const hint = view.getByRole("tooltip");
  expect(hint.textContent).toContain("Rifle Section #22");
  expect(hint.textContent).toContain("SUPPRESSED");
  expect(card.getAttribute("aria-describedby")).toBe(hint.id);
  fireEvent.mouseLeave(card);
  expect(view.queryByRole("tooltip")).toBeNull();
  fireEvent.mouseEnter(view.getByRole("button", { name: "Tank #11" }));
  expect(view.getByRole("tooltip").textContent).toContain("MAIN GUN");
  expect(view.getByRole("tooltip").textContent).toContain("HMG");
  fireEvent.mouseEnter(card);
  view.rerender(
    <ArmyDeck
      state={new Feed({ own: [tank, { ...rifle, memberHp: rifle.memberHp.map((hp) => hp / 2) }] })}
      {...props}
    />,
  );
  expect(
    view.getByRole("meter", { name: "Rifle Section #22 health" }).getAttribute("aria-valuenow"),
  ).toBe("50");
  view.rerender(<ArmyDeck state={new Feed({ own: [tank] })} {...props} />);
  expect(view.queryByRole("button", { name: "Rifle Section #22" })).toBeNull();
  expect(view.queryByRole("tooltip")).toBeNull();
});

test("hover detail anchors to the deck rather than the lower HUD stack", () => {
  const view = render(
    <ArmyDeck
      state={new Feed({ own: [specimenUnit(TEST_CATALOG, "test_tank", { id: 11 })] })}
      selected={[]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      contactLines={<ul className="hud-contact-lines" />}
    />,
  );
  const deck = view.getByRole("group", { name: "Your units" }).closest("footer")!;
  vi.spyOn(deck, "getBoundingClientRect").mockReturnValue(new DOMRect(0, 500, 400, 80));
  fireEvent.mouseEnter(view.getByRole("button", { name: "Tank #11" }));
  expect(view.getByRole("tooltip").style.top).toBe("500px");
});

test("the army remains in replay and empty selection while commands follow only selected units", () => {
  const own = [
    specimenUnit(TEST_CATALOG, "test_tank", { id: 11 }),
    specimenUnit(TEST_CATALOG, "test_supply", { id: 22 }),
  ];
  function Harness({ replay = false }: { replay?: boolean }) {
    const [selected, setSelected] = useState<number[]>([]);
    const control: CommandControl = {
      selectedUnits: own.filter((unit) => selected.includes(unit.id)),
      mode: "move",
      setMode: vi.fn(),
      stop: vi.fn(),
      togglePolicy: vi.fn(),
      toggleDeployment: vi.fn(),
      exitBuilding: vi.fn(),
      pressUnload: vi.fn(),
      popSmoke: vi.fn(),
    };
    return (
      <ArmyDeck
        state={new Feed({ own, control: replay ? undefined : control })}
        selected={selected}
        onSelect={setSelected}
        rules={shippedRules as unknown as PanelRules}
        contactLines={null}
      />
    );
  }
  const view = render(<Harness />);
  const commands = view.getByRole("toolbar", { name: "Commands" });
  expect([...commands.querySelectorAll("button")].every((button) => button.disabled)).toBe(true);
  fireEvent.click(view.getByRole("button", { name: "Tank #11" }));
  expect(view.getByRole("toolbar", { name: "Commands" })).toBeTruthy();
  expect(view.getByRole("button", { name: /^Deploy / }).hasAttribute("disabled")).toBe(true);
  fireEvent.click(view.getByRole("button", { name: "Supply Truck #22" }));
  expect(view.getByRole("button", { name: /^Deploy / })).toBeTruthy();
  expect(view.getByRole("button", { name: /^Attack-move / }).hasAttribute("disabled")).toBe(false);
  expect(view.queryByText(/\d+ selected/i)).toBeNull();
  view.rerender(<Harness replay />);
  expect(view.queryByRole("toolbar")).toBeNull();
  fireEvent.mouseEnter(view.getByRole("button", { name: "Tank #11" }));
  expect(view.getByRole("tooltip").textContent).toContain("MAIN GUN");
});

test("reinforcements remain available before the first unit enters", () => {
  const view = render(
    <ArmyDeck
      state={new Feed({ own: [] })}
      selected={[]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
      reinforcements={<button>Reinforcements</button>}
    />,
  );
  expect(view.getByRole("button", { name: "Reinforcements" })).toBeTruthy();
  expect(view.getByRole("group", { name: "Your units" }).children.length).toBe(0);
});

test("pending reinforcements reuse dimmed vertical cards with deployment hover text", () => {
  const view = render(
    <ArmyDeck
      state={new Feed({ own: [] })}
      selected={[]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
      pending={[
        {
          id: 41,
          kind: "test_tank",
          destination: [10, 12],
          confirmedTick: 0,
          blocked: false,
          arrivalTick: 0,
        },
        {
          id: 43,
          kind: "test_tank",
          destination: [10, 12],
          confirmedTick: 0,
          blocked: true,
          arrivalTick: 0,
        },
      ]}
    />,
  );
  const card = view.getByRole("button", { name: "Tank — Will be deployed" });
  expect(card.className).toContain("hud-army-pending");
  expect(card.getAttribute("title")).toContain("Will be deployed");
  expect(card.hasAttribute("disabled")).toBe(true);
  expect(words(card)).toBe("");
  // A blocked card says so to a screen reader as its hover text does.
  const blocked = view.getByRole("button", { name: "Tank — Entry blocked" });
  expect(blocked.getAttribute("title")).toBe("Tank — Entry blocked");
});

test("a squad bought aboard a carrier waits as two cards, the squad marked aboard", () => {
  const view = render(
    <ArmyDeck
      state={new Feed({ own: [] })}
      selected={[]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
      pending={[
        {
          id: 44,
          kind: "test_rifle",
          carrier: "test_apc",
          destination: [10, 12],
          confirmedTick: 0,
          blocked: false,
          arrivalTick: 0,
        },
      ]}
    />,
  );
  expect(view.getByRole("group", { name: "Your units" }).children.length).toBe(2);
  const carrier = view.getByRole("button", { name: "Test APC — Will be deployed" });
  const squad = view.getByRole("button", {
    name: "Rifle Section, aboard Test APC — Will be deployed",
  });
  expect(carrier.hasAttribute("data-aboard")).toBe(false);
  expect(squad.getAttribute("data-aboard")).toBe("true");
});

test("a waiting aircraft's pending card counts down to its arrival, whole seconds rounded up", () => {
  const hz = shippedRules.tick_hz;
  const jet = {
    id: 42,
    kind: "test_jet",
    destination: [10, 12] as [number, number],
    confirmedTick: 0,
    blocked: false,
    arrivalTick: 20 * hz,
  };
  const deck = (tick: number) => (
    <ArmyDeck
      state={new Feed({ own: [] })}
      selected={[]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
      tick={tick}
      pending={[jet]}
    />
  );
  const view = render(deck(2 * hz + 1));
  const card = view.getByRole("button", { name: "Test Jet — Arriving in 18 s" });
  expect(card.className).toContain("hud-army-pending");
  expect(words(card)).toBe("Arriving in 18 s");
  view.rerender(deck(19 * hz));
  expect(view.getByRole("button", { name: "Test Jet — Arriving in 1 s" })).toBeTruthy();
});

test("the command row exposes refund for a selected skirmish unit", () => {
  const refund = vi.fn();
  const unit = specimenUnit(TEST_CATALOG, "test_tank", { id: 11 });
  const control = {
    selectedUnits: [unit],
    mode: "move" as const,
    setMode: vi.fn(),
    stop: vi.fn(),
    togglePolicy: vi.fn(),
    toggleDeployment: vi.fn(),
    exitBuilding: vi.fn(),
    pressUnload: vi.fn(),
    popSmoke: vi.fn(),
    refund,
  };
  const view = render(
    <ArmyDeck
      state={new Feed({ own: [unit], control })}
      selected={[11]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
    />,
  );
  fireEvent.click(view.getByRole("button", { name: /^Refund · Return to base/ }));
  expect(refund).toHaveBeenCalledOnce();
});

test("a unit returning to base says so on its army card", () => {
  const going = specimenUnit(TEST_CATALOG, "test_tank", { id: 3, withdrawing: true });
  const staying = specimenUnit(TEST_CATALOG, "test_tank", { id: 5 });
  const view = render(
    <ArmyDeck
      state={new Feed({ own: [going, staying] })}
      selected={[]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
    />,
  );
  expect(view.getByRole("button", { name: /#3, returning to base$/ }).dataset.withdrawing).toBe(
    "true",
  );
  expect(view.getByRole("button", { name: /#5$/ }).dataset.withdrawing).toBeUndefined();
});

test("a lost unit's card holds its place, out of reach, while its blink runs, then goes", () => {
  const [a, b, c] = [11, 12, 13].map((id) => specimenUnit(TEST_CATALOG, "test_tank", { id }));
  const state = new Feed({ own: [a, b, c] });
  const blinks = new Feed<ReadonlyMap<number, CardBlink>>(new Map());
  const onSelect = vi.fn();
  const view = render(
    <ArmyDeck
      state={state}
      selected={[]}
      onSelect={onSelect}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
      blinks={blinks}
    />,
  );
  /** Each card's unit, and whether it can be pressed. */
  const deck = () =>
    view
      .getAllByRole("button")
      .filter((card) => card.dataset.unit)
      .map((card) => [Number(card.dataset.unit), !(card as HTMLButtonElement).disabled]);
  const lost = (...ids: number[]) =>
    new Map(ids.map((id) => [id, { colour: "red" as const, serial: id, lost: true }]));

  act(() => {
    blinks.set(lost(12));
    state.set({ own: [a, c] });
  });
  expect(deck()).toEqual([
    [11, true],
    [12, false],
    [13, true],
  ]);
  fireEvent.click(view.getByRole("button", { name: /#12/ }));
  expect(onSelect).not.toHaveBeenCalled();
  act(() => blinks.set(new Map()));
  expect(deck()).toEqual([
    [11, true],
    [13, true],
  ]);
  // The last units lost still show theirs.
  act(() => {
    blinks.set(lost(11, 13));
    state.set({ own: [] });
  });
  expect(deck()).toEqual([
    [11, false],
    [13, false],
  ]);
});

test("a squad aboard a carrier keeps its card, dimmed as aboard, and clicking it selects its carrier", () => {
  const apc = specimenUnit(TEST_CATALOG, "test_apc", { id: 4 });
  const rider = {
    ...specimenUnit(TEST_CATALOG, "test_rifle", { id: 7 }),
    hold: { host: { kind: "carrier" as const, id: 4 }, phase: "inside", progress: 1 },
  };
  const onSelect = vi.fn();
  const view = render(
    <ArmyDeck
      state={new Feed({ own: [apc, rider] })}
      selected={[]}
      onSelect={onSelect}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
    />,
  );
  const card = view.getByRole("button", { name: /^Rifle Section #7/ });
  expect(card.dataset.aboard).toBe("true");
  expect(view.getByRole("button", { name: "Test APC #4" }).dataset.aboard).toBeUndefined();
  fireEvent.click(card);
  expect(onSelect).toHaveBeenLastCalledWith([4]);
});

test("a rider's selected card centres on the carrier it selects", () => {
  const apc = specimenUnit(TEST_CATALOG, "test_apc", { id: 4 });
  const rider = {
    ...specimenUnit(TEST_CATALOG, "test_rifle", { id: 7 }),
    hold: { host: { kind: "carrier" as const, id: 4 }, phase: "inside", progress: 1 },
  };
  const onCenter = vi.fn();
  function Harness() {
    const [selected, setSelected] = useState<number[]>([]);
    return (
      <ArmyDeck
        state={new Feed({ own: [apc, rider] })}
        selected={selected}
        onSelect={setSelected}
        onCenter={onCenter}
        rules={shippedRules as unknown as PanelRules}
        contactLines={null}
      />
    );
  }
  const view = render(<Harness />);
  const card = view.getByRole("button", { name: /^Rifle Section #7/ });
  fireEvent.click(card);
  expect(card.getAttribute("aria-pressed")).toBe("true");
  expect(onCenter).not.toHaveBeenCalled();
  fireEvent.click(card);
  expect(onCenter).toHaveBeenCalledOnce();
  expect(onCenter).toHaveBeenCalledWith(apc);
});

test("hovering a card hovers its unit, a rider's its carrier, until the pointer leaves", () => {
  const apc = specimenUnit(TEST_CATALOG, "test_apc", { id: 4 });
  const tank = specimenUnit(TEST_CATALOG, "test_tank", { id: 5 });
  const rider = {
    ...specimenUnit(TEST_CATALOG, "test_rifle", { id: 7 }),
    hold: { host: { kind: "carrier" as const, id: 4 }, phase: "inside", progress: 1 },
  };
  const onHover = vi.fn();
  const view = render(
    <ArmyDeck
      state={new Feed({ own: [apc, tank, rider] })}
      selected={[]}
      onSelect={vi.fn()}
      onHover={onHover}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
    />,
  );
  const card = (name: RegExp) => view.getByRole("button", { name });
  fireEvent.mouseEnter(card(/ #5$/));
  expect(onHover).toHaveBeenLastCalledWith(5);
  fireEvent.mouseLeave(card(/ #5$/));
  expect(onHover).toHaveBeenLastCalledWith(null);
  fireEvent.mouseEnter(card(/^Rifle Section #7/));
  expect(onHover).toHaveBeenLastCalledWith(4);
});

test("control groups lead the deck in key order, read their key, and delete from the hovered group's ×", () => {
  const own = [11, 22, 33, 44, 55].map((id) =>
    specimenUnit(TEST_CATALOG, id === 33 ? "test_supply" : "test_tank", { id }),
  );
  const onDeleteGroup = vi.fn();
  function Harness() {
    const [selected, setSelected] = useState<number[]>([]);
    return (
      <ArmyDeck
        state={new Feed({ own })}
        selected={selected}
        onSelect={setSelected}
        groups={
          new Map([
            ["0", [22]],
            ["2", [55, 33]],
          ])
        }
        onDeleteGroup={onDeleteGroup}
        rules={shippedRules as unknown as PanelRules}
        contactLines={null}
      />
    );
  }
  const view = render(<Harness />);
  const cards = () =>
    view.getAllByRole("button", { name: /#\d+/ }).map((b) => b.getAttribute("aria-label"));
  // Group 2 before group 0 (1–9, then 0), each in army order, then the rest.
  expect(cards()).toEqual([
    "Supply Truck #33, group 2",
    "Tank #55, group 2",
    "Tank #22, group 0",
    "Tank #11",
    "Tank #44",
  ]);
  // A run follows the deck's order across groups.
  fireEvent.click(view.getByRole("button", { name: "Tank #55, group 2" }));
  fireEvent.click(view.getByRole("button", { name: "Tank #11" }), { shiftKey: true });
  expect(
    view.getAllByRole("button", { pressed: true }).map((b) => b.getAttribute("aria-label")),
  ).toEqual(["Tank #55, group 2", "Tank #22, group 0", "Tank #11"]);
  // Only the group under the pointer offers its ×.
  expect(view.queryByRole("button", { name: /^Delete group/ })).toBeNull();
  fireEvent.mouseEnter(view.getByRole("button", { name: "Tank #22, group 0" }).parentElement!);
  fireEvent.click(view.getByRole("button", { name: "Delete group 0" }));
  expect(onDeleteGroup).toHaveBeenCalledWith("0");
  expect(view.queryByRole("button", { name: "Delete group 2" })).toBeNull();
});

test("right-click asks the store for the exact unit, without selecting or centring it", () => {
  const tank = specimenUnit(TEST_CATALOG, "test_tank", { id: 11 });
  const onLookup = vi.fn();
  const onSelect = vi.fn();
  const onCenter = vi.fn();
  const view = render(
    <ArmyDeck
      state={new Feed({ own: [tank] })}
      selected={[]}
      onSelect={onSelect}
      onCenter={onCenter}
      onLookup={onLookup}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
    />,
  );
  const card = view.getByRole("button", { name: "Tank #11" });
  // The card's own context menu would cover the store.
  expect(fireEvent.contextMenu(card)).toBe(false);
  expect(onLookup).toHaveBeenCalledExactlyOnceWith(tank);
  expect(onSelect).not.toHaveBeenCalled();
  expect(onCenter).not.toHaveBeenCalled();
  expect(card.getAttribute("aria-pressed")).toBe("false");
});

test("a card's reserve mark is its unit panel's reserve row", () => {
  const own = [
    specimenUnit(TEST_CATALOG, "test_tank", { id: 1 }),
    specimenUnit(TEST_CATALOG, "test_tank", { id: 2 }, [{ ammo: [1, 1] }]),
    specimenUnit(TEST_CATALOG, "test_tank", { id: 3 }, [{ ammo: [0, 10] }]),
  ];
  const view = render(
    <ArmyDeck
      state={new Feed({ own })}
      selected={[]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      contactLines={null}
    />,
  );
  const rules = shippedRules as unknown as PanelRules;
  for (const unit of own) {
    const row = reserveRow(TEST_CATALOG.units, unit, rules);
    const panel = ownPanel(TEST_CATALOG.units, unit, own, rules);
    expect(panel.states.filter((r) => r.state === row?.state)).toHaveLength(row ? 1 : 0);
    const name = `Tank #${unit.id}${row ? `, ${row.word.toLowerCase()}` : ""}`;
    const mark = view.getByRole("button", { name }).querySelector<HTMLElement>(".hud-army-reserve");
    const drawn = document.createElement("span");
    drawn.innerHTML = row ? (iconSvg(row.icon) ?? "") : "";
    expect(mark?.innerHTML ?? null, name).toBe(row ? drawn.innerHTML : null);
  }
  // Tank #2 runs low; tank #3's empty AP is out though its HE has rounds.
  expect(own.map((u) => reserveRow(TEST_CATALOG.units, u, rules)?.state ?? null)).toEqual([
    null,
    "low_ammo",
    "no_ammo",
  ]);
});
