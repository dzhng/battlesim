// The player's pointer input at the input seam: `useUnitControl.onPointer`
// with a pick, and the orders it sends to a recording client.
import { act, fireEvent, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { expect, test } from "vitest";
import { UnitCatalog } from "@packages/scene-assets/src/units";
import { SessionCatalogProvider } from "@web/battle/catalog/context";
import { TEST_CATALOG } from "./catalog";
import { contactUnder } from "../src/battle/input/contactPick";
import { useUnitControl } from "../src/battle/input/useUnitControl";
import type { PointerPick } from "../src/battle/input/pointerIntent";
import type { SimClient } from "../src/battle/sim/client";
import type { ContactView, ObservationView, OwnUnitView } from "../src/battle/sim/observation";
import type { Order } from "../src/battle/sim/protocol";

// Distinct types sharing a role make type selection distinguishable from role selection.
const catalog = {
  ...TEST_CATALOG,
  units: new UnitCatalog({
    ...TEST_CATALOG.units.view,
    units: TEST_CATALOG.units.view.units.map((type) =>
      type.id === "test_recon"
        ? { ...type, roles: TEST_CATALOG.units.type("test_rifle").roles }
        : type,
    ),
  }),
};
const wrapper = ({ children }: { children: ReactNode }) => (
  <SessionCatalogProvider catalog={catalog}>{children}</SessionCatalogProvider>
);

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
    own: [own(1, "test_rifle"), own(2, "test_supply")],
    contacts,
  } as unknown as ObservationView;
  const { client, sent } = recordingClient();
  const hook = renderHook(() => useUnitControl(client, observation), { wrapper });
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

test("a building click sends the mixed selection as one queued building intent", async () => {
  const { sent, click } = await control();
  await click({ building: 9, shift: true });
  expect(sent).toEqual([
    {
      order: { kind: "occupy_building", units: [1, 2], building: 9, gesture: 1 },
      queued: true,
    },
  ]);
});

test("attack ground confirms with left-click, preserves selection and queues only armed units", async () => {
  const { hook, sent, click } = await control();
  act(() => hook.result.current.setMode("attack_ground"));
  await click({ button: "left", shift: true, ctrl: true, enemy: 3, contact: 7, building: 9 });
  expect(sent).toEqual([
    {
      order: { kind: "attack", units: [1], target: { kind: "ground", point: [50, 50, 0] } },
      queued: true,
    },
  ]);
  expect(hook.result.current.selected).toEqual([1, 2]);
  expect(hook.result.current.mode).toBe("move");
});

test("right-click cancels attack ground without issuing an order or changing selection", async () => {
  const { hook, sent, click } = await control();
  act(() => hook.result.current.setMode("attack_ground"));
  await click({ ctrl: true, enemy: 3, building: 9 });
  expect(sent).toEqual([]);
  expect(hook.result.current.selected).toEqual([1, 2]);
  expect(hook.result.current.mode).toBe("move");
  await click({});
  expect(sent[0].order).toMatchObject({ kind: "move", units: [1, 2], goal: [50, 50] });
});

test("a right press captured during attack ground stays cancelled after Escape", async () => {
  const { hook, sent } = await control();
  act(() => hook.result.current.setMode("attack_ground"));
  const pick: PointerPick = {
    unit: null,
    button: "right",
    shift: false,
    ctrl: false,
    x: 0,
    y: 0,
    time: 0,
    ground: [50, 50],
  };
  const captured = hook.result.current.intentAt(pick);
  act(() => fireEvent.keyDown(window, { code: "Escape" }));
  await act(async () => hook.result.current.onPointer(pick, captured));
  expect(sent).toEqual([]);
  expect(hook.result.current.selected).toEqual([1, 2]);
  expect(hook.result.current.mode).toBe("move");
});

test("an unarmed enemy click disarms the command but sends no attack", async () => {
  const { hook, sent, click } = await control();
  act(() => {
    hook.result.current.setSelected([2]);
    hook.result.current.setMode("garrison");
  });
  await click({ enemy: 3 });
  expect(hook.result.current.mode).toBe("move");
  expect(sent).toEqual([]);
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
  const observation = { own: [own(1, "test_rifle")], contacts: [] } as unknown as ObservationView;
  const { client, sent } = recordingClient();
  const heard: { order: Order; queued: boolean }[] = [];
  const hook = renderHook(
    () => useUnitControl(client, observation, (order, queued) => heard.push({ order, queued })),
    { wrapper },
  );
  act(() => hook.result.current.setSelected([1]));
  const pick = { unit: null, button: "right" as const, ctrl: false, x: 0, y: 0 };
  await act(async () =>
    hook.result.current.onPointer({ ...pick, shift: false, time: 0, ground: [50, 50] }),
  );
  await act(async () =>
    hook.result.current.onPointer({ ...pick, shift: true, time: 5000, ground: [60, 50] }),
  );
  expect(sent.map((s) => s.queued)).toEqual([false, true]);
  expect(heard).toEqual(sent);
});

