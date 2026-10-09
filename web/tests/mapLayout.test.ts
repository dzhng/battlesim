// @vitest-environment node
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, generate_map } from "@wasm/game_wasm.js";
import { WHOLE_MAP_MS } from "./support/wholeMap";

const fixture = (path: string) =>
  readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8");
const presets = fixture("map-presets.json");
const templates = fixture("prototype-building-templates.json");
// The resolved physical rules shared by generation and the battle.
const rules = JSON.stringify({
  ...JSON.parse(fixture("game.json")),
  catalog: JSON.parse(fixture("catalog.json")).documents,
});

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
});

type Record = { name: string; command: string; request_json: string; native_sha256: string };
const cases: Record[] = JSON.parse(fixture("parity/map-layout/paired-records.json")).cases;
// The browser generates whole maps; the CLI's plan-only records are native.
const maps = cases.filter((c) => c.command === "generate-map");
const recordedRules = fixture("parity/map-layout/physical-rules.json");

test.each(maps)(
  "$name: native CLI and WASM agree on bytes or refusal",
  (record) => {
    const outcome = generate_map(record.request_json, presets, templates, recordedRules);
    expect(createHash("sha256").update(outcome).digest("hex")).toBe(record.native_sha256);
  },
  WHOLE_MAP_MS,
);

test(
  "a generated map carries its streets and buildings and none of the plan-only layers",
  () => {
    const record = maps.find((c) => c.name.includes("metro medium"))!;
    const map = JSON.parse(generate_map(record.request_json, presets, templates, rules)).result.map;
    expect(map.size).toEqual([6000, 6000]);
    expect(map.surfaces.filter((s: { kind: string }) => s.kind === "road").length).toBeGreaterThan(
      100,
    );
    expect(map.buildings.length).toBeGreaterThan(1000);
    for (const layer of ["settlements", "approaches", "lots"]) {
      expect(Object.keys(map)).not.toContain(layer);
    }
  },
  WHOLE_MAP_MS,
);

test(
  "a generated river and the bridges over it reach the map",
  () => {
    const record = maps.find((c) => c.name.includes("river"))!;
    const map = JSON.parse(generate_map(record.request_json, presets, templates, rules)).result.map;
    expect(map.rivers.length).toBe(1);
    expect(map.bridges.length).toBeGreaterThan(0);
  },
  WHOLE_MAP_MS,
);

test(
  "generation uses the battle's explicit physical rules and pins their identity",
  () => {
    const request = cases.find((c) => c.name === "open medium")!.request_json;
    const physical = JSON.parse(rules);
    const build = (input: unknown) =>
      JSON.parse(generate_map(request, presets, templates, JSON.stringify(input)));
    const before = build(physical);
    expect(before.status).toBe("ok");
    // A crown-height change does not move authored geometry, but changes the
    // physical world that stands on it and therefore the generation inputs.
    physical.forests.rule.canopy_height_m += 0.01;
    const after = build(physical);
    expect(after.status).toBe("ok");
    expect(after.result.identity.map_hash).toBe(before.result.identity.map_hash);
    expect(after.result.identity.config_hash).not.toBe(before.result.identity.config_hash);
    delete physical.forests;
    const refused = build(physical);
    expect(refused.status).toBe("error");
    expect(refused.diagnostics[0].code).toBe("invalid_physical_rules");
    expect(refused.diagnostics[0].location).toBe("$.rules");
  },
  WHOLE_MAP_MS,
);
