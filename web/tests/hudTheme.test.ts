// @vitest-environment node
import { presentationSettings } from "@web/presentationSettings";
import { expect, test } from "vitest";
import { hudProperties, gameCalloutGlow, gameHud } from "../src/battle/present/hudTheme";

test("the HUD theme becomes channel triples every rule can give its own alpha", () => {
  const props = hudProperties(gameHud);
  expect(props["--hud-accent"]).toMatch(/^\d{1,3} \d{1,3} \d{1,3}$/);
  expect(props["--hud-glass-alpha"]).toBe(String(gameHud.glass[3]));
  expect(props["--hud-font"]).toBe(gameHud.font);
});

test("the admitted callout glow reaches CSS", () => {
  expect(hudProperties(gameHud, gameCalloutGlow)["--hud-callout-glow"]).toBe(
    String(presentationSettings.overlay.glow.callouts),
  );
});
