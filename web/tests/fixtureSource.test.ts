import { expect, test, vi } from "vitest";
import { TEST_RULES } from "./catalog";
import * as fixtures from "@apps/battle-lab/src/fixtures";
import { enduranceScenario } from "@apps/battle-lab/src/savedMaps";

vi.mock("@web/maps/browser", () => ({
  loadMap: async (id: string) => ({
    definition: { size: id === "river" ? [640, 480] : [2200, 1800] },
  }),
}));

test("a shared scenario factory builds on its calling fixture's declared source", async () => {
  const fixture = fixtures.LAB_FIXTURES.find((fixture) => fixture.id === "endurance")!;
  const original = fixture.map;
  fixture.map = "river";
  try {
    const authority = {
      endurance_scenario: (map: string, _rules: string, seed: bigint, late: boolean) =>
        JSON.stringify({ map: JSON.parse(map), seed: Number(seed), late }),
    };
    const scenario = JSON.parse(
      await enduranceScenario(authority, "endurance", 7, false, TEST_RULES),
    );
    expect(scenario.map.size).toEqual([640, 480]);
    expect(scenario.seed).toBe(7);
  } finally {
    fixture.map = original;
  }
});
