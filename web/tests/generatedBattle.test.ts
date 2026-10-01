// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import lab from "@fixtures/generated-lab.json";
import encounters from "@fixtures/encounters.json";
import { VILLAGE_RULES } from "@apps/battle-lab/src/scenarios";
import { PreparationRefused, prepareGeneratedBattle } from "../src/battle/prepare/generatedBattle";
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
const assault = encounters.recipes.assault;

test("the lab plans a recipe the recipes file holds", () => {
  expect(Object.keys(encounters.recipes)).toContain(lab.encounter.recipe);
  expect(lab.encounter.seed).toMatch(/^(0|[1-9]\d{0,19})$/);
});
const request = (over: Partial<PrepareRequest> = {}): PrepareRequest => ({
  type: "prepare",
  map: { type: "open", size: "small", seed: "1" },
  presets: fixture("map-presets.json"),
  templates: fixture("prototype-building-templates.json"),
  limits: lab.limits,
  rules,
  recipe: JSON.stringify(assault),
  encounterSeed: lab.encounter.seed,
  ...over,
});

interface Scenario {
  map: { size: [number, number]; buildings: { owner: number; kind: string }[] };
  units: { side: string; kind: string; position: [number, number] }[];
  scripts: { tick: number; side: string }[];
  opponent: { side: string; garrisons: [number, number][] };
  encounter: { attacker: string; success_zone_center: [number, number] };
}

test("a prepared battle is the requested map with the planned encounter on it, and it runs", () => {
  const stages: string[] = [];
  const { scenario: json, report } = prepareGeneratedBattle(wasm, memory, request(), (s) =>
    stages.push(s),
  );
  expect(stages).toEqual(["generating", "placing"]);
  expect(report.map).toEqual({ type: "open", size: "small", seed: "1" });
  expect(report.identity.seed).toBe("1");
  expect(report.size).toEqual([6000, 6000]);
  expect(report.encounter.encounter_seed).toBe(lab.encounter.seed);
  expect(report.encounter.recipe_hash).toMatch(/^[0-9a-f]{64}$/);

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

  // Both rosters stand on the map in the recipe's order, blue's rows first.
  const blue = scenario.units.filter((u) => u.side === "blue");
  const red = scenario.units.filter((u) => u.side === "red");
  expect(blue.map((u) => u.kind)).toEqual(assault.forces.blue.map((r) => r.kind));
  expect(red.map((u) => u.kind)).toEqual(assault.forces.red.map((r) => r.kind));
  expect(scenario.units.slice(0, blue.length)).toEqual(blue);

  // Blue's column stands on road at the bottom of the map and red's at the
  // top; every unit stands on ground a mover may stand on.
  const view = new wasm.WorldView(JSON.stringify(scenario.map), rules);
  const layout = JSON.parse(wasm.world_layout(rules)) as WorldLayout;
  const surface = (p: [number, number]) => {
    const [, , , , , kind, , traversable] = view.surface_at(p[0], p[1]);
    return { kind: layout.surfaceKinds[kind], traversable: traversable === 1 };
  };
  for (const unit of scenario.units) expect(surface(unit.position).traversable).toBe(true);
  const [attack, defence] = report.placement.deployments;
  expect([attack.side, attack.edge, defence.side, defence.edge]).toEqual([
    "blue",
    "bottom",
    "red",
    "top",
  ]);
  for (const id of [...attack.units, ...defence.units])
    expect(surface(scenario.units[id].position).kind).toBe("road");
  for (const id of attack.units) expect(scenario.units[id].position[1]).toBeLessThan(2000);
  for (const id of defence.units) expect(scenario.units[id].position[1]).toBeGreaterThan(4000);
  // The two drives to the objective are within the recipe's difference.
  expect(Math.abs(attack.route_s - defence.route_s)).toBeLessThanOrEqual(
    assault.deployment.max_route_difference_s,
  );

  // The objective is the report's town, and the camera's anchor the head of
  // blue's column: its leader, the unit nearest the objective.
  expect(scenario.encounter.attacker).toBe("blue");
  expect(scenario.encounter.success_zone_center).toEqual(report.placement.objective.center);
  expect(report.anchors.town).toEqual(report.placement.objective.center);
  expect(report.anchors.blue).toEqual(blue[0].position);
  const toTown = (u: { position: [number, number] }) =>
    Math.hypot(u.position[0] - report.anchors.town[0], u.position[1] - report.anchors.town[1]);
  expect(Math.min(...attack.units.map((id) => toTown(scenario.units[id])))).toBe(toTown(blue[0]));

  // Every garrison row names a red squad and a building of its own that a
  // squad can hold, with a seat for each soldier.
  const garrisonable = new Set(
    scenario.map.buildings
      .filter((b) => layout.garrisonPropKinds.includes(b.kind))
      .map((b) => b.owner),
  );
  expect(scenario.opponent.side).toBe("red");
  expect(scenario.opponent.garrisons.length).toBe(
    assault.forces.red.filter((r) => r.post === "garrison").length,
  );
  expect(new Set(scenario.opponent.garrisons.map(([, building]) => building)).size).toBe(
    scenario.opponent.garrisons.length,
  );
  for (const [unit, building] of scenario.opponent.garrisons) {
    expect(scenario.units[unit].side).toBe("red");
    expect(garrisonable.has(building)).toBe(true);
  }
  expect(scenario.opponent.garrisons).toEqual(
    report.placement.garrisons.map((g) => [g.unit, g.building]),
  );
  for (const g of report.placement.garrisons) expect(g.seats).toBeGreaterThanOrEqual(g.soldiers);
  view.free();

  // The simulation takes it: the scripted relief column and the defender's
  // garrison orders are accepted, and the battle steps.
  expect(scenario.scripts.map((s) => [s.tick, s.side])).toEqual([[0, "red"]]);
  const battle = new wasm.Battle(json, 1);
  for (let tick = 0; tick < 300; tick++) battle.step();
  expect(battle.tick()).toBe(300);
  battle.free();
}, 60_000);

