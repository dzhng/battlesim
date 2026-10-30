// @vitest-environment node
import { expect, test } from "vitest";
import { contactUnder } from "../src/battle/input/contactPick";
import type { ContactView } from "../src/battle/sim/observationView";

const contact = (
  id: number,
  [x, y]: [number, number],
  radius: number,
  z = 0,
  layer = "ground",
): ContactView => ({
  id,
  source: "firing",
  center: [x, y, z],
  layer,
  aloft: layer !== "ground",
  radius,
  evidenceTick: 0,
  expiresTick: 100,
  kind: null,
  heard: [],
});

test("the contact under a ground point is the one whose area holds it, the nearest centre first", () => {
  const areas = [contact(1, [0, 0], 10), contact(2, [12, 0], 10), contact(3, [100, 0], 5)];
  expect(contactUnder(areas, [8, 0])).toBe(2);
  expect(contactUnder(areas, [-9, 0])).toBe(1);
  expect(contactUnder(areas, [50, 0])).toBe(null);
  expect(contactUnder(areas, [104.9, 0])).toBe(3);
  expect(contactUnder(areas, null)).toBe(null);
});

test("an aloft contact is picked where the pointer's ray meets it at height, not on the ground below", () => {
  // A helicopter's report 30 m up over (100, 0); the camera looks down at
  // 45° from the west, so the ray through the report meets the ground 30 m
  // beyond it.
  const air = [contact(1, [100, 0], 10, 30, "low_air")];
  const s = Math.SQRT1_2;
  const through = { origin: [0, 0, 130], dir: [s, 0, -s] } as const;
  expect(contactUnder(air, [130, 0], through)).toBe(1);
  const under = { origin: [0, 0, 100], dir: [s, 0, -s] } as const;
  expect(contactUnder(air, [100, 0], under), "the ground beneath it is empty").toBe(null);
  const behind = { origin: [200, 0, 130], dir: [s, 0, -s] } as const;
  expect(contactUnder(air, [330, 0], behind), "a ray already past it misses").toBe(null);
  // Heard from the low hover, the report lies on the ground like any other.
  const low = [{ ...contact(1, [100, 0], 10, 4, "low_air"), aloft: false }];
  expect(contactUnder(low, [100, 0], under)).toBe(1);
  expect(contactUnder(low, [130, 0], through)).toBe(null);
});
