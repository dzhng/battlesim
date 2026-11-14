// Armed commands at the input seam: a left-click confirms one, a right-click
// cancels it, and an armed command reaches only the units that can carry it.
import { act, fireEvent } from "@testing-library/react";
import { expect, test } from "vitest";
import type { PointerPick } from "../src/battle/input/pointerIntent";
import { control } from "./unitControlHarness";

test("every armed command confirms with left-click and cancels with right-click", async () => {
  const expected = {
    attack_move: { kind: "attack_move", goal: [50, 50] },
    reverse_move: { kind: "move", goal: [50, 50], direction: "reverse" },
    fast_move: { kind: "move", goal: [50, 50], route: "fastest" },
    attack_ground: { kind: "attack", target: { kind: "ground", point: [50, 50, 0] } },
  } as const;
  for (const [mode, order] of Object.entries(expected)) {
    const { hook, sent, click } = await control();
    act(() => hook.result.current.setMode(mode as keyof typeof expected));
    await click({ button: "left" });
    expect(
      sent.map((s) => s.order),
      mode,
    ).toEqual([expect.objectContaining(order)]);
    expect(hook.result.current.selected, mode).toEqual([1, 2]);
    expect(hook.result.current.mode, mode).toBe("move");

    act(() => hook.result.current.setMode(mode as keyof typeof expected));
    await click({ button: "right" });
    expect(sent, mode).toHaveLength(1);
    expect(hook.result.current.selected, mode).toEqual([1, 2]);
    expect(hook.result.current.mode, mode).toBe("move");
  }
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
    hook.result.current.setMode("reverse_move");
  });
  await click({ button: "left", enemy: 3 });
  expect(hook.result.current.mode).toBe("move");
  expect(sent).toEqual([]);
});

test("attack-move sends the entire selection, including unarmed units, by click and keyboard", async () => {
  const { hook, sent, click } = await control();
  await click({ ctrl: true });
  act(() => hook.result.current.setMode("attack_move"));
  await click({ button: "left", shift: true });
  act(() => hook.result.current.setSelected([2]));
  act(() => fireEvent.keyDown(window, { code: "KeyC" }));
  expect(hook.result.current.mode).toBe("attack_move");
  await click({ button: "left" });
  expect(sent).toEqual([
    { order: { kind: "attack_move", units: [1, 2], goal: [50, 50], gesture: 1 }, queued: false },
    { order: { kind: "attack_move", units: [1, 2], goal: [50, 50], gesture: 2 }, queued: true },
    { order: { kind: "attack_move", units: [2], goal: [50, 50], gesture: 3 }, queued: false },
  ]);
});
