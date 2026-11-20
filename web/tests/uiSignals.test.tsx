// What the coach hears of the player's acts (`uiSignals.ts`): each signal
// is emitted where the act happens, once per act, and never waits on what
// the battle holds.
import { act, renderHook } from "@testing-library/react";
import { expect, test } from "vitest";
import { own, recordingClient, wrapper } from "./unitControlHarness";
import { useUnitControl } from "../src/battle/input/useUnitControl";
import { listenContactFocus, type FocusTarget } from "../src/battle/input/contactFocus";
import { ContactFocusBinding } from "../src/battle/input/commandBindings";
import { onUiSignal, type UiSignal } from "../src/battle/input/uiSignals";
import type { ObservationView } from "../src/battle/sim/observationView";

test("the coach hears a selection as the player makes it, not with each new observation", () => {
  const observed = (tick: number) =>
    ({
      tick,
      own: [own(1, "test_rifle"), own(2, "test_supply")],
      contacts: [],
    }) as unknown as ObservationView;
  const { client } = recordingClient();
  const heard: UiSignal[] = [];
  const stop = onUiSignal((signal) => heard.push(signal));
  const hook = renderHook(({ observation }) => useUnitControl(client, observation), {
    wrapper,
    initialProps: { observation: observed(1) },
  });
  const selections = () => heard.filter((signal) => signal === "selected").length;
  act(() => hook.result.current.setSelected([1, 2]));
  expect(selections()).toBe(1);
  // The battle goes on with the same selection: nothing was selected again.
  hook.rerender({ observation: observed(2) });
  hook.rerender({ observation: observed(3) });
  expect(selections()).toBe(1);
  // Losing a unit from the selection is no selection either.
  hook.rerender({ observation: { ...observed(4), own: [own(1, "test_rifle")] } });
  expect(hook.result.current.selected).toEqual([1]);
  expect(selections()).toBe(1);
  // Selecting the same unit again is another act.
  act(() => hook.result.current.setSelected([1]));
  expect(selections()).toBe(2);
  stop();
  hook.unmount();
});

test("the coach hears the jump key whether or not a contact is there to jump to", () => {
  const stop = new AbortController();
  const heard: UiSignal[] = [];
  const unhear = onUiSignal((signal) => heard.push(signal));
  const centred: (readonly [number, number])[] = [];
  let list: FocusTarget[] = [];
  let blocked = false;
  listenContactFocus(
    stop.signal,
    () => list,
    () => blocked,
    (target) => centred.push(target),
  );
  const press = () =>
    window.dispatchEvent(new KeyboardEvent("keydown", { code: ContactFocusBinding.code }));
  try {
    press();
    expect(heard).toEqual(["contact_focus"]);
    expect(centred).toEqual([]);
    list = [{ id: 7, target: [7, 7], evidenceTick: 7 }];
    press();
    expect(heard).toEqual(["contact_focus", "contact_focus"]);
    expect(centred).toEqual([[7, 7]]);
    // A key the battle doesn't take (its menu is open) is no jump.
    blocked = true;
    press();
    expect(heard).toHaveLength(2);
  } finally {
    stop.abort();
    unhear();
  }
});
