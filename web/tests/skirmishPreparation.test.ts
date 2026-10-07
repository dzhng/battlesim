// @vitest-environment node
import { expect, test } from "vitest";
import { prepare, type PreparationModule } from "../src/battle/prepare/prepare";
import type { PrepareBattleRequest } from "../src/battle/prepare/protocol";

test("the exact battle address preserves the compact geometry profile", async () => {
  const { askedBattle, battleHref } = await import("@apps/battle-lab/src/battleLinks");
  const map = {
    type: "open" as const,
    size: "small" as const,
    profile: "skirmish" as const,
    seed: "17",
  };
  expect(
    askedBattle(new URL(battleHref({ ...map, faction: "us" }), "http://game").search),
  ).toMatchObject({ map, factions: ["us", "eastern"] });
});

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
    factions: ["europe", "eastern"],
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
      free() {}
    },
  } as unknown as PreparationModule;
  const prepared = await prepare(
    module,
    new WebAssembly.Memory({ initial: 1 }),
    request,
    { rules: "{}", presets: "{}", templates: "[]" },
    {
      loadMap: () => {
        throw new Error("no saved map");
      },
    },
  );
  try {
    expect(fieldInputs).toEqual([selected, ["europe", "eastern"]]);
    expect(JSON.parse(prepared.scenario)).toMatchObject({
      units: [],
      skirmish: { factions: ["europe", "eastern"], sites: selected.skirmish },
    });
    // The camera starts at the admitted base, not the generator's.
    expect(prepared.report.start).toEqual({ at: [825, 1790], yaw: -1.4 });
  } finally {
    prepared.world.free();
  }
});
