// @vitest-environment node
import { expect, test } from "vitest";
import {
  createBuilderDraft,
  parseBuilderDraft,
  cloneDraft,
  removeBuilderUnit,
} from "../../apps/battle-lab/src/scenario-builder/model";
import {
  canonicalScenarioBundle,
  validateScenarioBundle,
  type ScenarioBundleWasm,
} from "../src/battle/scenarioBundle";

const rules = { catalog: [] } as never;
const units = { ids: ["blue-scout", "red-scout"], type: (id: string) => ({ name: id }) } as never;

test("builder draft round trips through the explicit JSON boundary", () => {
  const draft = createBuilderDraft(rules, units);
  expect(draft.players.find((player) => player.side === "neutral")).not.toHaveProperty("faction");
  draft.scenario.units[0].position = [24, 42];
  const parsed = parseBuilderDraft(JSON.stringify(draft));
  expect(parsed.scenario.units[0].position).toEqual([24, 42]);
  expect(cloneDraft(parsed)).toEqual(parsed);
});

test("removing a unit keeps editor ids aligned with canonical player ownership", () => {
  const draft = createBuilderDraft(rules, units);
  draft.scenario.units.push({ id: 2, side: "blue", kind: "blue-scout", position: [1, 1], yaw: 0 });
  draft.players[0].units.push(2);
  const removed = removeBuilderUnit(draft, 1);
  expect(removed.scenario.units.map((unit) => unit.id)).toEqual([0, 1]);
  expect(removed.players.map((player) => player.units)).toEqual([[0, 1], [], []]);
});

test("bundle upload refuses invalid bytes and returns the owner identity", () => {
  const wasm: ScenarioBundleWasm = {
    check_scenario_bundle: (json) =>
      json === "valid"
        ? JSON.stringify({
            status: "ok",
            identity: { sha256: "a", map_hash: "b", rules_hash: "c" },
          })
        : JSON.stringify({
            status: "error",
            diagnostics: [{ code: "malformed", location: "$", message: "invalid package" }],
          }),
    canonical_scenario_bundle: () => "valid",
  };
  expect(validateScenarioBundle(wasm, "valid").sha256).toBe("a");
  expect(canonicalScenarioBundle(wasm, "valid")).toBe("valid");
  expect(() => validateScenarioBundle(wasm, "bad")).toThrow("invalid package");
});
