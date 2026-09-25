// @vitest-environment node
import { expect, test } from "vitest";
import fixtures from "@apps/battle-lab/src/fixtures.json";
import { LAB_FIXTURES, ROUTES } from "@apps/battle-lab/src/router";

test("every registered fixture has a page and every page a fixture", () => {
  expect(Object.keys(ROUTES).sort()).toEqual(fixtures.map((f) => f.id).sort());
});

test("fixture ids and routes are unique and builds are known", () => {
  expect(new Set(LAB_FIXTURES.map((f) => f.id)).size).toBe(LAB_FIXTURES.length);
  expect(new Set(LAB_FIXTURES.map((f) => f.route)).size).toBe(LAB_FIXTURES.length);
  for (const f of fixtures as { build?: string }[])
    expect([undefined, "production"]).toContain(f.build);
});
