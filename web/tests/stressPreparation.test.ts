// @vitest-environment node
import { expect, test, vi } from "vitest";
import { prepare, type PreparationModule } from "../src/battle/prepare/prepare";
import type { PrepareBattleRequest, PrepareDocuments } from "../src/battle/prepare/protocol";

const request: PrepareBattleRequest = {
  map_source: {
    kind: "generated",
    request: {
      type: "metro",
      size: "large",
      seed: "4",
      profile: "standard",
      generator_version: "test",
      preset_revision: "test",
      template_catalog_hash: "a".repeat(64),
      limits: { max_authored_parts: 10, max_bay_positions: 10, max_ground_points: 10 },
    },
  },
  factions: ["us", "eastern"],
  battle_seed: 7,
};
const documents: PrepareDocuments = {
  rules: "{}",
  presets: "{}",
  templates: "[]",
};
const map = { size: [3000, 2000], buildings: [], surfaces: [], forests: [], props: [] };
const identity = { seed: "4", map_hash: "b".repeat(64) };
const scenario = JSON.stringify({
  map,
  rules: {},
  units: [
    { side: "blue", position: [1200, 1000] },
    { side: "red", position: [1800, 1000] },
  ],
  scripts: [{ tick: 1 }],
});
// The external Wasm seam supplies admitted compiled geometry and its authoritative
// stress setup; preparation owns carrying these bytes and metadata off-page.
const fromScenario = vi.fn((json: string) => {
  expect(json).toBe(scenario);
  return {
    free: vi.fn(),
    extents: () =>
      JSON.stringify({
        playable: [0, 0, 3000, 2000],
        physical: [0, 0, 3000, 2000],
        rendered: [0, 0, 3000, 2000],
      }),
  };
});
const module = {
  PreparedWorld: class {
    constructor() {
      throw new Error("stress must prepare its authoritative scenario map");
    }
    static from_scenario = fromScenario;
  },
  check_prepare_request: (json: string) =>
    JSON.stringify({ status: "ok", request: JSON.parse(json) }),
  generate_map: () =>
    JSON.stringify({ status: "ok", result: { map, identity, report: {}, sites: {} } }),
  map_generator_version: () => "test",
  template_catalogue_json: () => "{}",
  city_stress_preparation: (json: string, rules: string, seed: bigint, late: boolean) => {
    expect(JSON.parse(json)).toEqual(map);
    expect(rules).toBe(documents.rules);
    expect(seed).toBe(7n);
    expect(late).toBe(true);
    return `{"scenario":${scenario},"report":${JSON.stringify({ start: { at: [1200, 1000], yaw: 0 }, livingUnits: { blue: 1, red: 1 } })}}`;
  },
} as unknown as PreparationModule;
const saved = {
  loadMap: () => {
    throw new Error("no substituted saved map");
  },
};

test("generated stress preparation retains the resolved full map and authoritative fixture", async () => {
  const stages: string[] = [];
  const result = await prepare(
    module,
    new WebAssembly.Memory({ initial: 1 }),
    request,
    documents,
    saved,
    (s) => stages.push(s),
    () => 0,
    { kind: "city-arena-2", late: true },
  );
  expect(result.scenario).toBe(scenario);
  expect(fromScenario).toHaveBeenCalledOnce();
  expect(result.report.identity).toEqual({ kind: "generated", generation: identity });
  expect(result.report.size).toEqual([3000, 2000]);
  expect(result.report.stress).toEqual({
    kind: "city-arena-2",
    late: true,
    livingUnits: { blue: 1, red: 1 },
  });
  expect(stages).toEqual(["map", "encounter"]);
});

test("a malformed stress selector refuses before another encounter can be substituted", async () => {
  await expect(
    prepare(
      module,
      new WebAssembly.Memory({ initial: 1 }),
      request,
      documents,
      saved,
      undefined,
      undefined,
      null as never,
    ),
  ).rejects.toMatchObject({
    stage: "request",
    diagnostics: [expect.objectContaining({ location: "$.stress" })],
  });
});
