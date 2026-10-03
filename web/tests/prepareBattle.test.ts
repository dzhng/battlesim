// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import config from "@fixtures/generated-battle.json";
import encounters from "@fixtures/encounters.json";
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";
import { PreparationRefused, prepare, prepareReplay } from "../src/battle/prepare/prepare";
import { parseReplayFile, isPreparedReplay } from "@apps/battle-lab/src/replayFile";
import type { PrepareBattleRequest, PrepareDocuments } from "../src/battle/prepare/protocol";
import { loadEncounter, loadMap } from "../src/maps/node";
import { generationRequest, type MapChoice } from "../src/maps/source";
import type { WorldLayout } from "@packages/battle-renderer/src/worldMesh";

const fixture = (path: string) =>
  readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8");

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = wasm.initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

const rules = JSON.stringify(GAME_RULES);
const assault = encounters.recipes.assault;
const documents: PrepareDocuments = {
  rules,
  presets: fixture("map-presets.json"),
  templates: fixture("prototype-building-templates.json"),
  recipes: fixture("encounters.json"),
};
const saved = { loadMap, loadEncounter };

test("the game plans a recipe the recipes file holds", () => {
  expect(Object.keys(encounters.recipes)).toContain(config.encounter.recipe);
  expect(config.encounter.seed).toMatch(/^(0|[1-9]\d{0,19})$/);
});

const OPEN: MapChoice = { type: "open", size: "small", seed: "1" };
/** A battle on the generated map `choice`, under the game's limits unless
 *  told. */
const request = (
  choice: MapChoice = OPEN,
  over: Partial<PrepareBattleRequest> = {},
  limits = config.limits,
): PrepareBattleRequest => ({
  map_source: { kind: "generated", request: generationRequest(wasm, choice, documents, limits) },
  recipe_id: "assault",
  encounter_seed: config.encounter.seed,
  battle_seed: 1,
  ...over,
});
const prepared = (r = request(), d = documents, onStage?: (stage: string) => void) =>
  prepare(wasm, memory, r, d, saved, onStage).then(({ world, ...battle }) => {
    world.free();
    return battle;
  });

test("a saved prepared battle replays its captured map, encounter and rules after fixture edits", async () => {
  const captured = await prepared({
    map_source: { kind: "catalogue", id: "village" },
    recipe_id: "lean",
    encounter_seed: "1",
    battle_seed: 7,
  });
  const live = new wasm.Battle(captured.scenario, 7);
  try {
    for (let tick = 0; tick < 30; tick++) live.step();
    const file = parseReplayFile(JSON.stringify({ battle: captured, replay: live.replay_json() }));
    if (!isPreparedReplay(file)) throw new Error("not a prepared replay");
    const changed = JSON.parse(documents.rules);
    changed.tick_hz *= 2;
    const edited = await prepared(captured.report.request, {
      ...documents,
      rules: JSON.stringify(changed),
    });
    expect(edited.scenario).not.toBe(captured.scenario);
    const regenerated = wasm.PreparedWorld.from_scenario(edited.scenario);
    // into_replay consumes the prepared world even when identity checks refuse it.
    expect(() => regenerated.into_replay(edited.scenario, file.replay)).toThrow();
    const playback = prepareReplay(wasm, file.battle);
    expect(playback.scenario).toBe(captured.scenario);
    const replay = playback.world.into_replay(playback.scenario, file.replay);
    try {
      for (let tick = 0; tick < 30; tick++) replay.step();
      expect(replay.digest()).toBe(live.digest());
      const stale = JSON.parse(file.replay);
      stale.engine_build = "another-build";
      const other = wasm.PreparedWorld.from_scenario(playback.scenario);
      expect(() => other.into_replay(playback.scenario, JSON.stringify(stale))).toThrow();
    } finally {
      replay.free();
    }
  } finally {
    live.free();
  }
});

