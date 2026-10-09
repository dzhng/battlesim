// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import config from "@fixtures/generated-battle.json";
import { TEST_RULES } from "./catalog";
import { PreparationRefused, prepare, prepareReplay } from "../src/battle/prepare/prepare";
import { parseReplayFile } from "@apps/battle-lab/src/replayFile";
import type { PrepareBattleRequest, PrepareDocuments } from "../src/battle/prepare/protocol";
import { generationRequest, type MapChoice } from "../src/maps/source";

const fixture = (path: string) =>
  readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8");

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = wasm.initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

const rules = JSON.stringify(TEST_RULES);
const documents: PrepareDocuments = {
  rules,
  presets: fixture("map-presets.json"),
  templates: fixture("prototype-building-templates.json"),
};

const OPEN: MapChoice = { type: "open", size: "medium", seed: "1", profile: "skirmish" };
/** Europe against Eastern on the generated map `choice`, under the game's
 *  limits unless told. */
const request = (
  choice: MapChoice = OPEN,
  over: Partial<PrepareBattleRequest> = {},
  limits = config.limits,
): PrepareBattleRequest => ({
  map_source: { kind: "generated", request: generationRequest(wasm, choice, documents, limits) },
  factions: ["europe", "eastern"],
  battle_seed: 1,
  ...over,
});
const prepared = (r = request(), d = documents, onStage?: (stage: string) => void) =>
  prepare(wasm, memory, r, d, onStage).then(({ world, ...battle }) => {
    world.free();
    return battle;
  });

test("a saved prepared battle replays its captured map, forces and rules after fixture edits", async () => {
  const captured = await prepared(request(OPEN, { battle_seed: 7 }));
  const live = new wasm.Battle(captured.scenario, 7);
  try {
    for (let tick = 0; tick < 30; tick++) live.step();
    const file = parseReplayFile(JSON.stringify({ battle: captured, replay: live.replay_json() }));
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
    // Compact skirmish geography: a medium map is 2400 m square.
    expect(result.report.size).toEqual([2400, 2400]);
    expect(result.report.extents).toEqual({
      playable: [0, 0, 2400, 2400],
      physical: [0, 0, 2400, 2400],
      rendered: [-500, -500, 2900, 2900],
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
  map: unknown;
  units: unknown[];
  skirmish: { factions: [string, string]; sites: { entries: SkirmishEntry[] } };
}
interface SkirmishEntry {
  side: string;
  center: [number, number];
  yaw: number;
}

test("a prepared battle fields both asked factions on the requested map, and it runs", async () => {
  const stages: string[] = [];
  const asked = request();
  const { scenario: json, report } = await prepared(asked, documents, (s) => stages.push(s));
  expect(stages).toEqual(["map", "encounter"]);
  expect(report.request).toEqual(asked);
  if (report.identity.kind !== "generated") throw new Error("a generated map was asked for");
  const identity = report.identity.generation;
  expect(identity.seed).toBe("1");

  const scenario = JSON.parse(json) as Scenario;
  // The scenario carries exactly the map the generator makes for that request.
  if (asked.map_source.kind !== "generated") throw new Error("unreachable");
  const direct = JSON.parse(
    wasm.generate_map(
      JSON.stringify(asked.map_source.request),
      documents.presets,
      documents.templates,
      rules,
    ),
  );
  expect(identity).toEqual(direct.result.identity);
  expect(scenario.map).toEqual(direct.result.map);

  // Both factions, as asked, and no army laid in advance: the match starts
  // in preparation from each side's admitted base, where the camera starts.
  expect(scenario.skirmish.factions).toEqual(["europe", "eastern"]);
  expect(scenario.units).toEqual([]);
  const base = scenario.skirmish.sites.entries.find((e) => e.side === "blue")!;
  expect(report.start).toEqual({ at: base.center, yaw: base.yaw });

  const battle = new wasm.Battle(json, 1);
  for (let tick = 0; tick < 30; tick++) battle.step();
  expect(battle.tick()).toBe(30);
  battle.free();
}, 60_000);

test("the same request prepares the same battle; another map seed another map", async () => {
  const a = await prepared();
  const b = await prepared();
  expect(b.scenario).toBe(a.scenario);
  // The next seed that prepares: a seed's map or its sites may be refused
  // by name, so which one that is belongs to the generator, not this test.
  let otherMap: Awaited<ReturnType<typeof prepared>> | undefined;
  for (let seed = 2; seed <= 6 && !otherMap; seed++) {
    otherMap = await prepared(request({ ...OPEN, seed: String(seed) })).catch((error) => {
      if (!(error instanceof PreparationRefused)) throw error;
      return undefined;
    });
  }
  expect(otherMap!.report.identity).not.toEqual(a.report.identity);
}, 60_000);

test("a map seed a JavaScript number cannot hold reaches the generator as written", async () => {
  // 2^53 + 1: a number would round it to 2^53.
  const { report } = await prepared(request({ ...OPEN, seed: "9007199254740993" }));
  if (report.identity.kind !== "generated") throw new Error("a generated map was asked for");
  expect(report.identity.generation.seed).toBe("9007199254740993");
  const rounded = await prepared(request({ ...OPEN, seed: "9007199254740992" }));
  expect(rounded.report.identity).not.toEqual(report.identity);
}, 60_000);

test("a request the simulation's check refuses prepares nothing, and names the field", async () => {
  const { factions: _, ...faceless } = request();
  for (const [asked, location, names] of [
    [request(OPEN, { battle_seed: 2 ** 53 }), "$.battle_seed", /battle seed/],
    [request(OPEN, { battle_seed: 1.5 }), "$", /./],
    [faceless, "$", /factions/],
    [{ ...request(), recipe_id: "assault" }, "$", /recipe_id/],
    [{ ...request(), encounter_seed: "1" }, "$", /encounter_seed/],
  ] as const) {
    const refused = await refusal(prepared(asked as PrepareBattleRequest));
    expect(refused.stage).toBe("request");
    expect(refused.diagnostics[0].location).toBe(location);
    expect(refused.diagnostics[0].message).toMatch(names);
  }
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

test("a map without skirmish sites cannot field the factions, and a saved map is no battle's", async () => {
  // Standard geography has a base for neither side.
  const refused = await refusal(prepared(request({ ...OPEN, profile: "standard" })));
  expect(refused.stage).toBe("encounter");
  expect(refused.diagnostics[0]).toMatchObject({
    feature: "skirmish_sites",
    location: "$.map_source",
  });
  // A saved map (a test's or the menu's) is no source a battle can name.
  const saved = await refusal(
    prepared({
      ...request(),
      map_source: { kind: "catalogue", id: "market-town" },
    } as unknown as PrepareBattleRequest),
  );
  expect(saved.stage).toBe("request");
  expect(saved.diagnostics[0].message).toContain("unknown variant `catalogue`");
}, 60_000);
