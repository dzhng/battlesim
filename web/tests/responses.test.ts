// @vitest-environment node
// What a unit says back to its owner (R1–R12): which event asks which line,
// who in a group answers, and when a move counts as an arrival.
import { expect, test } from "vitest";
import {
  Arrivals,
  responseFor,
  type ResponseUnit,
} from "@packages/battle-audio/src/voices/responses";

const unit = (id: number, value: number, extra: Partial<ResponseUnit> = {}): ResponseUnit => ({
  id,
  set: `set_${id}`,
  value,
  stressed: false,
  ...extra,
});
const TANK = unit(1, 300);
const RIFLES = unit(2, 90);
const TRUCK = unit(3, 40);

test("each accepted order is answered with its own line, as a response", () => {
  const answer = (order: { kind: string; policy?: string }) =>
    responseFor({ kind: "accepted", order, units: [TANK], id: 7 })?.trigger ?? null;
  expect(
    [
      { kind: "move" },
      { kind: "attack" },
      { kind: "attack_move" },
      { kind: "stop" },
      { kind: "set_engagement", policy: "fire_at_will" },
      { kind: "set_engagement", policy: "return_fire_only" },
      { kind: "garrison" },
      { kind: "occupy_building" },
      { kind: "exit_building" },
      { kind: "board" },
      { kind: "collect" },
      { kind: "unload" },
      { kind: "refund" },
    ].map(answer),
  ).toEqual([
    "move",
    "attack",
    "attack_move",
    "stop",
    "weapons_free",
    "hold_fire",
    "enter_building",
    "enter_building",
    "leave_building",
    "board",
    "board",
    "unload",
    "withdraw",
  ]);
  // Orders no unit answers: the match's own, and a gesture's upgrade.
  for (const kind of ["ready", "confirm_purchase", "cancel_pending", "upgrade_move"])
    expect(answer({ kind })).toBeNull();
  expect(responseFor({ kind: "accepted", order: { kind: "move" }, units: [TANK], id: 7 })).toEqual({
    tier: "response",
    speaker: "own/1",
    set: "set_1",
    trigger: "move",
    intensity: "steady",
    groupKey: "order/7",
  });
});

test("a group answers once: the lead when the order includes it, else its highest-value unit", () => {
  const order = (units: ResponseUnit[], lead?: number) =>
    responseFor({ kind: "accepted", order: { kind: "move" }, units, lead, id: 3 })?.speaker;
  expect(order([TRUCK, RIFLES, TANK])).toBe("own/1");
  expect(order([TRUCK, RIFLES, TANK], 3)).toBe("own/3");
  // The lead isn't in this order (an attack goes to the armed units only).
  expect(order([RIFLES, TANK], 3)).toBe("own/1");
  // Equal value: the lower id, so the same unit always answers.
  expect(order([unit(9, 90), unit(4, 90)])).toBe("own/4");
  expect(order([])).toBeUndefined();
});

test("a selection is answered by the unit clicked, or a box's highest-value unit", () => {
  const select = (units: ResponseUnit[], lead?: number) =>
    responseFor({ kind: "selected", units, lead });
  expect(select([RIFLES, TANK, TRUCK])).toMatchObject({
    trigger: "selected",
    speaker: "own/1",
    tier: "response",
  });
  expect(select([RIFLES, TANK, TRUCK], 2)?.speaker).toBe("own/2");
  // Selections in quick succession are one answer.
  expect(select([TRUCK])?.groupKey).toBe(select([TANK])?.groupKey);
});

test("a stressed unit answers in its stressed voice", () => {
  const hurt = unit(5, 300, { stressed: true });
  expect(responseFor({ kind: "selected", units: [hurt] })?.intensity).toBe("stressed");
});

test("a refusal the player caused is answered; a refusal of the match or the input path is not", () => {
  const refused = (reason: string) =>
    responseFor({ kind: "refused", reason, units: [TANK], group: "order/4" })?.trigger ?? null;
  for (const reason of [
    "no_valid_destination",
    "out_of_bounds",
    "unknown_target",
    "air_contact",
    "not_infantry",
    "not_a_building",
    "capacity_full",
    "one_squad_per_building",
    "building_occupied",
    "not_a_carrier",
    "aboard",
    "wrong_player",
  ])
    expect(refused(reason)).toBe("refused");
  expect(refused("no_room_to_unload")).toBe("no_room_to_unload");
  for (const reason of [
    "rate_limited",
    "out_of_sequence",
    "replay_in_progress",
    "match_finished",
    "insufficient_credits",
    "destroyed",
    "not_own_unit",
  ])
    expect(refused(reason)).toBeNull();
});