test("losing a selected unit leaves survivors commandable without reselecting", async () => {
  const { client, sent } = recordingClient();
  const observation = (ids: number[]) =>
    ({ own: ids.map((id) => own(id, "test_rifle")), contacts: [] }) as unknown as ObservationView;
  const hook = renderHook(({ ids }) => useUnitControl(client, observation(ids)), {
    wrapper,
    initialProps: { ids: [1, 2] },
  });
  act(() => hook.result.current.setSelected([1, 2]));
  hook.rerender({ ids: [1] });
  expect(hook.result.current.selected).toEqual([1]);
  for (const [index, mode] of (["move", "fast_move", "reverse_move"] as const).entries()) {
    act(() => hook.result.current.setMode(mode));
    await act(async () =>
      hook.result.current.onPointer({
        unit: null,
        button: "right",
        shift: false,
        ctrl: false,
        x: 0,
        y: 0,
        time: index * 1000,
        ground: [50, 50],
      }),
    );
  }
  await act(async () => hook.result.current.stop());
  await act(async () => hook.result.current.togglePolicy());
  expect(sent.map(({ order }) => "units" in order && order.units)).toEqual(
    Array.from({ length: 5 }, () => [1]),
  );
  hook.rerender({ ids: [] });
  await act(async () => hook.result.current.stop());
  expect(hook.result.current.selected).toEqual([]);
  expect(sent).toHaveLength(5);
  hook.unmount();
});

test("successive double-clicks widen type to role; external selection and timeout start over", () => {
  const observation = {
    own: [own(1, "test_rifle"), own(2, "test_rifle"), own(3, "test_recon")],
    contacts: [],
  } as unknown as ObservationView;
  const { client } = recordingClient();
  const hook = renderHook(() => useUnitControl(client, observation), { wrapper });
  const pair = (time: number, ctrl = false, unit = 1) => {
    for (const t of [time, time + 100])
      act(() =>
        hook.result.current.onPointer({
          unit,
          button: "left",
          shift: false,
          ctrl,
          x: 0,
          y: 0,
          time: t,
          ground: [0, 0],
        }),
      );
    return hook.result.current.selected;
  };
  expect(pair(0)).toEqual([1, 2]);
  expect(pair(200, false, 2)).toEqual([1, 2, 3]);
  act(() => hook.result.current.setSelected([3]));
  expect(pair(400)).toEqual([1, 2]);
  expect(pair(60_000)).toEqual([1, 2]);
  expect(pair(61_000, true)).toEqual([1, 2, 3]);
  act(() => hook.result.current.selectInRect((u) => u.id === 3, false));
  expect(pair(61_200)).toEqual([1, 2]);
  expect(pair(61_400, false, 3)).toEqual([3]);
  hook.unmount();
});

test("a captured building intent keeps its selection, target and modifiers when release follows a mode change", async () => {
  const { hook, sent } = await control();
  const pick: PointerPick = {
    unit: null,
    button: "right",
    shift: true,
    ctrl: false,
    x: 100,
    y: 100,
    time: 20,
    ground: [50, 50],
    building: 7,
  };
  const held = hook.result.current.intentAt(pick);
  act(() => {
    hook.result.current.setSelected([2]);
    hook.result.current.setMode("fast_move");
  });
  await act(async () =>
    hook.result.current.onPointer({ ...pick, shift: false, building: 9 }, held),
  );
  expect(sent).toEqual([
    { order: { kind: "occupy_building", units: [1, 2], building: 7, gesture: 1 }, queued: true },
  ]);
});

test("a casualty during a captured press leaves the surviving original selection commandable", async () => {
  const { client, sent } = recordingClient();
  const observation = (ids: number[]) =>
    ({ own: ids.map((id) => own(id, "test_rifle")), contacts: [] }) as unknown as ObservationView;
  const hook = renderHook(({ ids }) => useUnitControl(client, observation(ids)), {
    wrapper,
    initialProps: { ids: [1, 2, 3] },
  });
  act(() => hook.result.current.setSelected([1, 2]));
  const pick: PointerPick = {
    unit: null,
    button: "right",
    shift: false,
    ctrl: false,
    x: 0,
    y: 0,
    time: 0,
    ground: [50, 50],
    building: 7,
  };
  const held = hook.result.current.intentAt(pick);
  hook.rerender({ ids: [1, 3] });
  act(() => hook.result.current.setSelected([3]));
  await act(async () => hook.result.current.onPointer(pick, held));
  expect(sent).toEqual([
    { order: { kind: "occupy_building", units: [1], building: 7, gesture: 1 }, queued: false },
  ]);
  hook.unmount();
});

