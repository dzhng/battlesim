// @vitest-environment node
import { presentationSettings } from "@web/presentationSettings";
import { expect, test } from "vitest";
import {
  strokeWidth,
  validateStrokeRule,
  type StrokeRule,
} from "../../packages/battle-renderer/src/strokeWidth";

const RULE: StrokeRule = {
  full_m_per_px: 0.05,
  thin_m_per_px: 0.4,
  thin_scale: 0.4,
  min_px: 1,
  min_m: 0.05,
};
/** A stroke's width on screen, in pixels, at zoom `m`. */
const onScreen = (px: number, m: number, rule = RULE) => strokeWidth(rule, m)(px) / m;

test("a stroke is its full width at the default camera and closer", () => {
  expect(onScreen(5, 0.05)).toBeCloseTo(5, 9);
  expect(onScreen(5, 0.02)).toBeCloseTo(5, 9);
});

test("pulling out thins every stroke smoothly to the thin scale, then holds", () => {
  const zooms = Array.from({ length: 200 }, (_, i) => 0.05 * 1.02 ** i);
  const widths = zooms.map((m) => onScreen(5, m));
  for (let i = 1; i < widths.length; i++) {
    expect(widths[i]).toBeLessThanOrEqual(widths[i - 1] + 1e-12);
    // No step: each 2 % zoom step moves the width by a sliver.
    expect(widths[i - 1] - widths[i]).toBeLessThan(0.1);
  }
  expect(onScreen(5, 0.4)).toBeCloseTo(2, 9);
  expect(onScreen(5, 2)).toBeCloseTo(2, 9);
  // Every stroke by the same factor: a marker's outline keeps its weight
  // relative to its route.
  expect(onScreen(3.75, 0.15) / onScreen(5, 0.15)).toBeCloseTo(0.75, 9);
});

test("no stroke thins below the legibility floor on screen, nor its own width", () => {
  expect(onScreen(1.5, 2)).toBeCloseTo(1, 9);
  expect(onScreen(0.5, 2)).toBeCloseTo(0.5, 9);
});

test("no stroke is narrower than the floor on the ground", () => {
  expect(strokeWidth(RULE, 1e-4)(5)).toBeCloseTo(0.05, 9);
});

test("the fixture's rule is valid and a backwards zoom range is refused", () => {
  expect(() => validateStrokeRule(presentationSettings.overlay.stroke)).not.toThrow();
  expect(() => validateStrokeRule({ ...RULE, thin_m_per_px: 0.01 })).toThrow(/stroke/);
  expect(() => validateStrokeRule({ ...RULE, thin_scale: 1.5 })).toThrow(/stroke/);
});

test("reduced render resolution preserves the minimum in render pixels without enlarging CSS-sized bodies", () => {
  const metresPerCssPixel = 2;
  for (const renderScale of [0.75, 1]) {
    const stroke = strokeWidth(RULE, metresPerCssPixel, renderScale);
    expect((stroke(1.5) / metresPerCssPixel) * renderScale).toBeCloseTo(RULE.min_px, 9);
    // A body/radius keeps the CSS zoom rule; raster resolution only controls strokes.
    const size = strokeWidth(RULE, metresPerCssPixel);
    expect(size(40) / metresPerCssPixel).toBe(40 * RULE.thin_scale);
    expect(stroke(40)).toBe(size(40));
  }
});
