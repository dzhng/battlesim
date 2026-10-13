// @vitest-environment node
import { expect, test } from "vitest";
import { browserChoice } from "./browser.mjs";

const wsl = { wsl: true, mirrored: true, windowsChrome: true };
const linux = { wsl: false, mirrored: false, windowsChrome: false };

test("inside WSL, WebGPU checks run on Windows Chrome's real GPU", () => {
  expect(browserChoice(undefined, wsl)).toEqual({ kind: "windows" });
});

test("elsewhere, and when asked, they run on Playwright's Chromium", () => {
  expect(browserChoice(undefined, linux)).toEqual({ kind: "local" });
  expect(browserChoice("local", wsl)).toEqual({ kind: "local" });
});

test("WSL that cannot reach Windows Chrome falls back and says why", () => {
  const nat = browserChoice(undefined, { ...wsl, mirrored: false });
  expect(nat.kind).toBe("local");
  expect(nat.note).toMatch(/mirrored/);
  const noChrome = browserChoice(undefined, { ...wsl, windowsChrome: false });
  expect(noChrome.kind).toBe("local");
  expect(noChrome.note).toMatch(/chrome\.exe/);
});

test("an unknown setting is refused by name", () => {
  expect(() => browserChoice("windows", wsl)).toThrow(/windows.*local/);
});
