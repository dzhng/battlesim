// @vitest-environment node
import { expect, test } from "vitest";
import { prepare, type PreparationModule } from "../src/battle/prepare/prepare";
import type { PrepareBattleRequest } from "../src/battle/prepare/protocol";

test("faction preparation fields both factions on the admitted sites", async () => {
  const request: PrepareBattleRequest = {
    map_source: {
      kind: "generated",
      request: {
        type: "open",
        size: "small",
        profile: "skirmish",
        seed: "1",
        generator_version: "test",
        preset_revision: "test",
        template_catalog_hash: "test",
        limits: { max_authored_parts: 10, max_bay_positions: 10, max_ground_points: 10 },
      },
    },
    players: [
      { id: "blue-1", side: "blue", faction: "europe", controller: "human", human_capable: true },
      {
        id: "red-1",
        side: "red",
        faction: "eastern",
        controller: "active_ai",
        human_capable: false,
      },
    ],
    battle_seed: 1,
  };
  const map = { size: [1800, 1800], relief: [], props: [], surfaces: [], forests: [], bridges: [] };
  const sites = {
    skirmish: { entries: [{ side: "blue", center: [821, 1792], yaw: -1.4 }], objectives: [] },
  };
  const selected = {
    ...sites,
    skirmish: { ...sites.skirmish, entries: [{ side: "blue", center: [825, 1790], yaw: -1.4 }] },
  };
  let fieldInputs: unknown;
  const module = {
    check_prepare_request: (json: string) =>
      JSON.stringify({ status: "ok", request: JSON.parse(json) }),
    check_scenario_player_contract: () => JSON.stringify({ status: "ok" }),
    generate_map: () =>
      JSON.stringify({ status: "ok", result: { map, identity: { seed: "1" }, report: {}, sites } }),
    PreparedWorld: class {
      admit_skirmish() {
        return JSON.stringify({ sites: selected, journeys: [] });
      }
      skirmish_scenario(sitesJson: string, playersJson: string) {
        fieldInputs = [JSON.parse(sitesJson), JSON.parse(playersJson)];
        return JSON.stringify({
          map,
          rules: {},
          units: [],
          players: JSON.parse(playersJson),
          skirmish: { sites: JSON.parse(sitesJson).skirmish },
        });
      }
      extents() {
        return JSON.stringify({
          playable: [0, 0, 1800, 1800],
          physical: [0, 0, 1800, 1800],
          rendered: [0, 0, 1800, 1800],
        });
      }
      free() {}
    },
  } as unknown as PreparationModule;
  const prepared = await prepare(module, new WebAssembly.Memory({ initial: 1 }), request, {
    rules: "{}",
    presets: "{}",
    templates: "[]",
  });
  try {
    expect(fieldInputs).toEqual([selected, request.players]);
    expect(JSON.parse(prepared.scenario)).toMatchObject({
      units: [],
      players: request.players,
      skirmish: { sites: selected.skirmish },
    });
    // The camera starts at the admitted base, not the generator's.
    expect(prepared.report.start).toEqual({ at: [825, 1790], yaw: -1.4 });
  } finally {
    prepared.world.free();
  }
});

test("preparation starts the camera on the human player's lane", async () => {
  const request: PrepareBattleRequest = {
    map_source: {
      kind: "generated",
      request: {
        type: "open",
        size: "large",
        profile: "skirmish",
        seed: "1",
        generator_version: "test",
        preset_revision: "test",
        template_catalog_hash: "test",
        entries_per_side: 2,
        limits: { max_authored_parts: 10, max_bay_positions: 10, max_ground_points: 10 },
      },
    },
    players: [
      {
        id: "blue-1",
        side: "blue",
        faction: "europe",
        controller: "active_ai",
        human_capable: false,
      },
      { id: "blue-2", side: "blue", faction: "europe", controller: "human", human_capable: true },
      {
        id: "red-1",
        side: "red",
        faction: "eastern",
        controller: "active_ai",
        human_capable: false,
      },
    ],
    battle_seed: 1,
  };
  const map = { size: [3000, 3000], relief: [], props: [], surfaces: [], forests: [], bridges: [] };
  const sites = {
    skirmish: {
      entries: [
        { side: "blue", center: [500, 2990], yaw: -1.57 },
        { side: "blue", center: [1500, 2990], yaw: -1.57 },
        { side: "red", center: [500, 10], yaw: 1.57 },
        { side: "red", center: [1500, 10], yaw: 1.57 },
      ],
      objectives: [],
    },
  };
  const module = {
    check_prepare_request: (json: string) =>
      JSON.stringify({ status: "ok", request: JSON.parse(json) }),
    check_scenario_player_contract: () => JSON.stringify({ status: "ok" }),
    generate_map: () =>
      JSON.stringify({ status: "ok", result: { map, identity: { seed: "1" }, report: {}, sites } }),
    PreparedWorld: class {
      admit_skirmish() {
        return JSON.stringify({ sites, journeys: [] });
      }
      skirmish_scenario(sitesJson: string, playersJson: string) {
        return JSON.stringify({
          map,
          rules: {},
          units: [],
          players: JSON.parse(playersJson),
          skirmish: JSON.parse(sitesJson).skirmish,
        });
      }
      extents() {
        return JSON.stringify({
          playable: [0, 0, 3000, 3000],
          physical: [0, 0, 3000, 3000],
          rendered: [0, 0, 3000, 3000],
        });
      }
      free() {}
    },
  } as unknown as PreparationModule;
  const prepared = await prepare(module, new WebAssembly.Memory({ initial: 1 }), request, {
    rules: "{}",
    presets: "{}",
    templates: "[]",
  });
  expect(prepared.report.start).toEqual({ at: [1500, 2990], yaw: -1.57 });
  prepared.world.free();
});
