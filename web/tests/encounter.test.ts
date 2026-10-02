// @vitest-environment node
// The Wasm half of the encounter planner's native/Wasm proof: the records'
// hashes are the native planner's (`crates/mapgen/tests/encounter.rs`), and
// the Wasm exports must plan the same bytes, placement or diagnostics.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, generate_map, plan_encounter } from "@wasm/game_wasm.js";
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";
import { mapAndSites } from "../src/maps/source";

const fixture = (path: string) =>
  readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8");
const presets = fixture("map-presets.json");
const templates = fixture("prototype-building-templates.json");
const recipes = JSON.parse(fixture("encounters.json")).recipes as Record<
  string,
  Record<string, Record<string, unknown>>
>;
const rules = JSON.stringify(GAME_RULES);

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
});

interface Record_ {
  name: string;
  request_json: string;
  recipe: string;
  recipe_patch?: Record<string, Record<string, unknown>>;
  encounter_seed: string;
  native_sha256: string;
  status: "ok" | "error";
}
const cases: Record_[] = JSON.parse(fixture("parity/encounter/paired-records.json")).cases;

/** The recipe a record names, with its patch merged over it section by section. */
function recipeJson(record: Record_): string {
  const recipe = structuredClone(recipes[record.recipe]);
  for (const [section, fields] of Object.entries(record.recipe_patch ?? {}))
    Object.assign(recipe[section], fields);
  return JSON.stringify(recipe);
}

test.each(cases)(
  "$name: native and WASM plan the same bytes or refusal",
  (record) => {
    const generated = generate_map(
      record.request_json,
      presets,
      templates,
      JSON.stringify(GAME_RULES),
    );
    // The generator's own bytes, as preparation hands them on.
    const { map, sites } = mapAndSites(generated);
    const outcome = plan_encounter(map, sites, rules, recipeJson(record), record.encounter_seed);
    expect(JSON.parse(outcome).status).toBe(record.status);
    expect(createHash("sha256").update(outcome).digest("hex")).toBe(record.native_sha256);
  },
  60_000,
);
