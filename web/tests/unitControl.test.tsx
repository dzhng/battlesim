// The player's right-click at the input seam: `useUnitControl.onPointer`
// with a pick, and the orders it sends to a recording client.
import { act, renderHook } from "@testing-library/react";
import { expect, test } from "vitest";
import { contactUnder } from "../src/battle/input/contactPick";
import { useUnitControl, type PointerPick } from "../src/battle/input/useUnitControl";
import type { SimClient } from "../src/battle/sim/client";
import type { ContactView, ObservationView, OwnUnitView } from "../src/battle/sim/observation";
import type { Order } from "../src/battle/sim/protocol";

/** A client that records what it is sent and accepts all of it. */
function recordingClient() {
  const sent: { order: Order; queued: boolean }[] = [];
  const client = {
    command: async (order: Order, queued = false) => {
      sent.push({ order, queued });
      return { seq: sent.length, applied_tick: 0, error: null };
    },
  } as unknown as SimClient;
  return { client, sent };
}

const own = (id: number, kind: string) =>
  ({ id, kind, position: [0, 0, 0], yaw: 0 }) as unknown as OwnUnitView;
const contact = (id: number, center: [number, number], radius: number): ContactView => ({
  id,
  source: "firing",
  center,
  radius,
  evidenceTick: 0,
  expiresTick: 100,
  kind: null,
  heard: [],
});

/** A rifle squad (armed) and a supply truck (unarmed), both selected. */
async function control(contacts: ContactView[] = []) {
  const observation = {
    own: [own(1, "rifle"), own(2, "supply")],
    contacts,
  } as unknown as ObservationView;
  const { client, sent } = recordingClient();
  const hook = renderHook(() => useUnitControl(client, observation));
  act(() => hook.result.current.setSelected([1, 2]));
  const click = async (pick: Partial<PointerPick>) => {
    await act(async () =>
      hook.result.current.onPointer({
        unit: null,
        button: "right",
        shift: false,
        ctrl: false,
        x: 0,
        y: 0,
        time: 0,
        ground: [50, 50],
        ...pick,
      }),
    );
  };
  return { hook, sent, click };
}

test("right-clicking a contact attacks it with the armed units; Shift queues", async () => {
  const { sent, click } = await control();
  await click({ contact: 7 });
  await click({ contact: 7, shift: true });
  expect(sent).toEqual([
    { order: { kind: "attack", units: [1], target: { kind: "contact", id: 7 } }, queued: false },
    { order: { kind: "attack", units: [1], target: { kind: "contact", id: 7 } }, queued: true },
  ]);
});

test("an identified enemy under the cursor wins over a contact", async () => {
  const { sent, click } = await control();
  await click({ contact: 7, enemy: 3 });
  expect(sent.map((s) => s.order)).toEqual([
    { kind: "attack", units: [1], target: { kind: "identified", id: 3 } },
  ]);
});

test("an unarmed selection right-clicking a contact moves there instead", async () => {
  const { hook, sent, click } = await control();
  act(() => hook.result.current.setSelected([2]));
  await click({ contact: 7 });
  expect(sent.map((s) => s.order.kind)).toEqual(["move"]);
});

test("the contact under a ground point is the one whose area holds it, the nearest centre first", () => {
  const areas = [contact(1, [0, 0], 10), contact(2, [12, 0], 10), contact(3, [100, 0], 5)];
  expect(contactUnder(areas, [8, 0])).toBe(2);
  expect(contactUnder(areas, [-9, 0])).toBe(1);
  expect(contactUnder(areas, [50, 0])).toBe(null);
  expect(contactUnder(areas, [104.9, 0])).toBe(3);
  expect(contactUnder(areas, null)).toBe(null);
});

test("every order sent is heard as it goes, a queued (Shift) one too", async () => {
  // The order flash starts from this hook: it hears what is sent.
  const observation = { own: [own(1, "rifle")], contacts: [] } as unknown as ObservationView;
  const { client, sent } = recordingClient();
  const heard: Order[] = [];
  const hook = renderHook(() => useUnitControl(client, observation, (o) => heard.push(o)));
  act(() => hook.result.current.setSelected([1]));
  const pick = { unit: null, button: "right" as const, ctrl: false, x: 0, y: 0 };
  await act(async () =>
    hook.result.current.onPointer({ ...pick, shift: false, time: 0, ground: [50, 50] }),
  );
  await act(async () =>
    hook.result.current.onPointer({ ...pick, shift: true, time: 5000, ground: [60, 50] }),
  );
  expect(sent.map((s) => s.queued)).toEqual([false, true]);
  expect(heard).toEqual(sent.map((s) => s.order));
});