test("an arrival is the unit's own report, keyed to its journey", () => {
  expect(responseFor({ kind: "arrived", unit: TANK, group: "order/9" })).toEqual({
    tier: "response",
    speaker: "own/1",
    set: "set_1",
    trigger: "arrived",
    intensity: "steady",
    groupKey: "arrived/order/9",
  });
});

/** Own units at `time`: each id under way or not. */
const at = (arrivals: Arrivals, time: number, moving: Record<number, boolean>) =>
  arrivals.observe(
    Object.entries(moving).map(([id, under]) => ({ id: Number(id), moving: under })),
    time,
  );

test("a unit arrives when its ordered move ends after a real journey", () => {
  const arrivals = new Arrivals(3);
  at(arrivals, 0, { 1: false });
  arrivals.ordered({ kind: "move", units: [1] }, false, "order/1");
  // The order is applied a tick later: the unit isn't under way yet.
  expect(at(arrivals, 0.1, { 1: false })).toEqual([]);
  expect(at(arrivals, 0.2, { 1: true })).toEqual([]);
  expect(at(arrivals, 5, { 1: true })).toEqual([]);
  expect(at(arrivals, 6, { 1: false })).toEqual([{ id: 1, group: "order/1" }]);
  expect(at(arrivals, 7, { 1: false })).toEqual([]);
});

test("a short hop is not an arrival", () => {
  const arrivals = new Arrivals(3);
  at(arrivals, 0, { 1: false });
  arrivals.ordered({ kind: "move", units: [1] }, false, "order/1");
  at(arrivals, 0.1, { 1: true });
  expect(at(arrivals, 2, { 1: false })).toEqual([]);
});

test("a group's journey is reported once, by the first unit to arrive", () => {
  const arrivals = new Arrivals(3);
  at(arrivals, 0, { 1: false, 2: false });
  arrivals.ordered({ kind: "move", units: [1, 2] }, false, "order/1");
  at(arrivals, 0.1, { 1: true, 2: true });
  expect(at(arrivals, 8, { 1: true, 2: false })).toEqual([{ id: 2, group: "order/1" }]);
  expect(at(arrivals, 9, { 1: false, 2: false })).toEqual([]);
});

test("a purchased unit reports when it lands where it was sent", () => {
  const arrivals = new Arrivals(3);
  at(arrivals, 0, { 1: false });
  // It enters the field already driving to its destination.
  at(arrivals, 1, { 1: false, 7: true });
  expect(at(arrivals, 20, { 1: false, 7: false })).toEqual([{ id: 7, group: "unit/7" }]);
});

test("halting, pursuing and leaving the field are not arrivals", () => {
  const arrivals = new Arrivals(3);
  at(arrivals, 0, { 1: false, 2: false, 3: false });
  arrivals.ordered({ kind: "move", units: [1, 2, 3] }, false, "order/1");
  at(arrivals, 0.1, { 1: true, 2: true, 3: true });
  // Stopped; told to attack (it chases, then stops in range); withdrawn.
  arrivals.ordered({ kind: "stop", units: [1] }, false, "order/2");
  arrivals.ordered({ kind: "attack", units: [2] }, false, "order/3");
  arrivals.ordered({ kind: "refund", units: [3] }, false, "order/4");
  expect(at(arrivals, 10, { 1: false, 2: true, 3: true })).toEqual([]);
  expect(at(arrivals, 20, { 1: false, 2: false })).toEqual([]);
  // A unit already on the field that starts moving by itself (a pursuit) reports nothing.
  expect(at(arrivals, 21, { 1: true, 2: false })).toEqual([]);
  expect(at(arrivals, 30, { 1: false, 2: false })).toEqual([]);
});

test("a fire-policy change or a queued stop leaves the journey running", () => {
  const arrivals = new Arrivals(3);
  at(arrivals, 0, { 1: false });
  arrivals.ordered({ kind: "move", units: [1] }, false, "order/1");
  at(arrivals, 0.1, { 1: true });
  arrivals.ordered({ kind: "set_engagement", units: [1] }, false, "order/2");
  arrivals.ordered({ kind: "stop", units: [1] }, true, "order/3");
  expect(at(arrivals, 6, { 1: false })).toEqual([{ id: 1, group: "order/1" }]);
});
