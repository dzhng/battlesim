// @vitest-environment node
// The scenario builder's economy mode fields the simulation's own skirmish
// setup: Rust's synthetic sites (entries and objectives) and its starter
// rules, with only the builder's authoring overrides on top.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import matchRules from "@fixtures/skirmish.json";
import { builderEconomy, emptyBuilderMap } from "@apps/battle-lab/src/scenario-builder/model";
import { playersFor } from "@apps/battle-lab/src/skirmishSetup";
import { TEST_RULES } from "./catalog";

beforeAll(() => {
  const module = wasm.initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  });
  // Keep the initialized module alive for this file's calls.
  expect(module.memory).toBeDefined();
});

test.each([
  [1, 1],
  [2, 2],
  [3, 3],
] as const)(
  "builder economy fields Rust's synthetic sites for %s a side (%s entries)",
  (count, entries) => {
    const map = emptyBuilderMap();
    const builder = builderEconomy(wasm, {
      map,
      rules: TEST_RULES,
      players: playersFor(count, count),
    });
    expect(builder.sites).toEqual(
      JSON.parse(wasm.synthetic_sites(JSON.stringify(map.size), entries)),
    );
  },
);

test("builder economy keeps Rust's starter rules but for the builder's own overrides", () => {
  const builder = builderEconomy(wasm, {
    map: emptyBuilderMap(),
    rules: TEST_RULES,
    players: playersFor(1, 1),
  });
  const overridden = new Set(["starting_minutes", "preparation_s"]);
  const kept = (rules: Record<string, unknown>) =>
    Object.fromEntries(Object.entries(rules).filter(([name]) => !overridden.has(name)));
  // The starter rules are `fixtures/skirmish.json`, which Rust reads.
  expect(kept(builder.rules)).toEqual(kept(matchRules));
  expect(Object.keys(builder.rules).sort()).toEqual(Object.keys(matchRules).sort());
});
