// @vitest-environment node
import { expect, test } from "vitest";
import {
  createBuilderDraft,
  parseBuilderDraft,
  cloneDraft,
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
  draft.scenario.units[0].position = [24, 42];
  const parsed = parseBuilderDraft(JSON.stringify(draft));
  expect(parsed.scenario.units[0].position).toEqual([24, 42]);
  expect(cloneDraft(parsed)).toEqual(parsed);
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
