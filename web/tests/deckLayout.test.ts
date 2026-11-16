import { expect, test } from "vitest";
import { deckSlots } from "../src/battle/present/deckLayout";

// Two rows, filled top to bottom: group 1 takes two whole columns; group 2's
// three cards leave the foot of its second column to the next card, an L.
//   column:  0    1    2    3    4
//   row 0:   1a   1c   2a   2c   y
//   row 1:   1b   1d   2b   x
test("a group's bracket edges every side whose neighbour is outside it, filling top to bottom then across", () => {
  const slots = deckSlots(["1", "1", "1", "1", "2", "2", "2", null, null], 2);
  expect(slots.map((s) => [s.row, s.column])).toEqual([
    [0, 0],
    [1, 0],
    [0, 1],
    [1, 1],
    [0, 2],
    [1, 2],
    [0, 3],
    [1, 3],
    [0, 4],
  ]);
  expect(slots.map((s) => s.sides)).toEqual([
    ["top", "left"],
    ["bottom", "left"],
    ["top", "right"],
    ["right", "bottom"],
    ["top", "left"],
    ["right", "bottom", "left"],
    ["top", "right", "bottom"],
    [],
    [],
  ]);
  // The key sits on a group's first card, the × on the last card of its top row.
  expect(slots.flatMap((s, i) => (s.keyTab ? [i] : []))).toEqual([0, 4]);
  expect(slots.flatMap((s, i) => (s.deleteTab ? [i] : []))).toEqual([2, 6]);
});

test("one row is a plain run: each group closes at its ends", () => {
  const slots = deckSlots(["3", "3", null, "4"], 1);
  expect(slots.map((s) => s.sides)).toEqual([
    ["top", "bottom", "left"],
    ["top", "right", "bottom"],
    [],
    ["top", "right", "bottom", "left"],
  ]);
  expect(slots.flatMap((s, i) => (s.deleteTab ? [i] : []))).toEqual([1, 3]);
});
