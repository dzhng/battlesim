// @vitest-environment node
// The unseen look's CPU seam (battle-look slice 15): `presentation.fog` names
// styles and selects one; a tint only tints. The GPU half (seen pixels
// untouched, every material path styled) runs in the `fog-look` scene.
import { expect, test } from "vitest";
import {
  FOG_DISTANCE_CAP_PX,
  FOG_EDGE_REACH_PX,
  fogStyleUniform,
  selectedFogStyle,
  validateFogPresentation,
  validateFogStyle,
  type FogPresentation,
  type FogStyle,
} from "@packages/battle-renderer/src/frame/fogStyle";
import village from "@fixtures/village.json";

const fixture = village.presentation.fog as unknown as FogPresentation;
const base = (): FogStyle => structuredClone(selectedFogStyle(fixture));

test("the fixture's styles are all drawable and one is selected", () => {
  expect(() => validateFogPresentation(structuredClone(fixture))).not.toThrow();
  expect(Object.keys(fixture.styles).length).toBeGreaterThan(1);
  expect(() => validateFogPresentation({ ...fixture, style: "no-such-style" })).toThrow(/style/);
});

test("a tint changes hue, never brightness", () => {
  for (const tint of [
    [0.5, 0.7, 1.4],
    [3, 3, 3],
    [0.1, 0.1, 2],
  ] as [number, number, number][]) {
    const u = fogStyleUniform({ ...base(), tint });
    const luma = 0.2126 * u.tint.x + 0.7152 * u.tint.y + 0.0722 * u.tint.z;
    expect(luma).toBeCloseTo(1, 6);
    // Same hue: the channels keep their ratios.
    expect(u.tint.z / u.tint.x).toBeCloseTo(tint[2] / tint[0], 6);
  }
});

test("a style that would brighten unseen, or has no lines to draw, is refused", () => {
  expect(() => validateFogStyle({ ...base(), dim: 1.5 })).toThrow(/dim/);
  expect(() => validateFogStyle({ ...base(), tint: [0, 0, 0] })).toThrow(/tint/);
  expect(() => validateFogStyle({ ...base(), veil: -0.1 })).toThrow(/veil/);
  expect(() => validateFogStyle({ ...base(), lines: { ...base().lines, spacing_px: 0 } })).toThrow(
    /spacing_px/,
  );
});

// Slice 15b: the edge between seen and unseen. The mask pass resolves each
// pixel's distance to the other side within a bounded reach, so a rim or a
// soft edge wider than that reach could not be drawn and is refused.
test("a rim or soft edge wider than the mask pass reaches is refused", () => {
  const s = base();
  expect(() =>
    validateFogStyle({ ...s, rim: { ...s.rim, width_px: FOG_EDGE_REACH_PX + 1 } }),
  ).toThrow(/rim.width_px/);
  expect(() => validateFogStyle({ ...s, edge_softness: FOG_EDGE_REACH_PX + 1 })).toThrow(
    /edge_softness/,
  );
  expect(() => validateFogStyle({ ...s, edge_softness: -1 })).toThrow(/edge_softness/);
  expect(() => validateFogStyle({ ...s, rim: { ...s.rim, alpha: 1.5 } })).toThrow(/rim.alpha/);
  expect(() => validateFogStyle({ ...s, rim: { ...s.rim, color: [1, 1] as never } })).toThrow(
    /rim.color/,
  );
});

test("the mask pass searches past the style's rim and soft edge, and no further than it can store", () => {
  for (const [width_px, edge_softness] of [
    [0, 0],
    [1.5, 0],
    [0, 3.2],
    [2, 6],
    [FOG_EDGE_REACH_PX, FOG_EDGE_REACH_PX],
  ]) {
    const s = base();
    const u = fogStyleUniform({ ...s, edge_softness, rim: { ...s.rim, width_px } });
    // A rim pixel lies within width + 1 pixel centres of the unseen side; a
    // softened unseen pixel within softness + 1 of the seen side.
    expect(u.reachPx).toBeGreaterThanOrEqual(Math.ceil(width_px) + 1);
    expect(u.reachPx).toBeGreaterThanOrEqual(Math.ceil(edge_softness) + 1);
    expect(u.reachPx).toBeLessThan(FOG_DISTANCE_CAP_PX);
  }
});
