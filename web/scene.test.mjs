// @vitest-environment node
import { expect, test } from "vitest";
import { loadRegistry, parseArgs, selectFixtures } from "./scene.mjs";

test("scene args: fixture ids, the bun `--` separator and --list", () => {
  expect(parseArgs(["--", "foundation"])).toEqual({ ids: ["foundation"], list: false });
  expect(parseArgs(["--list"])).toEqual({ ids: [], list: true });
  expect(() => parseArgs(["--bogus"])).toThrow("unknown flag");
});

test("every registered fixture has exactly one scene", async () => {
  const fixtures = await loadRegistry();
  expect(fixtures.length).toBeGreaterThan(0);
  expect(new Set(fixtures.map((fixture) => fixture.id)).size).toBe(fixtures.length);
  expect(selectFixtures(fixtures, []).length).toBe(fixtures.length);
  expect(() => selectFixtures(fixtures, ["nope"])).toThrow("unknown fixture");
});
