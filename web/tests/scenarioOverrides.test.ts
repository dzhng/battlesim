// @vitest-environment node
// The Wasm half of the scenario-override paired record
// (`crates/sim/tests/maps.rs`): the browser makes a saved encounter's
// scenario through the simulation's own `EncounterDefinition::on`, its
// overrides merged in, so the battle it plays is the native one, digest for
// digest.
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import { savedScenario } from "@apps/battle-lab/src/savedMaps";
import { loadMap } from "@web/maps/node";
import type { Encounter } from "@web/maps/resolve";
import { TEST_RULES } from "./catalog";

const record: {
  map: string;
  seed: number;
  every: number;
  ticks: number;
  encounter: Encounter;
  digests: string[];
} = JSON.parse(
  readFileSync(new URL("../../fixtures/parity/scenario-overrides.json", import.meta.url), "utf8"),
);

test("an overridden encounter plays the native battle in Wasm", () => {
  const map = loadMap(record.map);
  const battle = new wasm.Battle(
    savedScenario(wasm, map.json, record.encounter, TEST_RULES),
    record.seed,
  );
  try {
    const digests = [battle.digest()];
    for (let tick = 1; tick <= record.ticks; tick++) {
      battle.step();
      if (tick % record.every === 0) digests.push(battle.digest());
    }
    expect(digests).toEqual(record.digests);
  } finally {
    battle.free();
  }
});
