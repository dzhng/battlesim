import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { CommandBar } from "../src/battle/present/readouts";
import type { OwnUnitView } from "../src/battle/sim/observation";

const unit = (kind: string, patch: Partial<OwnUnitView> = {}) =>
  ({ kind, engagement: "fire_at_will", garrison: null, deployment: null, ...patch }) as OwnUnitView;
const control = (selectedUnits: OwnUnitView[]) => ({
  selectedUnits,
  mode: "move" as const,
  setMode: vi.fn(),
  stop: vi.fn(),
  togglePolicy: vi.fn(),
  toggleDeployment: vi.fn(),
  exitBuilding: vi.fn(),
});
afterEach(cleanup);

test("commands omit redundant Move/Garrison while retaining mixed-selection capability actions", () => {
  const c = control([
    unit("rifle", { garrison: { building: 1 } as OwnUnitView["garrison"] }),
    unit("supply"),
  ]);
  const view = render(<CommandBar control={c} />);
  expect(view.queryByRole("button", { name: /^Move / })).toBeNull();
  expect(view.queryByRole("button", { name: /^Garrison / })).toBeNull();
  const deploy = view.getByRole("button", { name: /^Deploy / });
  const exit = view.getByRole("button", { name: "Leave building" });
  expect(deploy.getAttribute("data-reach")).toBe("1/2");
  expect(exit.getAttribute("data-reach")).toBe("1/2");
  fireEvent.click(deploy);
  fireEvent.click(exit);
  fireEvent.click(view.getByRole("button", { name: /^Attack-move / }));
  expect(c.toggleDeployment).toHaveBeenCalledOnce();
  expect(c.exitBuilding).toHaveBeenCalledOnce();
  expect(c.setMode).toHaveBeenCalledWith("attack_move");
});

test("icon commands expose the full shortcut on focus and hover, including partial capability reach", () => {
  const c = control([unit("tank"), unit("supply")]);
  const view = render(<CommandBar control={c} />);
  const attack = view.getByRole("button", { name: /^Attack-move / });
  expect(view.queryByRole("tooltip")).toBeNull();
  fireEvent.focus(attack);
  const tip = view.getByRole("tooltip");
  expect(tip.textContent).toBe("Attack-move (X or Ctrl+right-click): 1 of 2 selected");
  expect(attack.getAttribute("aria-describedby")).toBe(tip.id);
  fireEvent.mouseEnter(attack);
  fireEvent.mouseLeave(attack);
  expect(view.getByRole("tooltip").textContent).toBe(tip.textContent);
  fireEvent.blur(attack);
  expect(view.queryByRole("tooltip")).toBeNull();
  fireEvent.mouseEnter(attack);
  expect(view.getByRole("tooltip").textContent).toContain("Ctrl+right-click");
  fireEvent.mouseLeave(attack);
  expect(view.queryByRole("tooltip")).toBeNull();
});

test("Escape dismisses a focused tooltip without claiming the battle's Escape event", () => {
  const view = render(<CommandBar control={control([unit("tank")])} />);
  const attack = view.getByRole("button", { name: /^Attack-move / });
  fireEvent.focus(attack);
  expect(view.getByRole("tooltip")).toBeTruthy();
  const key = new KeyboardEvent("keydown", {
    key: "Escape",
    code: "Escape",
    bubbles: true,
    cancelable: true,
  });
  const battleEscape = vi.fn();
  window.addEventListener("keydown", battleEscape);
  try {
    fireEvent(attack, key);
    expect(view.queryByRole("tooltip")).toBeNull();
    expect(key.defaultPrevented).toBe(false);
    expect(battleEscape).toHaveBeenCalledOnce();
  } finally {
    window.removeEventListener("keydown", battleEscape);
  }
});
