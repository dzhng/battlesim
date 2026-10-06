// @vitest-environment node
import { expect, test } from "vitest";
import { savedScenario } from "@apps/battle-lab/src/savedMaps";

test("a saved encounter becomes the scenario natively built from it: its opponent and rules included", () => {
  const map = { size: [100, 100] };
  const rules = { tick_hz: 30 };
  const saved = {
    units: [{ side: "blue", kind: "tank", position: [1, 2] }],
    scripts: [{ tick: 0, side: "blue", order: { kind: "stop", units: [0] } }],
    opponent: { side: "red", garrisons: [] },
    encounter: { attacker: "blue" },
  };
  expect(JSON.parse(savedScenario(map, saved, rules))).toEqual({
    map,
    rules,
    units: saved.units,
    events: [],
    scripts: saved.scripts,
    opponent: saved.opponent,
    encounter: saved.encounter,
  });
  // An encounter with neither leaves them out, as the native scenario has none.
  const bare = JSON.parse(savedScenario(map, { units: [] }, rules));
  expect(bare.opponent).toBeUndefined();
  expect(bare.encounter).toBeUndefined();
});
