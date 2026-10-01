// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import lab from "@fixtures/generated-lab.json";
import { VILLAGE_RULES } from "@apps/battle-lab/src/scenarios";
import { MapRefused, prepareGeneratedBattle } from "../src/battle/prepare/generatedBattle";
import type { PrepareRequest } from "../src/battle/prepare/protocol";
import type { WorldLayout } from "@packages/battle-renderer/src/worldMesh";

const fixture = (path: string) =>
  readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8");

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = wasm.initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

const rules = JSON.stringify(VILLAGE_RULES);
const request = (over: Partial<PrepareRequest> = {}): PrepareRequest => ({
  type: "prepare",
  map: { type: "open", size: "small", seed: "1" },
  presets: fixture("map-presets.json"),
  templates: fixture("prototype-building-templates.json"),
  limits: lab.limits,
  rules,
  encounter: lab.encounter,
  ...over,
});

interface Scenario {
  map: { size: [number, number]; buildings: { owner: number; kind: string }[] };
  units: { side: string; kind: string; position: [number, number] }[];
  opponent: { garrisons: [number, number][] };
  encounter: { success_zone_center: [number, number] };
}

test("a prepared battle is the requested map with both forces standing on it, and it runs", () => {
  const stages: string[] = [];
  const { scenario: json, report } = prepareGeneratedBattle(wasm, memory, request(), (s) =>
    stages.push(s),
  );
  expect(stages).toEqual(["generating", "placing"]);
  expect(report.map).toEqual({ type: "open", size: "small", seed: "1" });
  expect(report.identity.seed).toBe("1");
  expect(report.size).toEqual([6000, 6000]);

  const scenario = JSON.parse(json) as Scenario;
  // The scenario carries exactly the map the generator makes for that request.
  const direct = JSON.parse(
    wasm.generate_map(
      JSON.stringify({
        generator_version: wasm.map_generator_version(),
        preset_revision: JSON.parse(fixture("map-presets.json")).revision,
        seed: "1",
        template_catalog_hash: report.identity.template_catalog_hash,
        type: "open",
        size: "small",
        limits: lab.limits,
      }),
      fixture("map-presets.json"),
      fixture("prototype-building-templates.json"),
    ),
  );
  expect(report.identity).toEqual(direct.result.identity);
  expect(scenario.map).toEqual(direct.result.map);

  // Blue's column stands on road, in the recipe's order; red stands on open ground.
  const view = new wasm.WorldView(JSON.stringify(scenario.map), rules);
  const layout = JSON.parse(wasm.world_layout(rules)) as WorldLayout;
  const surface = (p: [number, number]) => {
    const [, , , , , kind, , traversable] = view.surface_at(p[0], p[1]);
    return { kind: layout.surfaceKinds[kind], traversable: traversable === 1 };
  };
  const blue = scenario.units.filter((u) => u.side === "blue");
  const red = scenario.units.filter((u) => u.side === "red");
  expect(blue.map((u) => u.kind)).toEqual(lab.encounter.blue.column);
  expect(red.map((u) => u.kind)).toEqual(lab.encounter.red.map((r) => r.kind));
  for (const unit of blue) expect(surface(unit.position).kind).toBe("road");
  for (const unit of red) expect(surface(unit.position).traversable).toBe(true);
  // The column heads for the town: its leader is the unit nearest it.
  const toTown = (u: { position: [number, number] }) =>
    Math.hypot(
      u.position[0] - scenario.encounter.success_zone_center[0],
      u.position[1] - scenario.encounter.success_zone_center[1],
    );
  expect(Math.min(...blue.map(toTown))).toBe(toTown(blue[0]));
  expect(report.anchors.blue).toEqual(blue[0].position);

  // Every garrison row names a red unit and a building a squad can hold.
  const garrisonable = new Set(
    scenario.map.buildings
      .filter((b) => layout.garrisonPropKinds.includes(b.kind))
      .map((b) => b.owner),
  );
  expect(scenario.opponent.garrisons.length).toBe(
    lab.encounter.red.filter((r) => r.garrison).length,
  );
  for (const [unit, building] of scenario.opponent.garrisons) {
    expect(scenario.units[unit].side).toBe("red");
    expect(garrisonable.has(building)).toBe(true);
  }
  view.free();

  // The simulation takes it, and the defender's squads walk into their buildings.
  const battle = new wasm.Battle(json, 1);
  for (let tick = 0; tick < 300; tick++) battle.step();
  expect(battle.tick()).toBe(300);
  battle.free();
});

test("the same request prepares the same battle, and another seed another map", () => {
  const a = prepareGeneratedBattle(wasm, memory, request());
  const b = prepareGeneratedBattle(wasm, memory, request());
  const other = prepareGeneratedBattle(
    wasm,
    memory,
    request({ map: { type: "open", size: "small", seed: "2" } }),
  );
  expect(b.scenario).toBe(a.scenario);
  expect(other.report.identity.map_hash).not.toBe(a.report.identity.map_hash);
});

test("a refused request reports the generator's diagnostics and prepares nothing", () => {
  const refuse = () =>
    prepareGeneratedBattle(
      wasm,
      memory,
      request({ limits: { ...lab.limits, max_authored_parts: 10 } }),
    );
  expect(refuse).toThrow(MapRefused);
  try {
    refuse();
  } catch (error) {
    expect((error as MapRefused).diagnostics[0].code).toBe("complexity_limit");
  }
});
