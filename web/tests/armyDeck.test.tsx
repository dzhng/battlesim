import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, expect, test, vi } from "vitest";
import { useState } from "react";
import game from "@fixtures/game.json";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import type { PanelRules } from "../src/battle/present/panelRows";
import { CommandBar } from "../src/battle/present/readouts";
import { ArmyDeck } from "../src/battle/present/armyDeck";

beforeAll(() => {
  HTMLElement.prototype.scrollIntoView = vi.fn();
});
afterAll(() => {
  delete (HTMLElement.prototype as Partial<HTMLElement>).scrollIntoView;
});
afterEach(cleanup);

test("the army row includes unselected own units and stays visible without a selection", () => {
  const own = [specimenUnit("tank", { id: 11 }), specimenUnit("rifle", { id: 22 })];
  const view = render(
    <ArmyDeck
      own={own}
      selected={[]}
      onSelect={vi.fn()}
      rules={game as unknown as PanelRules}
      captions={null}
    />,
  );
  expect(view.getByRole("button", { name: "Tank #11" }).getAttribute("aria-pressed")).toBe("false");
  expect(view.getByRole("button", { name: "Rifle squad #22" }).getAttribute("aria-pressed")).toBe(
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
        own={[specimenUnit("tank", { id: 11 }), specimenUnit("rifle", { id: 22 })]}
        selected={selected}
        onSelect={setSelected}
        rules={game as unknown as PanelRules}
        captions={null}
      />
    );
  }
  const view = render(<Harness />);
  const tank = view.getByRole("button", { name: "Tank #11" });
  const rifle = view.getByRole("button", { name: "Rifle squad #22" });
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

test("hover and focus expose live unit facts, health and casualty removal without claiming Escape", () => {
  const tank = specimenUnit("tank", { id: 11 });
  const rifle = specimenUnit("rifle", { id: 22, suppression: "suppressed", service: "serving" });
  const props = {
    selected: [],
    onSelect: vi.fn(),
    rules: game as unknown as PanelRules,
    captions: null,
  };
  const view = render(<ArmyDeck {...props} own={[tank, rifle]} />);
  const card = view.getByRole("button", { name: "Rifle squad #22" });
  fireEvent.mouseEnter(card);
  const hint = view.getByRole("tooltip");
  expect(hint.textContent).toContain("Rifle squad #22");
  expect(hint.textContent).toContain("SUPPRESSED");
  expect(card.getAttribute("aria-describedby")).toBe(hint.id);
  fireEvent.mouseLeave(card);
  expect(view.queryByRole("tooltip")).toBeNull();
  fireEvent.focus(view.getByRole("button", { name: "Tank #11" }));
  expect(view.getByRole("tooltip").textContent).toContain("CANNON");
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
      {...props}
      own={[tank, { ...rifle, memberHp: rifle.memberHp.map((hp) => hp / 2) }]}
    />,
  );
  expect(
    view.getByRole("meter", { name: "Rifle squad #22 health" }).getAttribute("aria-valuenow"),
  ).toBe("50");
  view.rerender(<ArmyDeck {...props} own={[tank]} />);
  expect(view.queryByRole("button", { name: "Rifle squad #22" })).toBeNull();
  expect(view.queryByRole("tooltip")).toBeNull();
});

test("the army remains in replay and empty selection while commands follow only selected units", () => {
  const own = [specimenUnit("tank", { id: 11 }), specimenUnit("supply", { id: 22 })];
  function Harness({ replay = false }: { replay?: boolean }) {
    const [selected, setSelected] = useState<number[]>([]);
    const control: Parameters<typeof CommandBar>[0]["control"] = {
      selectedUnits: own.filter((unit) => selected.includes(unit.id)),
      mode: "move",
      setMode: vi.fn(),
      stop: vi.fn(),
      togglePolicy: vi.fn(),
      toggleDeployment: vi.fn(),
      exitBuilding: vi.fn(),
    };
    return (
      <ArmyDeck
        own={own}
        selected={selected}
        onSelect={setSelected}
        rules={game as unknown as PanelRules}
        control={replay ? undefined : control}
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
  fireEvent.click(view.getByRole("button", { name: "Supply truck #22" }));
  expect(view.getByRole("button", { name: /^Deploy / })).toBeTruthy();
  expect(view.getByRole("button", { name: /^Attack-move / }).hasAttribute("disabled")).toBe(false);
  expect(view.queryByText(/\d+ selected/i)).toBeNull();
  view.rerender(<Harness replay />);
  expect(view.queryByRole("toolbar")).toBeNull();
  fireEvent.focus(view.getByRole("button", { name: "Tank #11" }));
  expect(view.getByRole("tooltip").textContent).toContain("CANNON");
});

test("keyboard focus takes ownership from an old hover and mouse leave restores focused facts", () => {
  const view = render(
    <ArmyDeck
      own={[specimenUnit("tank", { id: 11 }), specimenUnit("rifle", { id: 22 })]}
      selected={[]}
      onSelect={vi.fn()}
      rules={game as unknown as PanelRules}
      captions={null}
    />,
  );
  const tank = view.getByRole("button", { name: "Tank #11" });
  const rifle = view.getByRole("button", { name: "Rifle squad #22" });
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
      own={[]}
      selected={[]}
      onSelect={vi.fn()}
      rules={game as unknown as PanelRules}
      captions={null}
      reinforcements={<button>Reinforcements</button>}
    />,
  );
  expect(view.getByRole("button", { name: "Reinforcements" })).toBeTruthy();
  expect(view.getByRole("group", { name: "Your units" }).children.length).toBe(0);
});

test("the command row exposes refund for a selected skirmish unit", () => {
  const refund = vi.fn();
  const unit = specimenUnit("tank", {id:11});
  const control = {selectedUnits:[unit],mode:"move" as const,setMode:vi.fn(),stop:vi.fn(),togglePolicy:vi.fn(),toggleDeployment:vi.fn(),exitBuilding:vi.fn(),refund};
  const view = render(<ArmyDeck own={[unit]} selected={[11]} onSelect={vi.fn()} rules={game as unknown as PanelRules} captions={null} control={control} />);
  fireEvent.click(view.getByRole("button", {name:"Refund · Return to base"}));
  expect(refund).toHaveBeenCalledOnce();
});
