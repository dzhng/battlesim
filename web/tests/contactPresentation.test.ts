// @vitest-environment node
import { expect, test } from "vitest";
import { ContactPresentation } from "../src/battle/present/contactPresentation";
import type { ContactView } from "../src/battle/sim/observation";

const contact: ContactView = {
  id: 7,
  source: "last_seen",
  center: [100, 200, 0],
  layer: "ground",
  aloft: false,
  radius: 20,
  evidenceTick: 0,
  expiresTick: 300,
  kind: "test_tank",
  heard: [],
};

test("every removed report and label fades for three seconds, even at expiry", () => {
  for (const removedAt of [31, 299, 301]) {
    const shown = new ContactPresentation(30, 3);
    expect(shown.update([contact], removedAt - 1)[0].opacity).toBe(1);
    const removed = shown.update([], removedAt)[0];
    expect(removed.opacity).toBe(1);
    expect(removed.retiring).toBe(true);
    expect(removed.center).toEqual(contact.center);
    expect(shown.update([], removedAt + 45)[0].opacity).toBeCloseTo(0.5);
    expect(shown.update([], removedAt + 89)[0].opacity).toBeGreaterThan(0);
    expect(shown.update([], removedAt + 90)).toEqual([]);
  }
});

test("renewed evidence cancels retirement and replaces the one visual slot", () => {
  const shown = new ContactPresentation(30, 3);
  shown.update([contact], 0);
  expect(shown.update([], 10)[0].retiring).toBe(true);
  expect(shown.update([], 55)[0].opacity).toBeCloseTo(0.5);
  const refreshed = {
    ...contact,
    source: "firing",
    center: [500, 600, 0] as [number, number, number],
    evidenceTick: 55,
    expiresTick: 955,
  };
  expect(shown.update([refreshed], 55)).toEqual([{ ...refreshed, opacity: 1, retiring: false }]);
  expect(shown.update([refreshed], 55)).toHaveLength(1);
});

test("rewinding clears visual memories instead of leaking the later battle", () => {
  const shown = new ContactPresentation(30, 3);
  shown.update([contact], 30);
  shown.update([], 31);
  expect(shown.update([], 0)).toEqual([]);
});

test("live evidence remains fully visible until authority removes it", () => {
  const shown = new ContactPresentation(30, 3);
  for (const tick of [0, 150, 255, 299, 300])
    expect(shown.update([contact], tick)[0].opacity).toBe(1);
});
