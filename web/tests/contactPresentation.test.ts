// @vitest-environment node
// The contacts a side's cards and glyphs show (`ContactPresentation`): what
// its observation reports now, whole, and what it reported and has lost,
// remembered and fading out over the fade on battle time, then gone. A
// contact the side never reported is never shown. The capture films this
// same presentation; nothing else decides what it remembers.
import { expect, test } from "vitest";
import { ContactPresentation } from "@web/battle/present/contactPresentation";
import type { ContactView } from "@web/battle/sim/observationView";

const TICK_HZ = 10;
const FADE_S = 2;
const contact = (id: number) => ({ id }) as ContactView;
const shown = (p: ReturnType<ContactPresentation["update"]>) =>
  p.map(({ id, opacity, retiring }) => ({ id, opacity, retiring }));

test("a lost contact is remembered, fading over the fade, then gone", () => {
  const presented = new ContactPresentation(TICK_HZ, FADE_S);
  expect(shown(presented.update([contact(2), contact(5)], 100))).toEqual([
    { id: 2, opacity: 1, retiring: false },
    { id: 5, opacity: 1, retiring: false },
  ]);
  // Contact 5 is lost at tick 110: whole as it goes, half way at a second.
  expect(shown(presented.update([contact(2)], 110))).toContainEqual({
    id: 5,
    opacity: 1,
    retiring: true,
  });
  expect(shown(presented.update([contact(2)], 120))).toContainEqual({
    id: 5,
    opacity: 0.5,
    retiring: true,
  });
  // Gone at the fade's end, and not brought back by later ticks.
  expect(shown(presented.update([contact(2)], 130)).map((c) => c.id)).toEqual([2]);
  expect(shown(presented.update([contact(2)], 140)).map((c) => c.id)).toEqual([2]);
});

test("a contact reported again is whole again", () => {
  const presented = new ContactPresentation(TICK_HZ, FADE_S);
  presented.update([contact(5)], 0);
  presented.update([], 10);
  expect(shown(presented.update([contact(5)], 15))).toEqual([
    { id: 5, opacity: 1, retiring: false },
  ]);
});

test("only what the side reported is shown, and a rewound battle forgets", () => {
  const presented = new ContactPresentation(TICK_HZ, FADE_S);
  // A hidden enemy is in no report: it has no contact to show.
  expect(presented.update([], 0)).toEqual([]);
  presented.update([contact(7)], 10);
  presented.update([], 11);
  expect(presented.update([], 5)).toEqual([]);
});