test("preparation publishes the compiled map's derived bounds without widening encounter space", async () => {
  const presets = JSON.parse(documents.presets);
  presets.terrain.render_margin_m = 500;
  const result = await prepared(request(), { ...documents, presets: JSON.stringify(presets) });
  const map = JSON.parse(result.scenario).map;
  const view = new wasm.WorldView(JSON.stringify(map), rules);
  try {
    expect(result.report.size).toEqual([6000, 6000]);
    expect(result.report.extents).toEqual({
      playable: [0, 0, 6000, 6000],
      physical: [0, 0, 6000, 6000],
      rendered: [-500, -500, 6500, 6500],
    });
    expect(JSON.parse(view.extents())).toEqual(result.report.extents);
    expect(map.render_margin_m).toBe(500);
  } finally {
    view.free();
  }
});
/** The refusal `preparing` ends in. */
async function refusal(preparing: Promise<unknown>): Promise<PreparationRefused> {
  const error = await preparing.then(
    () => null,
    (e: unknown) => e,
  );
  expect(error).toBeInstanceOf(PreparationRefused);
  return error as PreparationRefused;
}

interface Scenario {
  map: { size: [number, number]; buildings: { owner: number; kind: string }[] };
  units: { side: string; kind: string; position: [number, number] }[];
  scripts: { tick: number; side: string }[];
  opponent: { side: string; garrisons: [number, number][] };
  encounter: { attacker: string; success_zone_center: [number, number] };
}

test("a prepared battle is the requested map with the planned encounter on it, and it runs", async () => {
  const stages: string[] = [];
  const asked = request();
  const { scenario: json, report } = await prepared(asked, documents, (s) => stages.push(s));
  expect(stages).toEqual(["map", "encounter"]);
  expect(report.request).toEqual(asked);
  if (report.identity.kind !== "generated" || !report.planned)
    throw new Error("a generated map with a planned encounter was asked for");
  const identity = report.identity.generation;
  const { placement } = report.planned;
  expect(identity.seed).toBe("1");
  expect(report.size).toEqual([6000, 6000]);
  expect(report.planned.encounter_seed).toBe(config.encounter.seed);
  expect(report.planned.recipe_hash).toMatch(/^[0-9a-f]{64}$/);

  const scenario = JSON.parse(json) as Scenario;
  // The scenario carries exactly the map the generator makes for that request.
  const direct = JSON.parse(
    wasm.generate_map(
      JSON.stringify({
        generator_version: wasm.map_generator_version(),
        preset_revision: JSON.parse(fixture("map-presets.json")).revision,
        seed: "1",
        template_catalog_hash: identity.template_catalog_hash,
        type: "open",
        size: "small",
        limits: config.limits,
      }),
      fixture("map-presets.json"),
      fixture("prototype-building-templates.json"),
      JSON.stringify(GAME_RULES),
    ),
  );
  expect(identity).toEqual(direct.result.identity);
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
  const [attack, defence] = placement.deployments;
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
  expect(scenario.encounter.success_zone_center).toEqual(placement.objective.center);
  expect(report.objective).toEqual({
    center: placement.objective.center,
    radius_m: assault.objective.zone_radius_m,
    hold_s: assault.objective.hold_s,
  });
  expect(report.start.at).toEqual(blue[0].position);
  const town = placement.objective.center;
  const toTown = (u: { position: [number, number] }) =>
    Math.hypot(u.position[0] - town[0], u.position[1] - town[1]);
  expect(Math.min(...attack.units.map((id) => toTown(scenario.units[id])))).toBe(toTown(blue[0]));

  // Every garrison row names a red squad and a building of its own that a
  // squad can hold, with a seat for each soldier.
  const buildings = new Set(scenario.map.buildings.map((b) => b.owner));
  expect(scenario.opponent.side).toBe("red");
  expect(scenario.opponent.garrisons.length).toBe(
    assault.forces.red.filter((r) => r.post === "garrison").length,
  );
  expect(new Set(scenario.opponent.garrisons.map(([, building]) => building)).size).toBe(
    scenario.opponent.garrisons.length,
  );
  for (const [unit, building] of scenario.opponent.garrisons) {
    expect(scenario.units[unit].side).toBe("red");
    expect(buildings.has(building)).toBe(true);
  }
  expect(scenario.opponent.garrisons).toEqual(placement.garrisons.map((g) => [g.unit, g.building]));
  for (const g of placement.garrisons) expect(g.seats).toBeGreaterThanOrEqual(g.soldiers);
  view.free();

  // The simulation takes it: the scripted relief column and the defender's
  // garrison orders are accepted, and the battle steps.
  expect(scenario.scripts.map((s) => [s.tick, s.side])).toEqual([[0, "red"]]);
  const battle = new wasm.Battle(json, 1);
  for (let tick = 0; tick < 300; tick++) battle.step();
  expect(battle.tick()).toBe(300);
  for (const [unit, building] of scenario.opponent.garrisons) {
    const placement = JSON.parse(
      battle.preview_building("red", JSON.stringify({ units: [unit], building })),
    );
    expect(placement.building).toBe(building);
    expect(placement.entrant?.unit).toBe(unit);
  }
  battle.free();
}, 60_000);

