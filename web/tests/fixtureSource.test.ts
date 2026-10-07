import { expect, test, vi } from "vitest";
import { TEST_RULES } from "./catalog";
import * as fixtures from "@apps/battle-lab/src/fixtures";
import { villageScenario } from "@apps/battle-lab/src/savedMaps";

vi.mock("@web/maps/browser", () => ({
  loadMap: async (id: string) => ({
    definition: { size: id === "river" ? [640, 480] : [2200, 1800] },
  }),
}));

test("a shared scenario factory builds on its calling fixture's declared source", async () => {
  const fixture = fixtures.LAB_FIXTURES.find((fixture) => fixture.id === "fog")!;
  const original = fixture.map;
  fixture.map = "river";
  try {
    const authority = {
      village_scenario: (rules: string, variant: string) =>
        JSON.stringify({ map: JSON.parse(rules).map, variant }),
    };
    const scenario = JSON.parse(await villageScenario(authority, "fog", "ordinary", TEST_RULES));
    expect(scenario.map.size).toEqual([640, 480]);
    expect(scenario.variant).toBe("ordinary");
  } finally {
    fixture.map = original;
  }
});
