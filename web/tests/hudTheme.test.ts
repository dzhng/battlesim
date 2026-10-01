// @vitest-environment node
import { expect, test } from "vitest";
import {
  hudProperties,
  validateCalloutGlow,
  validateHudTheme,
  gameCalloutGlow,
  gameHud,
  type HudTheme,
} from "../src/battle/present/hudTheme";
import {
  GLOW_MAX_RADIUS_PX,
  validateOverlayGlow,
} from "../../packages/battle-renderer/src/frame/overlayPass";
import game from "../../fixtures/game.json";

test("the HUD theme becomes channel triples every rule can give its own alpha", () => {
  const props = hudProperties(gameHud);
  expect(props["--hud-accent"]).toMatch(/^\d{1,3} \d{1,3} \d{1,3}$/);
  expect(props["--hud-glass-alpha"]).toBe(String(gameHud.glass[3]));
  expect(props["--hud-font"]).toBe(gameHud.font);
});

test("a HUD theme missing a colour, or one out of range, is refused", () => {
  const { warn: _, ...missing } = gameHud;
  expect(() => validateHudTheme(missing as unknown as HudTheme)).toThrow(/presentation\.hud/);
  expect(() => validateHudTheme({ ...gameHud, accent: [0.5, 1.5, 0.5] })).toThrow();
});

test("the overlay glow is the fixture's, and one past the blur's reach is refused", () => {
  const glow = game.presentation.overlay.glow;
  const halo = { radius_px: glow.radius_px, strength: glow.overlay };
  expect(validateOverlayGlow(halo)).toEqual(halo);
  expect(hudProperties(gameHud, gameCalloutGlow)["--hud-callout-glow"]).toBe(String(glow.callouts));
  expect(() => validateCalloutGlow(-1)).toThrow(/callouts/);
  expect(() => validateOverlayGlow({ radius_px: GLOW_MAX_RADIUS_PX + 1, strength: 1 })).toThrow();
  expect(() => validateOverlayGlow({ radius_px: 4, strength: -1 })).toThrow();
});