test("a captured contact that expires is refused locally instead of repicking the ground", async () => {
  const { client, sent } = recordingClient();
  const observation = (contact: boolean) =>
    ({
      own: [own(1, "test_rifle")],
      contacts: contact ? [{ id: 7 }] : [],
    }) as unknown as ObservationView;
  const hook = renderHook(({ contact }) => useUnitControl(client, observation(contact)), {
    wrapper,
    initialProps: { contact: true },
  });
  act(() => hook.result.current.setSelected([1]));
  const pick: PointerPick = {
    unit: null,
    button: "right",
    shift: false,
    ctrl: false,
    x: 0,
    y: 0,
    time: 0,
    ground: [50, 50],
    contact: 7,
  };
  const held = hook.result.current.intentAt(pick);
  expect(held.kind).toBe("attack");
  hook.rerender({ contact: false });
  await act(async () => hook.result.current.onPointer({ ...pick, contact: null }, held));
  expect(sent).toEqual([]);
  hook.unmount();
});

test("an old client's delayed acknowledgement cannot repopulate a reset log", async () => {
  let reply!: (ack: { seq: number; applied_tick: number; error: null }) => void;
  const old = {
    command: () =>
      new Promise((resolve) => {
        reply = resolve;
      }),
  } as unknown as SimClient;
  const fresh = recordingClient().client;
  const observation = { own: [own(1, "test_rifle")], contacts: [] } as unknown as ObservationView;
  const hook = renderHook(({ client }) => useUnitControl(client, observation), {
    wrapper,
    initialProps: { client: old },
  });
  let pending!: Promise<unknown>;
  act(() => {
    pending = hook.result.current.issue({ kind: "stop", units: [1] });
  });
  hook.rerender({ client: fresh });
  await act(async () =>
    hook.result.current.issue({ kind: "set_engagement", units: [1], policy: "return_fire_only" }),
  );
  await act(async () => {
    reply({ seq: 99, applied_tick: 1, error: null });
    await pending;
  });
  expect(hook.result.current.acks.map((entry) => entry.order.kind)).toEqual(["set_engagement"]);
  hook.unmount();
});

test("blocked battle input preserves selection and armed mode without issuing shortcuts", async () => {
  const { client, sent } = recordingClient();
  const observation = { own: [own(1, "test_rifle")], contacts: [] } as unknown as ObservationView;
  const hook = renderHook(
    ({ enabled }) => useUnitControl(client, observation, undefined, enabled),
    {
      wrapper,
      initialProps: { enabled: true },
    },
  );
  act(() => {
    hook.result.current.setSelected([1]);
    hook.result.current.setMode("attack_ground");
  });
  hook.rerender({ enabled: false });
  const button = document.body.appendChild(document.createElement("button"));
  await act(async () => {
    fireEvent.keyDown(button, { code: "Backspace" });
    fireEvent.keyDown(button, { code: "Escape" });
    fireEvent.keyDown(button, { code: "Space" });
  });
  expect(sent).toEqual([]);
  expect(hook.result.current.selected).toEqual([1]);
  expect(hook.result.current.mode).toBe("attack_ground");
  expect(hook.result.current.showOrders).toBe(false);
  hook.rerender({ enabled: true });
  await act(async () => fireEvent.keyDown(window, { code: "Backspace" }));
  expect(sent.map(({ order }) => order)).toEqual([{ kind: "stop", units: [1] }]);
  hook.unmount();
  button.remove();
});

test("attack-move sends the entire selection, including unarmed units, by click and keyboard", async () => {
  const { hook, sent, click } = await control();
  await click({ ctrl: true });
  act(() => hook.result.current.setMode("attack_move"));
  await click({ shift: true });
  act(() => hook.result.current.setSelected([2]));
  act(() => fireEvent.keyDown(window, { code: "KeyX" }));
  expect(hook.result.current.mode).toBe("attack_move");
  await click({});
  expect(sent).toEqual([
    { order: { kind: "attack_move", units: [1, 2], goal: [50, 50], gesture: 1 }, queued: false },
    { order: { kind: "attack_move", units: [1, 2], goal: [50, 50], gesture: 2 }, queued: true },
    { order: { kind: "attack_move", units: [2], goal: [50, 50], gesture: 3 }, queued: false },
  ]);
});
