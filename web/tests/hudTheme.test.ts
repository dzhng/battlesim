// @vitest-environment node
import { expect, test } from "vitest";
import {
  hudProperties,
  validateCalloutGlow,
  validateHudTheme,
  villageCalloutGlow,
  villageHud,
  type HudTheme,
} from "../src/battle/present/hudTheme";
import {
  GLOW_MAX_RADIUS_PX,
  validateOverlayGlow,
} from "../../packages/battle-renderer/src/frame/overlayPass";
import village from "../../fixtures/village.json";

test("the HUD theme becomes channel triples every rule can give its own alpha", () => {
  const props = hudProperties(villageHud);
  expect(props["--hud-accent"]).toMatch(/^\d{1,3} \d{1,3} \d{1,3}$/);
  expect(props["--hud-glass-alpha"]).toBe(String(villageHud.glass[3]));
  expect(props["--hud-font"]).toBe(villageHud.font);
});

test("the HUD's selection colour is the order scheme's selection role", () => {
  // One selection colour everywhere: the selected leader line (hud.css
  // `--hud-selected`) and the ground marker come from the fixture's scheme.
  const orders = village.presentation.overlay.orders;
  const selected = (orders.schemes as Record<string, { selected: { color: number[] } }>)[
    orders.scheme
  ].selected.color;
  const rgb = selected.slice(0, 3).map((v) => Math.round(v * 255)).join(" ");
  expect(hudProperties(villageHud)["--hud-selected"]).toBe(rgb);
});

test("a HUD theme missing a colour, or one out of range, is refused", () => {
  const { warn: _, ...missing } = villageHud;
  expect(() => validateHudTheme(missing as unknown as HudTheme)).toThrow(/presentation\.hud/);
  expect(() => validateHudTheme({ ...villageHud, accent: [0.5, 1.5, 0.5] })).toThrow();
});

test("the overlay glow is the fixture's, and one past the blur's reach is refused", () => {
  const glow = village.presentation.overlay.glow;
  const halo = { radius_px: glow.radius_px, strength: glow.overlay };
  expect(validateOverlayGlow(halo)).toEqual(halo);
  expect(hudProperties(villageHud, villageCalloutGlow)["--hud-callout-glow"]).toBe(
    String(glow.callouts),
  );
  expect(() => validateCalloutGlow(-1)).toThrow(/callouts/);
  expect(() => validateOverlayGlow({ radius_px: GLOW_MAX_RADIUS_PX + 1, strength: 1 })).toThrow();
  expect(() => validateOverlayGlow({ radius_px: 4, strength: -1 })).toThrow();
});
