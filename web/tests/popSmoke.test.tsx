// Popping smoke (`specs/done/smoke-screen`, D15): H, or POP SMOKE on the command
// bar, sends one `pop_smoke` naming exactly the selected units that still
// have a charge, and nothing else, so no unit's current order changes; with
// no charge among them the bar's button is off and the key sends nothing.
// The order flashes no order marks: it sends no unit anywhere.
import { act, fireEvent, render, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { expect, test } from "vitest";
import { Feed } from "@web/feed";
import { SessionCatalogProvider } from "@web/battle/catalog/context";
import { TEST_CATALOG } from "./catalog";
import { useUnitControl } from "../src/battle/input/useUnitControl";
import { CommandBindings } from "../src/battle/input/commandBindings";
import { CommandBar } from "../src/battle/present/commandBar";
import { OrderReveal } from "../src/battle/present/orderReveal";
import type { SimClient } from "../src/battle/sim/client";
import type { ObservationView, OwnUnitView } from "../src/battle/sim/observationView";
import type { Order } from "../src/battle/sim/protocol";

const wrapper = ({ children }: { children: ReactNode }) => (
  <SessionCatalogProvider catalog={TEST_CATALOG}>{children}</SessionCatalogProvider>
);

const unit = (id: number, kind: string, smoke: number | null) =>
  ({ id, kind, position: [0, 0, 0], yaw: 0, smoke, state: "moving" }) as unknown as OwnUnitView;

/** A smoke tank with charges, a smoke squad with none, a tank without
 *  smoke, all selected, ordering through a client that records. */
function selection(charges = 2) {
  const sent: Order[] = [];
  const client = {
    command: async (order: Order) => {
      sent.push(order);
      return { seq: sent.length, applied_tick: 0, error: null, refused: [] };
    },
  } as unknown as SimClient;
  const observation = {
    own: [
      unit(1, "test_smoke_tank", charges),
      unit(2, "test_smoke_squad", 0),
      unit(3, "test_tank", null),
    ],
    contacts: [],
  } as unknown as ObservationView;
  const hook = renderHook(() => useUnitControl(client, observation), { wrapper });
  act(() => hook.result.current.setSelected([1, 2, 3]));
  return { hook, sent, observation };
}

const press = (code: string) =>
  act(async () => {
    window.dispatchEvent(new KeyboardEvent("keydown", { code, bubbles: true }));
  });

test("H pops smoke on exactly the selected units with a charge, and sends nothing else", async () => {
  const { hook, sent } = selection();
  await press(CommandBindings.pop_smoke.code);
  expect(sent).toEqual([{ kind: "pop_smoke", units: [1] }]);
  expect(hook.result.current.selected).toEqual([1, 2, 3]);
  expect(hook.result.current.mode).toBe("move");
  hook.unmount();
});

test("with no charge among the selection, H sends nothing and POP SMOKE is off", async () => {
  const { hook, sent, observation } = selection(0);
  await press(CommandBindings.pop_smoke.code);
  expect(sent).toEqual([]);
  const view = render(
    <CommandBar state={new Feed({ control: hook.result.current, own: observation.own })} />,
    { wrapper },
  );
  expect(view.getByRole("button", { name: /^Pop smoke/ }).hasAttribute("disabled")).toBe(true);
  view.unmount();
  hook.unmount();
});

test("POP SMOKE on the bar pops it as H does", async () => {
  const { hook, sent, observation } = selection();
  const view = render(
    <CommandBar state={new Feed({ control: hook.result.current, own: observation.own })} />,
    { wrapper },
  );
  await act(async () => fireEvent.click(view.getByRole("button", { name: /^Pop smoke/ })));
  expect(sent).toEqual([{ kind: "pop_smoke", units: [1] }]);
  view.unmount();
  hook.unmount();
});

test("a pop flashes no order marks", () => {
  const reveal = new OrderReveal({ hold_s: 1, fade_s: 1 });
  reveal.noteOrder({ kind: "pop_smoke", units: [1] }, 0);
  expect(reveal.at(0.5, false, [{ id: 1 }]).size).toBe(0);
});
