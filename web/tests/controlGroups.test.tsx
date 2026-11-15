import { act, renderHook } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { useUnitControl } from "../src/battle/input/useUnitControl";
import type { ObservationView, OwnUnitView } from "../src/battle/sim/observationView";
import { recordingClient, wrapper } from "./unitControlHarness";

const unit = (id: number, kind: string, x: number, extra: Partial<OwnUnitView> = {}) =>
  ({ id, kind, position: [x, 0, 0], yaw: 0, ...extra }) as unknown as OwnUnitView;
const aboard = {
  hold: { host: { kind: "carrier" as const, id: 4 }, phase: "inside", progress: 1 },
} as Partial<OwnUnitView>;
const observe = (own: OwnUnitView[]) => ({ own, contacts: [] }) as unknown as ObservationView;

/** A key pressed at the window, as the battle hears it; true if the game took it. */
function press(code: string, mods: { ctrl?: boolean; shift?: boolean } = {}, target?: Element) {
  const event = new KeyboardEvent("keydown", {
    code,
    ctrlKey: mods.ctrl ?? false,
    shiftKey: mods.shift ?? false,
    bubbles: true,
    cancelable: true,
  });
  act(() => {
    (target ?? window).dispatchEvent(event);
  });
  return event.defaultPrevented;
}

test("Ctrl+N stores a group, N selects it, Shift+N adds, N twice centres, and groups outlive riding but not death", () => {
  const rifle = unit(1, "test_rifle", 0);
  const tank = unit(2, "test_tank", 4);
  const truck = unit(3, "test_supply", 20);
  const apc = unit(4, "test_apc", 60);
  const squad = unit(5, "test_rifle", 61);
  const onCenter = vi.fn();
  const { client } = recordingClient();
  const hook = renderHook(({ observation }) => useUnitControl(client, observation, { onCenter }), {
    wrapper,
    initialProps: { observation: observe([rifle, tank, truck, apc, squad]) },
  });
  const groups = () => Object.fromEntries(hook.result.current.groups);
  const select = (ids: number[]) => act(() => hook.result.current.setSelected(ids));

  // Ctrl+1 takes the browser's tab switch for the battle.
  select([1, 2, 5]);
  expect(press("Digit1", { ctrl: true })).toBe(true);
  expect(groups()).toEqual({ 1: [1, 2, 5] });

  select([3]);
  expect(press("Digit1")).toBe(true);
  expect(hook.result.current.selected).toEqual([1, 2, 5]);
  expect(onCenter).not.toHaveBeenCalled();
  // Pressed again at once: the camera goes to the member nearest the group's centre.
  press("Digit1");
  expect(onCenter).toHaveBeenCalledWith(tank);

  // Shift+N adds; Ctrl+N takes units from any other group they were in.
  select([3]);
  press("Digit1", { shift: true });
  expect(groups()).toEqual({ 1: [1, 2, 5, 3] });
  select([2]);
  press("Digit2", { ctrl: true });
  expect(groups()).toEqual({ 1: [1, 5, 3], 2: [2] });
  // Ctrl+N with nothing selected keeps the group as it was.
  select([]);
  press("Digit2", { ctrl: true });
  expect(groups()).toEqual({ 1: [1, 5, 3], 2: [2] });

  // A squad that boards stays in its group, but only units on the field are selected.
  hook.rerender({ observation: observe([rifle, tank, truck, apc, { ...squad, ...aboard }]) });
  press("Digit1");
  expect(groups()).toEqual({ 1: [1, 5, 3], 2: [2] });
  expect(hook.result.current.selected).toEqual([1, 3]);

  // The dead leave their group, and a group with no one left disappears.
  hook.rerender({ observation: observe([rifle, truck, apc, { ...squad, ...aboard }]) });
  expect(groups()).toEqual({ 1: [1, 5, 3] });

  // Keys typed into a field stay text.
  const field = document.body.appendChild(document.createElement("input"));
  expect(press("Digit3", { ctrl: true }, field)).toBe(false);
  field.remove();

  act(() => hook.result.current.deleteGroup("1"));
  expect(groups()).toEqual({});
});

test("a new battle starts with no groups", () => {
  const observation = observe([unit(1, "test_rifle", 0)]);
  const first = recordingClient().client;
  const hook = renderHook(({ client }) => useUnitControl(client, observation), {
    wrapper,
    initialProps: { client: first },
  });
  act(() => hook.result.current.setSelected([1]));
  press("Digit4", { ctrl: true });
  expect([...hook.result.current.groups.keys()]).toEqual(["4"]);
  hook.rerender({ client: recordingClient().client });
  expect(hook.result.current.groups.size).toBe(0);
});
