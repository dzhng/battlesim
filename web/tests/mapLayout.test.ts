// @vitest-environment node
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, generate_map, generate_map_plan } from "@wasm/game_wasm.js";

const fixture = (path: string) =>
  readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8");

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
});

type Record = { name: string; command: string; request_json: string; native_sha256: string };

test("native CLI and WASM generate the same layout bytes and refusals", () => {
  const presets = fixture("map-presets.json");
  const cases: Record[] = JSON.parse(fixture("parity/map-layout/paired-records.json")).cases;
  for (const record of cases) {
    const outcome =
      record.command === "generate-map"
        ? generate_map(record.request_json, presets, "[]")
        : generate_map_plan(record.request_json, presets);
    expect(createHash("sha256").update(outcome).digest("hex"), record.name).toBe(
      record.native_sha256,
    );
  }
});

test("a generated map carries the plan's ground and none of its plan-only layers", () => {
  const presets = fixture("map-presets.json");
  const cases: Record[] = JSON.parse(fixture("parity/map-layout/paired-records.json")).cases;
  const record = cases.find((c) => c.command === "generate-map" && c.name.includes("metro small"))!;
  const plan = JSON.parse(generate_map_plan(record.request_json, presets)).plan;
  const map = JSON.parse(generate_map(record.request_json, presets, "[]")).result.map;
  expect(map.size).toEqual([6000, 6000]);
  expect(map.surfaces).toEqual(plan.surfaces);
  expect(map.forests).toEqual(plan.forests);
  expect(plan.settlements.length).toBeGreaterThan(0);
  expect(Object.keys(map)).not.toContain("settlements");
  expect(Object.keys(map)).not.toContain("approaches");
});
