// @vitest-environment node
import { expect, test } from "vitest";
import { prepare, type PreparationModule } from "../src/battle/prepare/prepare";
import type { PrepareBattleRequest } from "../src/battle/prepare/protocol";

test("an empty prepared army frames its admitted own road-edge base", async () => {
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
    recipe_id: "empty",
    encounter_seed: "1",
    battle_seed: 1,
  };
  const map = { size: [1800, 1800], relief: [], props: [], surfaces: [], forests: [], bridges: [] };
  const sites = {
    settlements: [],
    approaches: [],
    skirmish: {
      entries: [
        { side: "blue", center: [821, 1792], yaw: -1.4 },
        { side: "red", center: [900, 8], yaw: 1.2 },
      ],
      objectives: [],
    },
  };
  const selected = structuredClone(sites);
  selected.skirmish.entries[0]!.center = [825, 1790];
  let plannedSites: unknown;
  const module = {
    check_prepare_request: (json: string) =>
      JSON.stringify({ status: "ok", request: JSON.parse(json) }),
    generate_map: () =>
      JSON.stringify({ status: "ok", result: { map, identity: { seed: "1" }, report: {}, sites } }),
    PreparedWorld: class {
      admit_skirmish() {
        return JSON.stringify({ sites: selected, journeys: [] });
      }
      extents() {
        return JSON.stringify({
          playable: [0, 0, 1800, 1800],
          physical: [0, 0, 1800, 1800],
          rendered: [0, 0, 1800, 1800],
        });
      }
      plan_encounter(sitesJson: string) {
        plannedSites = JSON.parse(sitesJson);
        return JSON.stringify({
          status: "ok",
          encounter: {
            recipe_hash: "test",
            encounter_seed: "1",
            setup: { units: [], scripts: [], opponent: "none", encounter: null },
            placement: { deployments: [] },
          },
        });
      }
      free() {}
    },
  } as unknown as PreparationModule;
  const prepared = await prepare(
    module,
    new WebAssembly.Memory({ initial: 1 }),
    request,
    { rules: "{}", presets: "{}", templates: "[]", recipes: '{"recipes":{"empty":{}}}' },
    {
      loadMap: () => {
        throw new Error("no saved map");
      },
      loadEncounter: () => {
        throw new Error("no saved encounter");
      },
    },
  );
  try {
    expect(prepared.report.start).toEqual({ at: [825, 1790], yaw: -1.4 });
    expect(plannedSites).toEqual(selected);
    expect(JSON.parse(prepared.scenario).units).toEqual([]);
  } finally {
    prepared.world.free();
  }
});

test("the exact battle address preserves the compact geometry profile", async () => {
  const { askedBattle, battleHref } = await import("@apps/battle-lab/src/battleLinks");
  const map = {
    type: "open" as const,
    size: "small" as const,
    profile: "skirmish" as const,
    seed: "17",
  };
  expect(askedBattle(new URL(battleHref(map), "http://game").search)).toMatchObject({ map });
});

test("faction preparation bypasses authored forces and uses the admitted sites", async () => {
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
    skirmish: ["europe", "eastern"],
    recipe_id: "unused",
    encounter_seed: "1",
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
    generate_map: () =>
      JSON.stringify({ status: "ok", result: { map, identity: { seed: "1" }, report: {}, sites } }),
    PreparedWorld: class {
      admit_skirmish() {
        return JSON.stringify({ sites: selected, journeys: [] });
      }
      skirmish_fields(sitesJson: string, factionsJson: string) {
        fieldInputs = [JSON.parse(sitesJson), JSON.parse(factionsJson)];
        return JSON.stringify({
          units: [],
          skirmish: { sites: JSON.parse(sitesJson).skirmish, factions: JSON.parse(factionsJson) },
        });
      }
      extents() {
        return JSON.stringify({
          playable: [0, 0, 1800, 1800],
          physical: [0, 0, 1800, 1800],
          rendered: [0, 0, 1800, 1800],
        });
      }
      plan_encounter() {
        throw new Error("skirmishes must not lay an authored army");
      }
      free() {}
    },
  } as unknown as PreparationModule;
  const prepared = await prepare(
    module,
    new WebAssembly.Memory({ initial: 1 }),
    request,
    { rules: "{}", presets: "{}", templates: "[]", recipes: '{"recipes":{}}' },
    {
      loadMap: () => {
        throw new Error("no saved map");
      },
      loadEncounter: () => {
        throw new Error("no saved encounter");
      },
    },
  );
  try {
    expect(fieldInputs).toEqual([selected, ["europe", "eastern"]]);
    expect(JSON.parse(prepared.scenario)).toMatchObject({
      units: [],
      skirmish: { factions: ["europe", "eastern"], sites: selected.skirmish },
    });
    expect(prepared.report.start.at).toEqual([825, 1790]);
    expect(prepared.report.planned).toBeNull();
  } finally {
    prepared.world.free();
  }
});
