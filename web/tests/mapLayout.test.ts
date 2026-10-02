// @vitest-environment node
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, generate_map, generate_map_plan } from "@wasm/game_wasm.js";

const fixture = (path: string) =>
  readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8");
const presets = fixture("map-presets.json");
const templates = fixture("prototype-building-templates.json");
// The catalog's documents, as the game's rules carry them.
const catalog = JSON.stringify(JSON.parse(fixture("catalog.json")).documents);

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
});

type Record = { name: string; command: string; request_json: string; native_sha256: string };
const cases: Record[] = JSON.parse(fixture("parity/map-layout/paired-records.json")).cases;

test.each(cases)("$name: native CLI and WASM agree on bytes or refusal", (record) => {
  const generate = record.command === "generate-map" ? generate_map : generate_map_plan;
  const outcome = generate(record.request_json, presets, templates, catalog);
  expect(createHash("sha256").update(outcome).digest("hex")).toBe(record.native_sha256);
});

test("a generated map carries the plan's ground and buildings and none of its plan-only layers", () => {
  const record = cases.find((c) => c.command === "generate-map" && c.name.includes("metro small"))!;
  const plan = JSON.parse(generate_map_plan(record.request_json, presets, templates, catalog)).plan;
  const map = JSON.parse(generate_map(record.request_json, presets, templates, catalog)).result.map;
  expect(map.size).toEqual([6000, 6000]);
  expect(map.surfaces).toEqual(plan.surfaces);
  expect(map.forests).toEqual(plan.forests);
  // The plan's streets and its buildings reach the map: each building is the
  // plan row's template in the plan row's frame.
  expect(plan.surfaces.filter((s: { kind: string }) => s.kind === "road").length).toBeGreaterThan(
    100,
  );
  expect(plan.buildings.length).toBeGreaterThan(1000);
  expect(
    map.buildings.map((b: { geometry: { template_id: string; frame: unknown } }) => [
      b.geometry.template_id,
      b.geometry.frame,
    ]),
  ).toEqual(
    plan.buildings.map((b: { template_id: string; frame: unknown }) => [b.template_id, b.frame]),
  );
  expect(plan.settlements.length).toBeGreaterThan(0);
  expect(plan.lots.length).toBeGreaterThanOrEqual(plan.buildings.length);
  for (const layer of ["settlements", "approaches", "lots"]) {
    expect(Object.keys(map)).not.toContain(layer);
  }
});

test("a generated river and the bridges over it reach the map as the plan wrote them", () => {
  const record = cases.find((c) => c.command === "generate-map" && c.name.includes("river"))!;
  const plan = JSON.parse(generate_map_plan(record.request_json, presets, templates, catalog)).plan;
  const map = JSON.parse(generate_map(record.request_json, presets, templates, catalog)).result.map;
  expect(plan.rivers.length).toBe(1);
  expect(plan.bridges.length).toBeGreaterThan(0);
  expect(map.rivers).toEqual(plan.rivers);
  expect(map.bridges).toEqual(plan.bridges);
});
