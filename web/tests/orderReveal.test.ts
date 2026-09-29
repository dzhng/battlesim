// Which own units' order marks show, and how strongly: Space held shows
// every unit's; an order just issued shows its units' for a moment, then
// fades; nothing else (a selection alone) shows any.
import { expect, test } from "vitest";
import { OrderReveal } from "../src/battle/present/orderReveal";
import type { Order } from "../src/battle/sim/protocol";

const FLASH = { hold_s: 1, fade_s: 0.5 };
const OWN = [{ id: 1 }, { id: 2 }, { id: 3 }];
const move = (units: number[]): Order => ({
  kind: "move",
  units,
  gesture: 1,
  goal: [0, 0],
  route: "shortest",
});

test("without Space or a fresh order no unit's order marks show", () => {
  expect(new OrderReveal(FLASH).at(10, false, OWN).size).toBe(0);
});

test("Space held shows every own unit's order marks in full", () => {
  const reveal = new OrderReveal(FLASH);
  reveal.noteOrder(move([2]), 9);
  expect([...reveal.at(10, true, OWN)]).toEqual([
    [1, 1],
    [2, 1],
    [3, 1],
  ]);
});

test("an order's units show in full for the hold, then fade out over the fade", () => {
  const reveal = new OrderReveal(FLASH);
  reveal.noteOrder(move([1, 3]), 20);
  const at = (since: number) => [...reveal.at(20 + since, false, OWN)];
  expect(at(0)).toEqual([
    [1, 1],
    [3, 1],
  ]);
  expect(at(0.99)).toEqual([
    [1, 1],
    [3, 1],
  ]);
  const half = at(1.25);
  expect(half.map(([id]) => id)).toEqual([1, 3]);
  for (const [, opacity] of half) expect(opacity).toBeCloseTo(0.5, 1);
  expect(at(1.5)).toEqual([]);
  expect(at(5)).toEqual([]);
});

test("the fade only ever falls: every later moment is at most as strong", () => {
  const reveal = new OrderReveal(FLASH);
  reveal.noteOrder(move([1]), 0);
  let last = 1;
  for (let t = 0; t <= 2; t += 1 / 60) {
    const opacity = reveal.at(t, false, OWN).get(1) ?? 0;
    expect(opacity).toBeLessThanOrEqual(last);
    last = opacity;
  }
  expect(last).toBe(0);
});

test("a new order restarts its units' flash", () => {
  const reveal = new OrderReveal(FLASH);
  reveal.noteOrder(move([1]), 0);
  reveal.noteOrder(move([1]), 1.2);
  expect(reveal.at(2, false, OWN).get(1)).toBe(1);
});

test("an order that sets a unit's fire policy or deployment flashes nothing", () => {
  const reveal = new OrderReveal(FLASH);
  reveal.noteOrder({ kind: "set_engagement", units: [1], policy: "fire_at_will" }, 0);
  reveal.noteOrder({ kind: "set_deployment", units: [2], deployed: true }, 0);
  expect(reveal.at(0, false, OWN).size).toBe(0);
});

test("clearing (a new battle) forgets every flash", () => {
  const reveal = new OrderReveal(FLASH);
  reveal.noteOrder(move([1]), 5);
  reveal.clear();
  expect(reveal.at(5, false, OWN).size).toBe(0);
});
