import { shippedRules } from "@web/shippedRules";
import { act, cleanup, fireEvent, render as renderView } from "@testing-library/react";
import { Feed, useFeed } from "@apps/battle-lab/src/feed";
import { WithTestCatalog } from "./catalog";
import { TEST_CATALOG } from "./catalog";
import { afterAll, afterEach, beforeAll, expect, test, vi } from "vitest";
import { useLayoutEffect, useState } from "react";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import type { PanelRules } from "../src/battle/present/panelRows";
import type { CommandControl } from "../src/battle/present/commandBar";
import { ArmyDeck } from "../src/battle/present/armyDeck";

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
          captions={null}
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
    },
  });
  const view = render(
    <ArmyDeck
      state={state}
      selected={[11]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      captions={null}
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
      captions={null}
    />,
  );
  expect(view.getByRole("button", { name: "Tank #11" }).getAttribute("aria-pressed")).toBe("false");
  expect(view.getByRole("button", { name: "Rifle Section #22" }).getAttribute("aria-pressed")).toBe(
    "false",
  );
  expect(view.queryByRole("toolbar", { name: "Commands" })).toBeNull();
  expect(view.queryByRole("tooltip")).toBeNull();
});

test("card selection replaces the current selection and Shift toggles individual units", () => {
  function Harness() {
    const [selected, setSelected] = useState([11]);
    return (
      <ArmyDeck
        state={
          new Feed({
            own: [
              specimenUnit(TEST_CATALOG, "test_tank", { id: 11 }),
              specimenUnit(TEST_CATALOG, "test_rifle", { id: 22 }),
            ],
          })
        }
        selected={selected}
        onSelect={setSelected}
        rules={shippedRules as unknown as PanelRules}
        captions={null}
      />
    );
  }
  const view = render(<Harness />);
  const tank = view.getByRole("button", { name: "Tank #11" });
  const rifle = view.getByRole("button", { name: "Rifle Section #22" });
  fireEvent.click(rifle);
  expect(tank.getAttribute("aria-pressed")).toBe("false");
  expect(rifle.getAttribute("aria-pressed")).toBe("true");
  fireEvent.click(tank, { shiftKey: true });
  expect(tank.getAttribute("aria-pressed")).toBe("true");
  expect(rifle.getAttribute("aria-pressed")).toBe("true");
  fireEvent.click(rifle, { shiftKey: true });
  expect(tank.getAttribute("aria-pressed")).toBe("true");
  expect(rifle.getAttribute("aria-pressed")).toBe("false");
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
        captions={null}
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
  fireEvent.click(rifleCard, { shiftKey: true });
  expect(rifleCard.getAttribute("aria-pressed")).toBe("false");
  expect(onCenter).not.toHaveBeenCalled();
});

test("hover and focus expose live unit facts, health and casualty removal without claiming Escape", () => {
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
    captions: null,
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
  fireEvent.focus(view.getByRole("button", { name: "Tank #11" }));
  expect(view.getByRole("tooltip").textContent).toContain("MAIN GUN");
  expect(view.getByRole("tooltip").textContent).toContain("HMG");
  const battleEscape = vi.fn();
  window.addEventListener("keydown", battleEscape);
  const key = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
  try {
    fireEvent(view.getByRole("button", { name: "Tank #11" }), key);
    expect(view.queryByRole("tooltip")).toBeNull();
    expect(key.defaultPrevented).toBe(false);
    expect(battleEscape).toHaveBeenCalledOnce();
  } finally {
    window.removeEventListener("keydown", battleEscape);
  }
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
      captions={<ul className="hud-subtitles" />}
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
    };
    return (
      <ArmyDeck
        state={new Feed({ own, control: replay ? undefined : control })}
        selected={selected}
        onSelect={setSelected}
        rules={shippedRules as unknown as PanelRules}
        captions={null}
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
  fireEvent.focus(view.getByRole("button", { name: "Tank #11" }));
  expect(view.getByRole("tooltip").textContent).toContain("MAIN GUN");
});

test("keyboard focus takes ownership from an old hover and mouse leave restores focused facts", () => {
  const view = render(
    <ArmyDeck
      state={
        new Feed({
          own: [
            specimenUnit(TEST_CATALOG, "test_tank", { id: 11 }),
            specimenUnit(TEST_CATALOG, "test_rifle", { id: 22 }),
          ],
        })
      }
      selected={[]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      captions={null}
    />,
  );
  const tank = view.getByRole("button", { name: "Tank #11" });
  const rifle = view.getByRole("button", { name: "Rifle Section #22" });
  fireEvent.mouseEnter(tank);
  fireEvent.focus(rifle);
  expect(view.getByRole("tooltip").getAttribute("data-unit")).toBe("22");
  expect(rifle.getAttribute("aria-describedby")).toBe(view.getByRole("tooltip").id);
  fireEvent.mouseEnter(tank);
  expect(view.getByRole("tooltip").getAttribute("data-unit")).toBe("11");
  fireEvent.mouseLeave(tank);
  expect(view.getByRole("tooltip").getAttribute("data-unit")).toBe("22");
});

test("reinforcements remain available before the first unit enters", () => {
  const view = render(
    <ArmyDeck
      state={new Feed({ own: [] })}
      selected={[]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      captions={null}
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
      captions={null}
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
      captions={null}
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
    refund,
  };
  const view = render(
    <ArmyDeck
      state={new Feed({ own: [unit], control })}
      selected={[11]}
      onSelect={vi.fn()}
      rules={shippedRules as unknown as PanelRules}
      captions={null}
    />,
  );
  fireEvent.click(view.getByRole("button", { name: "Refund · Return to base" }));
  expect(refund).toHaveBeenCalledOnce();
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
      captions={null}
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
        captions={null}
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
