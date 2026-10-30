import { Feed } from "@web/feed";
// The player's pointer input at the input seam: `useUnitControl.onPointer`
// with a pick, and the orders it sends to a recording client.
import { act, fireEvent, render, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { expect, test } from "vitest";
import { UnitCatalog } from "@packages/scene-assets/src/units";
import { SessionCatalogProvider } from "@web/battle/catalog/context";
import { TEST_CATALOG } from "./catalog";
import { useUnitControl } from "../src/battle/input/useUnitControl";
import { CommandBar } from "../src/battle/present/commandBar";
import type { PointerPick } from "../src/battle/input/pointerIntent";
import type { SimClient } from "../src/battle/sim/client";
import type { ContactView, ObservationView, OwnUnitView } from "../src/battle/sim/observationView";
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
      return { seq: sent.length, applied_tick: 0, error: null, refused: [] };
    },
  } as unknown as SimClient;
  return { client, sent };
}

const own = (id: number, kind: string) =>
  ({ id, kind, position: [0, 0, 0], yaw: 0 }) as unknown as OwnUnitView;

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

test("right-clicking an airborne contact offers no attack: area fire cannot reach the air", async () => {
  const { sent, click } = await control();
  await click({ contact: 7, contactAirborne: true });
  expect(sent.map((s) => s.order.kind)).toEqual(["move"]);
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

test("every order sent is heard as it goes, a queued (Shift) one too", async () => {
  // The order flash starts from this hook: it hears what is sent.
  const observation = { own: [own(1, "test_rifle")], contacts: [] } as unknown as ObservationView;
  const { client, sent } = recordingClient();
  const heard: { order: Order; queued: boolean }[] = [];
  const hook = renderHook(
    () =>
      useUnitControl(client, observation, {
        onIssue: (order, queued) => heard.push({ order, queued }),
      }),
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

test("the selection's growth is heard: the units it gained, and the one clicked to gain it", () => {
  // A unit answers when selected: the clicked one, or a box's highest-value one.
  const observation = {
    own: [own(1, "test_rifle"), own(2, "test_rifle"), own(3, "test_supply")],
    contacts: [],
  } as unknown as ObservationView;
  const { client } = recordingClient();
  const heard: { added: number[]; clicked: number | null }[] = [];
  const hook = renderHook(
    () =>
      useUnitControl(client, observation, {
        onSelect: (added, clicked) => heard.push({ added: [...added], clicked }),
      }),
    { wrapper },
  );
  const left = (unit: number | null, shift: boolean, time: number) =>
    act(() =>
      hook.result.current.onPointer({
        unit,
        button: "left",
        shift,
        ctrl: false,
        x: 0,
        y: 0,
        time,
        ground: [0, 0],
      }),
    );
  left(1, false, 0);
  left(2, true, 1000);
  // Shift-clicking a selected unit drops it: nothing was gained.
  left(2, true, 2000);
  act(() => hook.result.current.selectInRect(() => true, false));
  left(null, false, 3000);
  expect(heard).toEqual([
    { added: [1], clicked: 1 },
    { added: [2], clicked: 2 },
    { added: [2, 3], clicked: null },
  ]);
  hook.unmount();
});

test("every order's acknowledgement is heard with its order, a refusal too", async () => {
  // A unit answers an order once the authority accepts it, and says so when it refuses.
  const refusal = { reason: "no_valid_destination" };
  let seq = 0;
  const client = {
    command: async (order: Order) => ({
      seq: ++seq,
      applied_tick: 0,
      error: order.kind === "move" ? refusal : null,
      refused: [],
    }),
  } as unknown as SimClient;
  const observation = { own: [own(1, "test_rifle")], contacts: [] } as unknown as ObservationView;
  const heard: { order: string; seq: number; error: unknown }[] = [];
  const hook = renderHook(
    () =>
      useUnitControl(client, observation, {
        onAck: (order, ack) => heard.push({ order: order.kind, seq: ack.seq, error: ack.error }),
      }),
    { wrapper },
  );
  act(() => hook.result.current.setSelected([1]));
  await act(async () => hook.result.current.stop());
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
    }),
  );
  expect(heard).toEqual([
    { order: "stop", seq: 1, error: null },
    { order: "move", seq: 2, error: refusal },
  ]);
  hook.unmount();
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

test("the fire-policy key orders the selection it judged, even before a re-render", async () => {
  const { client, sent } = recordingClient();
  const observation = {
    own: [own(1, "test_rifle"), own(2, "test_rifle")],
    contacts: [],
  } as unknown as ObservationView;
  const hook = renderHook(() => useUnitControl(client, observation), { wrapper });
  act(() => hook.result.current.setSelected([1, 2]));
  // A key handler bound before the last render still holds the earlier toggle.
  const toggle = hook.result.current.togglePolicy;
  act(() => hook.result.current.setSelected([2]));
  await act(async () => toggle());
  expect(sent.map(({ order }) => "units" in order && order.units)).toEqual([[2]]);
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
  let reply!: (ack: { seq: number; applied_tick: number; error: null; refused: [] }) => void;
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
    reply({ seq: 99, applied_tick: 1, error: null, refused: [] });
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
  act(() => fireEvent.keyDown(window, { code: "KeyC" }));
  expect(hook.result.current.mode).toBe("attack_move");
  await click({});
  expect(sent).toEqual([
    { order: { kind: "attack_move", units: [1, 2], goal: [50, 50], gesture: 1 }, queued: false },
    { order: { kind: "attack_move", units: [1, 2], goal: [50, 50], gesture: 2 }, queued: true },
    { order: { kind: "attack_move", units: [2], goal: [50, 50], gesture: 3 }, queued: false },
  ]);
});

/** A 2v2's blue side as `blue-1` sees it: by default units 1 and 3 its
 *  own, unit 2 its ally `blue-2`'s, all of one type. Each of `units` is
 *  an id, a type and its owner's index (0 `blue-1`, 1 `blue-2`). */
function teamControl(
  units: [number, string, number][] = [
    [1, "test_rifle", 0],
    [2, "test_rifle", 1],
    [3, "test_rifle", 0],
  ],
) {
  const observation = {
    own: units.map(([id, kind, owner]) => ({ ...own(id, kind), owner })),
    contacts: [],
    skirmish: {
      players: [
        { index: 0, id: "blue-1", controller: "human" },
        { index: 1, id: "blue-2", controller: "active_ai" },
      ],
    },
  } as unknown as ObservationView;
  const { client, sent } = recordingClient();
  const hook = renderHook(() => useUnitControl(client, observation, undefined, true, "blue-1"), {
    wrapper,
  });
  const left = (unit: number | null, time: number, shift = false) =>
    act(() =>
      hook.result.current.onPointer({
        unit,
        button: "left",
        shift,
        ctrl: false,
        x: 0,
        y: 0,
        time,
        ground: [0, 0],
      }),
    );
  const right = (time: number, unit: number | null = null) =>
    act(async () =>
      hook.result.current.onPointer({
        unit,
        button: "right",
        shift: false,
        ctrl: false,
        x: 0,
        y: 0,
        time,
        ground: [50, 50],
      }),
    );
  return { hook, sent, left, right };
}

test("a box or a double-click over your units and your ally's selects only yours", () => {
  const { hook, left } = teamControl();
  act(() => hook.result.current.selectInRect(() => true, false));
  expect(hook.result.current.selected).toEqual([1, 3]);
  // Select similar: every unit of its type that is yours.
  left(1, 1000);
  left(1, 1100);
  expect(hook.result.current.selected).toEqual([1, 3]);
  hook.unmount();
});

test("an ally can be selected to read its panel, but no command leaves with its unit", async () => {
  const { hook, sent, left, right } = teamControl();
  // Clicking an ally selects it alone: nothing in the selection takes orders.
  left(1, 0);
  left(2, 5000, true);
  expect(hook.result.current.selected).toEqual([2]);
  expect(hook.result.current.selectedUnits).toEqual([]);
  await right(6000);
  await act(async () => hook.result.current.stop());
  await act(async () => hook.result.current.togglePolicy());
  act(() => {
    fireEvent.keyDown(window, { code: "KeyX", key: "x" });
  });
  expect(hook.result.current.mode).toBe("move");
  expect(sent).toEqual([]);
  // Your unit clicked next replaces the ally; an ally given from outside
  // (the army row, a lab) never reaches an order.
  left(3, 7000, true);
  expect(hook.result.current.selected).toEqual([3]);
  act(() => hook.result.current.setSelected([1, 2]));
  await right(8000);
  await act(async () => hook.result.current.stop());
  expect(sent.map(({ order }) => "units" in order && order.units)).toEqual([[1], [1]]);
  hook.unmount();
});

test("a right-click boards or collects only your own units, never an ally's", async () => {
  const { hook, sent, right } = teamControl([
    [1, "test_rifle", 0],
    [3, "test_apc", 0],
    [4, "test_apc", 1],
    [5, "test_rifle", 1],
  ]);
  // Your squad on your ally's carrier, and your carrier on your ally's
  // squad: each click falls through to the ground under it.
  act(() => hook.result.current.setSelected([1]));
  await right(0, 4);
  act(() => hook.result.current.setSelected([3]));
  await right(1000, 5);
  // Your own carrier and squad still take the click.
  act(() => hook.result.current.setSelected([1]));
  await right(2000, 3);
  act(() => hook.result.current.setSelected([3]));
  await right(3000, 1);
  expect(sent.map(({ order }) => order.kind)).toEqual(["move", "move", "board", "collect"]);
  hook.unmount();
});

/** A rifle squad held by carrier 4, boarding or aboard. */
const rider = (id: number, phase: "entering" | "inside") =>
  ({
    ...own(id, "test_rifle"),
    hold: { host: { kind: "carrier", id: 4 }, phase, progress: phase === "inside" ? 1 : 0.5 },
  }) as unknown as OwnUnitView;

test("a squad a carrier holds, boarding or aboard, is selected by no click, box or double-click", () => {
  const observation = {
    own: [own(1, "test_rifle"), rider(2, "inside"), rider(3, "entering"), own(4, "test_apc")],
    contacts: [],
  } as unknown as ObservationView;
  const { client } = recordingClient();
  const heard: number[][] = [];
  const hook = renderHook(
    () => useUnitControl(client, observation, { onSelect: (added) => heard.push([...added]) }),
    { wrapper },
  );
  const left = (unit: number, time: number, shift = false) =>
    act(() =>
      hook.result.current.onPointer({
        unit,
        button: "left",
        shift,
        ctrl: false,
        x: 0,
        y: 0,
        time,
        ground: [0, 0],
      }),
    );
  left(2, 0);
  expect(hook.result.current.selected).toEqual([]);
  left(3, 1000, true);
  expect(hook.result.current.selected).toEqual([]);
  act(() => hook.result.current.selectInRect(() => true, false));
  expect(hook.result.current.selected).toEqual([1, 4]);
  // A double-click selects the rifle squads on the field alone.
  left(1, 5000);
  left(1, 5100);
  expect(hook.result.current.selected).toEqual([1]);
  act(() => hook.result.current.setSelected([1, 2, 3]));
  expect(hook.result.current.selected).toEqual([1]);
  expect(heard.flat().filter((id) => id === 2 || id === 3)).toEqual([]);
  hook.unmount();
});

test("a selected squad drops out of the selection as it boards, and orders go to the rest", async () => {
  const { client, sent } = recordingClient();
  const observation = (boarding: boolean) =>
    ({
      own: [own(1, "test_rifle"), boarding ? rider(2, "entering") : own(2, "test_rifle")],
      contacts: [],
    }) as unknown as ObservationView;
  const hook = renderHook(({ boarding }) => useUnitControl(client, observation(boarding)), {
    wrapper,
    initialProps: { boarding: false },
  });
  act(() => hook.result.current.setSelected([1, 2]));
  hook.rerender({ boarding: true });
  expect(hook.result.current.selected).toEqual([1]);
  await act(async () => hook.result.current.stop());
  expect(sent.map(({ order }) => "units" in order && order.units)).toEqual([[1]]);
  hook.unmount();
});

/** A rifle squad (1), a supply truck (2), an APC (4) and an AT team (5) on
 *  the field; with `riders`, an APC (6) carrying rifle squad 7. */
async function carriers(riders = false) {
  const observation = {
    own: [
      own(1, "test_rifle"),
      own(2, "test_supply"),
      own(4, "test_apc"),
      own(5, "test_at"),
      ...(riders
        ? [
            own(6, "test_apc"),
            {
              ...own(7, "test_rifle"),
              hold: { host: { kind: "carrier", id: 6 }, phase: "inside", progress: 1 },
            } as unknown as OwnUnitView,
          ]
        : []),
    ],
    contacts: [],
  } as unknown as ObservationView;
  const { client, sent } = recordingClient();
  const hook = renderHook(() => useUnitControl(client, observation), { wrapper });
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
  const key = async (code: string, shiftKey = false) =>
    act(async () => fireEvent.keyDown(window, { code, shiftKey }));
  return { hook, sent, click, key, observation };
}

test("right-clicking an own carrier boards the selected squads; right-clicking an own squad sends the selected carriers to collect it; Shift queues", async () => {
  const { hook, sent, click } = await carriers();
  act(() => hook.result.current.setSelected([1, 2]));
  await click({ unit: 4 });
  await click({ unit: 4, shift: true });
  act(() => hook.result.current.setSelected([4, 2]));
  await click({ unit: 5 });
  await click({ unit: 5, shift: true });
  expect(sent).toEqual([
    { order: { kind: "board", units: [1], carrier: 4, gesture: 1 }, queued: false },
    { order: { kind: "board", units: [1], carrier: 4, gesture: 2 }, queued: true },
    { order: { kind: "collect", units: [4], squads: [5], gesture: 3 }, queued: false },
    { order: { kind: "collect", units: [4], squads: [5], gesture: 4 }, queued: true },
  ]);
});

test("a squad that boards another carrier while a right-press is held is left out of the release's order", async () => {
  const { client, sent } = recordingClient();
  const squad = (id: number, aboard: boolean) =>
    aboard
      ? ({
          ...own(id, "test_rifle"),
          hold: { host: { kind: "carrier", id: 6 }, phase: "entering", progress: 0.5 },
        } as unknown as OwnUnitView)
      : own(id, "test_rifle");
  // Squads 1 and 3, APCs 4 and 6; with `taken`, APC 6 has begun to take squad 3.
  const observation = (taken: boolean) =>
    ({
      own: [squad(1, false), squad(3, taken), own(4, "test_apc"), own(6, "test_apc")],
      contacts: [],
    }) as unknown as ObservationView;
  const hook = renderHook(({ taken }) => useUnitControl(client, observation(taken)), {
    wrapper,
    initialProps: { taken: false },
  });
  const press = (unit: number): PointerPick => ({
    unit,
    button: "right",
    shift: false,
    ctrl: false,
    x: 0,
    y: 0,
    time: 0,
    ground: [50, 50],
  });
  act(() => hook.result.current.setSelected([1, 3]));
  const board = hook.result.current.intentAt(press(4));
  act(() => hook.result.current.setSelected([4]));
  const collect = hook.result.current.intentAt(press(3));
  hook.rerender({ taken: true });
  await act(async () => hook.result.current.onPointer(press(4), board));
  await act(async () => hook.result.current.onPointer(press(3), collect));
  expect(sent).toEqual([
    { order: { kind: "board", units: [1], carrier: 4, gesture: 1 }, queued: false },
  ]);
  hook.unmount();
});

test("U then a ground click unloads there, only the carriers holding passengers; Shift queues it; right-click cancels", async () => {
  const { hook, sent, click, key } = await carriers(true);
  act(() => hook.result.current.setSelected([1, 2, 4, 6]));
  await key("KeyU");
  expect(hook.result.current.mode).toBe("unload");
  // The left-click confirms rather than selects, whatever is under it.
  await click({ button: "left", unit: 1 });
  expect(hook.result.current.mode).toBe("move");
  expect(hook.result.current.selected).toEqual([1, 2, 4, 6]);
  await key("KeyU");
  await click({ button: "left", shift: true, ground: [70, 20] });
  await key("KeyU");
  await click({});
  expect(hook.result.current.mode).toBe("move");
  expect(sent).toEqual([
    {
      order: { kind: "unload", units: [6], goal: [50, 50], route: "shortest", gesture: 1 },
      queued: false,
    },
    {
      order: { kind: "unload", units: [6], goal: [70, 20], route: "shortest", gesture: 2 },
      queued: true,
    },
  ]);
});

test("U pressed again while armed unloads in place at once, replacing the queue", async () => {
  const { hook, sent, key } = await carriers(true);
  act(() => hook.result.current.setSelected([1, 6]));
  await key("KeyU");
  await key("KeyU");
  expect(hook.result.current.mode).toBe("move");
  expect(sent).toEqual([
    {
      order: { kind: "unload", units: [6], goal: null, route: "shortest", gesture: 1 },
      queued: false,
    },
  ]);
});

test("Unload is unavailable, by key and on the bar, without a carrier holding passengers", async () => {
  const { hook, sent, key, observation } = await carriers(true);
  act(() => hook.result.current.setSelected([1, 4]));
  await key("KeyU");
  expect(hook.result.current.mode).toBe("move");
  // Armed, then the loaded carrier leaves the selection: U disarms.
  act(() => hook.result.current.setSelected([1, 6]));
  await key("KeyU");
  expect(hook.result.current.mode).toBe("unload");
  act(() => hook.result.current.setSelected([1, 4]));
  await key("KeyU");
  expect(hook.result.current.mode).toBe("move");
  const bar = () => (
    <CommandBar state={new Feed({ control: hook.result.current, own: observation.own })} />
  );
  const view = render(bar(), { wrapper });
  const unload = () => view.getByRole("button", { name: /^Unload/ });
  expect(unload().hasAttribute("disabled")).toBe(true);
  // With the loaded carrier selected the button arms Unload, and a second
  // press unloads where it stands, as the key does.
  act(() => hook.result.current.setSelected([1, 4, 6]));
  view.rerender(bar());
  expect(unload().hasAttribute("disabled")).toBe(false);
  await act(async () => fireEvent.click(unload()));
  expect(hook.result.current.mode).toBe("unload");
  view.rerender(bar());
  expect(unload().getAttribute("aria-pressed")).toBe("true");
  await act(async () => fireEvent.click(unload()));
  expect(sent.map(({ order }) => order)).toEqual([
    { kind: "unload", units: [6], goal: null, route: "shortest", gesture: 1 },
  ]);
  view.unmount();
});
