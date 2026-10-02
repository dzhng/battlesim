// @vitest-environment node
import { expect, test } from "vitest";
import { ContactPresentation } from "../src/battle/present/contactPresentation";
import type { ContactView } from "../src/battle/sim/observation";

const contact: ContactView = {
  id: 7,
  source: "last_seen",
  center: [100, 200],
  radius: 20,
  evidenceTick: 0,
  expiresTick: 300,
  primaryLabel: true,
  kind: "tank",
  heard: [],
};

test("removed contacts fade at their remembered position and stop accepting input", () => {
  const shown = new ContactPresentation(30, 3);
  const before = shown.update([contact], 30)[0];
  const removed = shown.update([], 31)[0];
  expect(removed.opacity).toBeCloseTo(before.opacity);
  expect(removed.retiring).toBe(true);
  expect(removed.center).toEqual(contact.center);
  const halfway = shown.update([], 76)[0];
  expect(halfway.opacity).toBeGreaterThan(0);
  expect(halfway.opacity).toBeCloseTo(removed.opacity / 2);
  expect(shown.update([], 121)).toEqual([]);
});

test("expiry fades to zero and refreshed evidence restores the report", () => {
  const shown = new ContactPresentation(30, 3);
  const fresh = shown.update([contact], 0)[0];
  const old = shown.update([contact], 299)[0];
  expect(old.opacity).toBeLessThan(fresh.opacity / 80);
  const refreshed = shown.update([{ ...contact, evidenceTick: 299, expiresTick: 599 }], 299)[0];
  expect(refreshed.opacity).toBe(1);
  expect(shown.update([], 599)).toEqual([]);
});

test("rewinding clears visual memories instead of leaking the later battle", () => {
  const shown = new ContactPresentation(30, 3);
  shown.update([contact], 30);
  shown.update([], 31);
  expect(shown.update([], 0)).toEqual([]);
});

test("contacts stay fully visible until their final three seconds", () => {
  const shown = new ContactPresentation(30, 3);
  expect(shown.update([contact], 0)[0].opacity).toBe(1);
  expect(shown.update([contact], 150)[0].opacity).toBe(1);
  expect(shown.update([contact], 210)[0].opacity).toBe(1);
  expect(shown.update([contact], 255)[0].opacity).toBeCloseTo(0.5);
  expect(shown.update([contact], 300)).toEqual([]);
});
