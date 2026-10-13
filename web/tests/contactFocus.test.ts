import { expect, test } from "vitest";
import {
  contactFocusTargets,
  focusStep,
  type FocusContact,
  type FocusTarget,
} from "../src/battle/input/contactFocus";

/** The evidence a side holds, newest first: ids only; the ground point the
 *  camera moves to is irrelevant to which one a step lands on. */
const targets = (...ids: number[]): FocusTarget[] =>
  ids.map((id) => ({ id, target: [id, id], evidenceTick: id }));

test("the camera's targets are the live contacts, newest evidence first", () => {
  const contact = (over: Partial<FocusContact> & { id: number }): FocusContact => ({
    center: [over.id, over.id, 0],
    evidenceTick: 0,
    retiring: false,
    ...over,
  });
  expect(
    contactFocusTargets([
      contact({ id: 1, evidenceTick: 40 }),
      contact({ id: 2, evidenceTick: 90 }),
      contact({ id: 3, evidenceTick: 40 }),
      contact({ id: 4, evidenceTick: 200, retiring: true }),
    ]),
  ).toEqual([
    { id: 2, target: [2, 2], evidenceTick: 90 },
    { id: 3, target: [3, 3], evidenceTick: 40 },
    { id: 1, target: [1, 1], evidenceTick: 40 },
  ]);
});

test("the first jump lands on the newest evidence, whichever way it steps", () => {
  expect(focusStep(targets(7, 4, 2), null, false)).toBe(0);
  expect(focusStep(targets(7, 4, 2), null, true)).toBe(0);
});

test("N walks older evidence and wraps; Shift+N walks back newer", () => {
  const list = targets(7, 4, 2);
  expect(focusStep(list, list[0], false)).toBe(1);
  expect(focusStep(list, list[0], true)).toBe(2);
  expect(focusStep(list, list[1], false)).toBe(2);
  expect(focusStep(list, list[2], false)).toBe(0);
  expect(focusStep(list, list[2], true)).toBe(1);
  expect(focusStep(list, list[1], true)).toBe(0);
});

test("evidence that has expired starts the next jump over at the newest", () => {
  expect(focusStep(targets(4, 2), { id: 7, evidenceTick: 7 }, false)).toBe(0);
  expect(focusStep([], { id: 7, evidenceTick: 7 }, false)).toBe(-1);
  expect(focusStep([], null, true)).toBe(-1);
});