test("the same request prepares the same battle; the map seed and the encounter seed are separate", () => {
  const a = prepareGeneratedBattle(wasm, memory, request());
  const b = prepareGeneratedBattle(wasm, memory, request());
  expect(b.scenario).toBe(a.scenario);

  const otherMap = prepareGeneratedBattle(
    wasm,
    memory,
    request({ map: { type: "open", size: "small", seed: "2" } }),
  );
  expect(otherMap.report.identity.map_hash).not.toBe(a.report.identity.map_hash);

  // Another encounter seed on the same map: the same map and columns, and
  // (over a few seeds) another choice of garrison buildings.
  const garrisons = new Set([JSON.stringify(a.report.placement.garrisons.map((g) => g.building))]);
  for (const encounterSeed of ["2", "3"]) {
    const other = prepareGeneratedBattle(wasm, memory, request({ encounterSeed }));
    expect(other.report.identity).toEqual(a.report.identity);
    expect(other.report.placement.deployments).toEqual(a.report.placement.deployments);
    garrisons.add(JSON.stringify(other.report.placement.garrisons.map((g) => g.building)));
  }
  expect(garrisons.size).toBeGreaterThan(1);
}, 60_000);

test("a refused map reports the generator's diagnostics and prepares nothing", () => {
  const refuse = () =>
    prepareGeneratedBattle(
      wasm,
      memory,
      request({ limits: { ...lab.limits, max_authored_parts: 10 } }),
    );
  expect(refuse).toThrow(PreparationRefused);
  try {
    refuse();
  } catch (error) {
    expect((error as PreparationRefused).stage).toBe("generating");
    expect((error as PreparationRefused).diagnostics[0].code).toBe("complexity_limit");
  }
}, 60_000);

test("an encounter the planner cannot place legally is refused by name, never started", () => {
  // No building stands within a metre of the objective's centre.
  const recipe = JSON.stringify({ ...assault, garrison: { ...assault.garrison, reach_m: 1 } });
  const refuse = () => prepareGeneratedBattle(wasm, memory, request({ recipe }));
  expect(refuse).toThrow(PreparationRefused);
  try {
    refuse();
  } catch (error) {
    const refused = error as PreparationRefused;
    expect(refused.stage).toBe("placing");
    // The summary first, then why each settlement tried was refused.
    expect(refused.diagnostics[0].code).toBe("no_objective");
    expect(refused.diagnostics[1].code).toBe("no_garrison_building");
    expect(refused.diagnostics[1].feature).toBe("forces.red[0]");
  }
}, 60_000);
