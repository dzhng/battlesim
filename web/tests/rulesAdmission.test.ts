// @vitest-environment node
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";

const fixture = (path: string) =>
  JSON.parse(readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8"));
const scenario = fixture("parity/skirmish-objectives.json").scenario;
scenario.rules = { ...fixture("rules.json"), catalog: fixture("catalog.json").documents };
const presets = JSON.stringify(fixture("map-presets.json"));
const templates = JSON.stringify(fixture("prototype-building-templates.json"));
const request = fixture("parity/map-layout/paired-records.json").cases.find(
  (record: { command: string }) => record.command === "generate-map",
).request_json;

beforeAll(() => {
  wasm.initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  });
});

const invalid = [
  {
    path: "weapons.test_bad_flight.lifetime_s",
    overrides: {
      weapons: { test_bad_flight: { extends: "rifle", name: "Bad Flight", lifetime_s: 1e6 } },
    },
  },
  {
    path: "infantry_movement.wander_period_s",
    overrides: { infantry_movement: { wander_period_s: 0 } },
  },
];

// Preserve Rust's canonical number spellings while corrupting a valid upload.
// Recompute its rules identity so identity/canonical checks cannot mask admission.
function badUpload(path: string): string {
  const safe = structuredClone(scenario);
  safe.skirmish = null;
  safe.rules.infantry_movement.wander_period_s = 1;
  safe.rules.weapons.test_bad_flight = { extends: "rifle", name: "Bad Flight", lifetime_s: 1 };
  const canonical = wasm.author_scenario_bundle(
    JSON.stringify({
      format_version: "scenario-bundle-2",
      runtime_build: "admission-test",
      map: { map_hash: "", template_catalog_hash: null },
      rules_hash: "",
      scenario: safe,
      mode: "sandbox",
      victory: { source: "return true" },
      authored_scripts: [],
      assets: [],
    }),
  );
  const rules = canonical.match(/"rules":(\{.*\}),"units":/)![1];
  const corrupted = path.startsWith("weapons.")
    ? rules.replace(
        /("test_bad_flight":\{[^}]*"lifetime_s":)1\.0/,
        (_, prefix: string) => `${prefix}1000000.0`,
      )
    : rules.replace('"wander_period_s":1.0', '"wander_period_s":0.0');
  expect(corrupted).not.toBe(rules);
  const hash = createHash("sha256").update(corrupted).digest("hex");
  return canonical
    .replace(rules, corrupted)
    .replace(/"rules_hash":"[a-f0-9]+"/, `"rules_hash":"${hash}"`);
}

test.each(invalid)("$path is refused before any WASM handle is returned", ({ path, overrides }) => {
  const bad = structuredClone(scenario);
  for (const [section, patch] of Object.entries(overrides))
    Object.assign(bad.rules[section], patch);
  const rules = JSON.stringify(bad.rules);
  const map = JSON.stringify(bad.map);
  const setup = JSON.stringify(bad);
  for (const create of [
    () => wasm.resolve_rules(rules),
    () => new wasm.Battle(setup, 1),
    () => wasm.Battle.from_replay(setup, "{}"),
    () => new wasm.PreparedWorld(map, rules),
    () => wasm.PreparedWorld.from_scenario(setup),
    () => new wasm.WorldView(map, rules),
    () => wasm.world_layout(rules),
    () => new wasm.FlightLab(map, rules, "{}", 1),
    () =>
      wasm.encounter_scenario(
        map,
        JSON.stringify({ units: [], overrides }),
        JSON.stringify(scenario.rules),
      ),
    () => new wasm.PreparedWorld(map, JSON.stringify(scenario.rules)).into_battle(setup, 1),
    () => new wasm.PreparedWorld(map, JSON.stringify(scenario.rules)).into_replay(setup, "{}"),
  ]) {
    expect(create).toThrow(path);
  }
  const upload = badUpload(path);
  for (const create of [
    () => wasm.resolve_rules(rules),
    () => wasm.PreparedWorld.from_bundle(upload),
    () => wasm.canonical_scenario_bundle(upload),
    () => wasm.scenario_bundle_details(upload),
    () => wasm.scenario_bundle_controllers(upload),
    () => wasm.author_scenario_bundle(upload),
  ])
    expect(create).toThrow(path);
  const admission = JSON.parse(wasm.check_scenario_bundle(upload));
  expect(admission.status).toBe("error");
  expect(admission.diagnostics[0].message).toContain(path);
  const result = JSON.parse(wasm.generate_map(request, presets, templates, rules));
  expect(result.status).toBe("error");
  expect(result.diagnostics[0].message).toContain(path);
});

test.each([0, -1])("forest spacing %s is refused by world and battle entries", (spacing) => {
  const bad = structuredClone(scenario);
  bad.rules.forests.rule.trunk_spacing_m = spacing;
  const map = JSON.stringify(bad.map);
  const rules = JSON.stringify(bad.rules);
  const setup = JSON.stringify(bad);
  for (const create of [
    () => new wasm.Battle(setup, 1),
    () => wasm.Battle.from_replay(setup, "{}"),
    () => new wasm.PreparedWorld(map, rules),
    () => wasm.PreparedWorld.from_scenario(setup),
    () => new wasm.WorldView(map, rules),
  ])
    expect(create).toThrow("trunk_spacing_m");
});
