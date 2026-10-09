// @vitest-environment node
import { expect, test } from "vitest";
import { loadRegistry, parseArgs, pinnedFixtures, selectFixtures } from "./scene.mjs";

test("scene args: fixture ids, the bun `--` separator, --list and --visual", () => {
  expect(parseArgs(["--", "foundation"])).toEqual({
    ids: ["foundation"],
    list: false,
    visual: false,
  });
  expect(parseArgs(["--list"])).toEqual({ ids: [], list: true, visual: false });
  expect(parseArgs(["--visual"])).toEqual({ ids: [], list: false, visual: true });
  expect(() => parseArgs(["--visual", "panels"])).toThrow("--visual");
  expect(() => parseArgs(["--bogus"])).toThrow("unknown flag");
});

test("every registered fixture has exactly one scene", async () => {
  const fixtures = await loadRegistry();
  expect(fixtures.length).toBeGreaterThan(0);
  expect(new Set(fixtures.map((fixture) => fixture.id)).size).toBe(fixtures.length);
  expect(new Set(fixtures.map((fixture) => fixture.route)).size).toBe(fixtures.length);
  for (const fixture of fixtures) expect([undefined, "production"]).toContain(fixture.build);
  expect(selectFixtures(fixtures, []).length).toBe(fixtures.length);
  expect(() => selectFixtures(fixtures, ["nope"])).toThrow("unknown fixture");
});

test("--visual selects exactly the fixtures with approved pictures", async () => {
  const fixtures = await loadRegistry();
  const pinned = (await pinnedFixtures(fixtures)).map((f) => f.id);
  expect(pinned).toEqual(expect.arrayContaining(["cursor", "panels", "ui"]));
  // A scene with no approved pictures (the GPU foundation) is not selected.
  expect(pinned).not.toContain("foundation");
});