test("the same request prepares the same battle; the map seed and the encounter seed are separate", async () => {
  const a = await prepared();
  const b = await prepared();
  expect(b.scenario).toBe(a.scenario);

  const otherMap = await prepared(request({ ...OPEN, seed: "2" }));
  expect(otherMap.report.identity).not.toEqual(a.report.identity);

  // Another encounter seed on the same map: the same map and columns, and
  // (over a few seeds) another choice of garrison buildings.
  const garrisons = new Set([
    JSON.stringify(a.report.planned!.placement.garrisons.map((g) => g.building)),
  ]);
  for (const encounter_seed of ["2", "3"]) {
    const other = await prepared(request(OPEN, { encounter_seed }));
    expect(other.report.identity).toEqual(a.report.identity);
    expect(other.report.planned!.placement.deployments).toEqual(
      a.report.planned!.placement.deployments,
    );
    garrisons.add(JSON.stringify(other.report.planned!.placement.garrisons.map((g) => g.building)));
  }
  expect(garrisons.size).toBeGreaterThan(1);
}, 60_000);

test("seeds a JavaScript number cannot hold reach the generator and the planner as written", async () => {
  // 2^53 + 1 and the largest u64: a number would round the first to 2^53.
  const { report } = await prepared(
    request({ ...OPEN, seed: "9007199254740993" }, { encounter_seed: "18446744073709551615" }),
  );
  if (report.identity.kind !== "generated") throw new Error("a generated map was asked for");
  expect(report.identity.generation.seed).toBe("9007199254740993");
  expect(report.planned!.encounter_seed).toBe("18446744073709551615");
  const rounded = await prepared(request({ ...OPEN, seed: "9007199254740992" }));
  expect(rounded.report.identity).not.toEqual(report.identity);
}, 60_000);

test("a request the simulation's check refuses prepares nothing, and names the field", async () => {
  for (const [over, location] of [
    [{ encounter_seed: "01" }, "$"],
    [{ battle_seed: 2 ** 53 }, "$.battle_seed"],
    [{ battle_seed: 1.5 }, "$"],
    [{ recipe_id: "../assault" }, "$.recipe_id"],
  ] as const) {
    const refused = await refusal(prepared(request(OPEN, over)));
    expect(refused.stage).toBe("request");
    expect(refused.diagnostics[0].location).toBe(location);
  }
  // A recipe the recipes file does not hold.
  const unknown = await refusal(prepared(request(OPEN, { recipe_id: "siege" })));
  expect(unknown.stage).toBe("encounter");
  expect(unknown.diagnostics[0].location).toBe("$.recipe_id");
}, 60_000);

