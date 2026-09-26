// @vitest-environment node
// The unseen look's CPU seam (battle-look slice 15): `presentation.fog` names
// styles and selects one; a tint only tints. The GPU half (seen pixels
// untouched, every material path styled) runs in the `fog-look` scene.
import { expect, test } from "vitest";
import {
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
  expect(() => validateFogStyle({ ...base(), lines: { ...base().lines, spacing_px: 0 } })).toThrow(
    /spacing_px/,
  );
});
