// @vitest-environment node
// The scenario builder's economy mode fields the simulation's own skirmish
// setup: Rust's skirmish sites (entries and objectives) and its starter
// rules, with only the builder's authoring overrides on top.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import matchRules from "@fixtures/skirmish.json";
import { builderEconomy, emptyBuilderMap } from "@apps/battle-lab/src/scenario-builder/model";
import { playersFor } from "@apps/battle-lab/src/skirmishSetup";

beforeAll(() => {
  const module = wasm.initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  });
  // Keep the initialized module alive for this file's calls.
  expect(module.memory).toBeDefined();
});

test.each([1, 2, 3])("builder economy fields Rust's skirmish sites for %s a side", (count) => {
  const map = emptyBuilderMap();
  const players = playersFor(count, count);
  const builder = builderEconomy(wasm, { map, players });
  const setup = JSON.parse(
    wasm.synthetic_skirmish_setup(JSON.stringify(map.size), JSON.stringify(players)),
  );
  expect(builder.sites).toEqual(setup.sites);
});

test("builder economy keeps Rust's starter rules but for the builder's own overrides", () => {
  const builder = builderEconomy(wasm, { map: emptyBuilderMap(), players: playersFor(1, 1) });
  const overridden = new Set(["starting_minutes", "preparation_s"]);
  const kept = (rules: Record<string, unknown>) =>
    Object.fromEntries(Object.entries(rules).filter(([name]) => !overridden.has(name)));
  // The starter rules are `fixtures/skirmish.json`, which Rust reads.
  expect(kept(builder.rules)).toEqual(kept(matchRules));
  expect(Object.keys(builder.rules).sort()).toEqual(Object.keys(matchRules).sort());
});