test("a refused map reports the generator's diagnostics and prepares nothing", async () => {
  const refused = await refusal(
    prepared(request(OPEN, {}, { ...config.limits, max_authored_parts: 10 })),
  );
  expect(refused.stage).toBe("map");
  expect(refused.diagnostics[0].code).toBe("complexity_limit");
  // A request pinned to another generator is refused, never regenerated.
  const stale = request();
  if (stale.map_source.kind !== "generated") throw new Error("unreachable");
  stale.map_source.request.generator_version = "layout-0";
  const other = await refusal(prepared(stale));
  expect(other.stage).toBe("map");
  expect(other.diagnostics[0].message).toContain('pins "layout-0"');
}, 60_000);

test("an encounter the planner cannot place legally is refused by name, never started", async () => {
  // No building stands within a metre of the objective's centre.
  const recipes = JSON.stringify({
    ...encounters,
    recipes: { assault: { ...assault, garrison: { ...assault.garrison, reach_m: 1 } } },
  });
  const refused = await refusal(prepared(request(), { ...documents, recipes }));
  expect(refused.stage).toBe("encounter");
  // The summary first, then why each settlement tried was refused.
  expect(refused.diagnostics[0].code).toBe("no_objective");
  expect(refused.diagnostics[1].code).toBe("no_garrison_building");
  expect(refused.diagnostics[1].feature).toBe("forces.red[0]");
}, 60_000);

test("the saved generated map plays its saved assault from the catalogue, and it runs", async () => {
  const { scenario: json, report } = await prepared({
    map_source: { kind: "catalogue", id: "market-town" },
    recipe_id: "assault",
    encounter_seed: "1",
    battle_seed: 1,
  });
  // The map is the catalogue's own generated map, and the scenario carries
  // the resolver's text of it: its buildings materialized, no number
  // reprinted by JavaScript.
  const town = loadMap("market-town");
  expect(town.identity.kind).toBe("generated");
  expect(report.identity).toEqual(town.identity);
  expect(json.startsWith(`{"map":${town.json},"rules":`)).toBe(true);
  expect(report.counts.buildings).toBe(town.definition.buildings!.length);
  expect(report.counts.buildings).toBeGreaterThan(1_000);
  // The saved encounter is a whole battle: both sides, and a town to take.
  const scenario = JSON.parse(json) as Scenario;
  expect(scenario.units).toEqual(loadEncounter("market-town", "assault").units);
  expect(new Set(scenario.units.map((u) => u.side))).toEqual(new Set(["blue", "red"]));
  expect(report.objective?.center).toEqual(scenario.encounter.success_zone_center);
  const battle = new wasm.Battle(json, 1);
  for (let tick = 0; tick < 30; tick++) battle.step();
  expect(battle.tick()).toBe(30);
  battle.free();
}, 60_000);

test("a catalogue map plays its saved encounter through the same preparation, and it runs", async () => {
  const { scenario: json, report } = await prepared({
    map_source: { kind: "catalogue", id: "village" },
    recipe_id: "lean",
    encounter_seed: "1",
    battle_seed: 1,
  });
  // The map is the catalogue's own, under the identity its sources pin.
  const village = loadMap("village");
  expect(report.identity).toEqual(village.identity);
  const scenario = JSON.parse(json) as Scenario;
  expect(scenario.map).toEqual(village.definition);
  expect(scenario.units).toEqual(loadEncounter("village", "lean").units);
  expect(report.planned).toBeNull();
  expect(report.start.at).toEqual(scenario.units.find((u) => u.side === "blue")!.position);
  expect(report.objective).toBeNull();
  const battle = new wasm.Battle(json, 1);
  for (let tick = 0; tick < 30; tick++) battle.step();
  expect(battle.tick()).toBe(30);
  battle.free();

  // A map or an encounter the catalogue lacks is refused at its own stage.
  const noMap = await refusal(
    prepared({ ...report.request, map_source: { kind: "catalogue", id: "nowhere" } }),
  );
  expect([noMap.stage, noMap.diagnostics[0].code]).toEqual(["map", "missing_document"]);
  const noEncounter = await refusal(prepared({ ...report.request, recipe_id: "siege" }));
  expect([noEncounter.stage, noEncounter.diagnostics[0].code]).toEqual([
    "encounter",
    "missing_document",
  ]);
}, 60_000);
