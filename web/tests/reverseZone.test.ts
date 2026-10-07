// @vitest-environment node
import { UNITS } from "./catalog";
import { expect, test } from "vitest";
import game from "@fixtures/game.json";
import { inReverseZone } from "../src/battle/input/reverseZone";
import type { OwnUnitView } from "../src/battle/sim/observation";

const { reverse_zone_length_m: length, reverse_zone_margin_m: margin } = game.controls;
const [halfLength, halfWidth] = UNITS.hull("test_tank")!.half_extents_m;

/** A unit as the zone reads it: kind, position and yaw. */
const unit = (kind: string, yaw: number, at: [number, number] = [100, 50]) =>
  ({ id: 0, kind, position: [at[0], at[1], 0], yaw }) as unknown as OwnUnitView;

/** A point `back` metres behind the rear face and `side` metres to the left
 *  of the centreline of a hull at `at` facing `yaw`. */
function behind(yaw: number, back: number, side: number, at: [number, number] = [100, 50]) {
  const along = -(halfLength + back);
  return [
    at[0] + along * Math.cos(yaw) - side * Math.sin(yaw),
    at[1] + along * Math.sin(yaw) + side * Math.cos(yaw),
  ] as [number, number];
}

test("a point in the strip behind a single vehicle is in the zone, whatever its facing", () => {
  for (const yaw of [0, 1, Math.PI, -2.5]) {
    const tank = unit("test_tank", yaw);
    expect(inReverseZone(UNITS, [tank], behind(yaw, 10, 0))).toBe(true);
    expect(inReverseZone(UNITS, [tank], behind(yaw, length - 0.1, halfWidth + margin - 0.1))).toBe(
      true,
    );
  }
});

test("past the strip's end or sides, or in front, is outside the zone", () => {
  const tank = unit("test_tank", 0.7);
  expect(inReverseZone(UNITS, [tank], behind(0.7, length + 0.5, 0))).toBe(false);
  expect(inReverseZone(UNITS, [tank], behind(0.7, 10, halfWidth + margin + 0.5))).toBe(false);
  expect(inReverseZone(UNITS, [tank], behind(0.7, -2 * halfLength - 10, 0))).toBe(false);
});

test("two selected units, or a squad, never reverse", () => {
  const tank = unit("test_tank", 0);
  expect(inReverseZone(UNITS, [tank, unit("test_tank", 0, [100, 60])], behind(0, 10, 0))).toBe(
    false,
  );
  expect(inReverseZone(UNITS, [unit("test_rifle", 0)], behind(0, 10, 0))).toBe(false);
});
