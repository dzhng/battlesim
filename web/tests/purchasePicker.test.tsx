import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { PurchasePicker } from "@web/battle/present/purchasePicker";
import type { UnitCard } from "@packages/scene-assets/src/units";
import type { SkirmishView } from "@web/battle/sim/observation";
afterEach(cleanup);
const card = (id: string, variant: string, reason: string | null = null): UnitCard => ({
  id,
  name: variant,
  description: "",
  family: "abrams",
  cost: 200,
  roster: { factions: ["us"], family_name: "M1 Abrams", category: "veh", variant },
  disabled_reason: reason,
});
const match: SkirmishView = {
  phase: "preparation",
  ready: [false, false],
  preparationRemainingS: 60,
  credits: 1000,
  occupiedSlots: 0,
  maxUnits: 30,
  pending: [],
  scores: [0, 0],
  result: null,
  objectives: [],
};
test("an empty army can inspect one family and purchase its enabled concrete variant", () => {
  const choose = vi.fn();
  const view = render(
    <PurchasePicker
      cards={[card("base", "SEP v2"), card("trophy", "SEP v2 Trophy", "Protection deferred")]}
      faction="us"
      match={match}
      onChoose={choose}
      onReady={vi.fn()}
      onCancelPending={vi.fn()}
    />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  expect(view.getAllByRole("button", { name: "M1 Abrams" })).toHaveLength(1);
  fireEvent.click(view.getByRole("button", { name: "M1 Abrams" }));
  expect(choose).toHaveBeenCalledExactlyOnceWith("base");
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  const unavailable = view.getByRole("button", {
    name: "SEP v2 Trophy — 200 credits — Unavailable",
  });
  expect(unavailable.hasAttribute("disabled")).toBe(true);
  fireEvent.click(unavailable);
  expect(choose).toHaveBeenCalledTimes(1);
  expect(view.queryByLabelText("Unit slots")).toBeNull();
});

test("purchase availability follows observed credits and reserved slots", () => {
  const choose = vi.fn();
  const props = {
    cards: [card("base", "SEP v2")],
    faction: "us" as const,
    onChoose: choose,
    onReady: vi.fn(),
    onCancelPending: vi.fn(),
  };
  // A one-variant family is bought by clicking it, once it is affordable
  // and a slot is free.
  const view = render(<PurchasePicker {...props} match={{ ...match, credits: 199 }} />);
  const family = () => view.getByRole("button", { name: "M1 Abrams" });
  const open = () => {
    fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
    fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  };
  open();
  fireEvent.click(family());
  expect(choose).not.toHaveBeenCalled();
  view.rerender(
    <PurchasePicker {...props} match={{ ...match, credits: 200, occupiedSlots: 30 }} />,
  );
  fireEvent.click(family());
  expect(choose).not.toHaveBeenCalled();
  view.rerender(
    <PurchasePicker {...props} match={{ ...match, credits: 200, occupiedSlots: 29 }} />,
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
      onReady={vi.fn()}
      onCancelPending={vi.fn()}
    />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  fireEvent.click(view.getByRole("tab", { name: "VEH" }));
  fireEvent.click(view.getByRole("button", { name: "M1 Abrams" }));
  expect(choose).toHaveBeenCalledExactlyOnceWith("base");
});
