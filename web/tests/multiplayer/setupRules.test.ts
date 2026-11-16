// @vitest-environment node
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import { nodeSetupData } from "../../../apps/game-server/src/serverData";
import { prepareBattle } from "../../../apps/game-server/src/battleHost/runtime";
import { prepare } from "../../src/battle/prepare/prepare";
import { fromBattleSetup, toBattleSetup } from "../../../packages/multiplayer/src/setup";
import { battleRequest, preset, type SetupData } from "../../../packages/skirmish-setup/src/lobby";

// Only the document identities are needed to make a request, before generation.
const requestData: SetupData = {
  generator: {
    map_generator_version: () => "test-generator",
    template_catalogue_json: () => JSON.stringify({ hash: "test-template-hash" }),
    generate_map: () => {
      throw new Error("request construction does not generate a map");
    },
  },
  presets: JSON.stringify({ revision: "test-presets" }),
  templates: "[]",
  limits: { max_authored_parts: 1, max_bay_positions: 1, max_ground_points: 1 },
};

test("a setup on Frontline rules asks for them, and one on skirmish rules asks for exactly what an offline lobby does", () => {
  const offline = battleRequest(preset("1v1"), requestData);
  const request = (rules: "skirmish" | "frontline") =>
    battleRequest(fromBattleSetup(toBattleSetup(preset("1v1"), rules)), requestData);
  expect(request("skirmish")).toEqual(offline);
  const { rules, ...other } = request("frontline");
  expect(other).toEqual(offline);
  expect(rules).toBe("frontline");
});

test.each(["frontline", "skirmish"] as const)(
  "page and server prepare and run the selected %s rules on the same map",
  async (rules) => {
    const root = join(import.meta.dirname, "../../..");
    const data = await nodeSetupData(root);
    const { memory } = wasm.initSync({
      module: readFileSync(join(root, "web/src/wasm/game_wasm_bg.wasm")),
    });
    const setup = toBattleSetup(preset("1v1"), rules);
    const request = battleRequest(fromBattleSetup(setup), data);
    const page = await prepare(wasm, memory, request, data);
    const server = await prepareBattle(setup, data);
    const base = JSON.parse(readFileSync(join(root, "fixtures/skirmish.json"), "utf8"));
    const overrides =
      rules === "frontline"
        ? JSON.parse(readFileSync(join(root, "fixtures/frontline.json"), "utf8"))
        : {};
    // JSON numbers compare by value here; Rust's serialized f64 has no new identity.
    const expected = { ...base, ...overrides };
    const scenario = JSON.parse(page.scenario);
    const pageBattle = page.world.into_battle(page.scenario, setup.battleSeed);
    const serverBattle = server.sim.createBattle(server.scenario, server.seed);
    try {
      expect(scenario.skirmish.rules).toEqual(expected);
      expect(JSON.parse(server.scenario).skirmish.rules).toEqual(expected);
      expect(page.report.identity.kind).toBe("generated");
      if (page.report.identity.kind !== "generated") throw new Error("not a generated map");
      expect(server.mapIdentity).toBe(page.report.identity.generation.map_hash);
      expect(serverBattle.replay_json()).toBe(pageBattle.replay_json());
      for (let tick = 0; tick < scenario.rules.tick_hz; tick++) {
        pageBattle.step();
        serverBattle.step();
        expect(serverBattle.digest()).toBe(pageBattle.digest());
      }
    } finally {
      pageBattle.free();
      serverBattle.free();
    }
  },
  60_000,
);
