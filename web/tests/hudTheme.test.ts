// @vitest-environment node
import { presentationSettings } from "@web/presentationSettings";
import { expect, test } from "vitest";
import { hudProperties } from "../src/battle/present/hudCss";
import { gameCalloutGlow, gameHud } from "../src/battle/present/hudTheme";

const selected = presentationSettings.overlay.orders.selected;

test("the HUD theme becomes channel triples every rule can give its own alpha", () => {
  const props = hudProperties(gameHud, selected);
  expect(props["--hud-accent"]).toMatch(/^\d{1,3} \d{1,3} \d{1,3}$/);
  expect(props["--hud-glass-alpha"]).toBe(String(gameHud.glass[3]));
  expect(props["--hud-font"]).toBe(gameHud.font);
});

test("the admitted callout glow reaches CSS", () => {
  expect(hudProperties(gameHud, selected, gameCalloutGlow)["--hud-callout-glow"]).toBe(
    String(presentationSettings.overlay.glow.callouts),
  );
});
