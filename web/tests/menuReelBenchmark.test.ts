import { expect, test } from "vitest";
import { MENU_REEL_WORKLOAD, menuReelFingerprint } from "../src/battle/benchmark/menuReel";

test("graphics workload keeps the menu fixture's complete scene identities", () => {
  expect(MENU_REEL_WORKLOAD.id).toBe("menu-reel");
  expect(MENU_REEL_WORKLOAD.scenes.length).toBeGreaterThan(0);
  expect(MENU_REEL_WORKLOAD.scenes.every((scene) => scene.map && scene.encounter)).toBe(true);
});

test("graphics workload fingerprint is deterministic and changes with its identity", () => {
  const first = menuReelFingerprint();
  expect(first).toMatch(/^[0-9a-f]{8}$/);
  expect(menuReelFingerprint()).toBe(first);
  expect(JSON.stringify(MENU_REEL_WORKLOAD)).toContain("market-town");
});

