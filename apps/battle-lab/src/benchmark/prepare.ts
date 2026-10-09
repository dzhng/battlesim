// Benchmark selection enters the same map/preparation worker as a battle.
// The city arena is an explicit synthetic contact workload, never a moved
// player encounter or a second map generator.
import config from "@fixtures/generated-battle.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import { generationRequest } from "@web/maps/source";
import { prepareBattle, type PreparedSession } from "@web/battle/prepare/client";
import type { PrepareMessage } from "@web/battle/prepare/protocol";
import type { Wasm } from "@web/battle/sim/module";
import { cityContactTour } from "@web/battle/benchmark/camera";
import type { BenchmarkPreset, BenchmarkScenario } from "@web/battle/benchmark/presets";
import type { GameRules } from "@web/battle/catalog/compose";

export function benchmarkPreparation(
  wasm: Wasm,
  scenario: BenchmarkPreset,
  rules: GameRules,
): PrepareMessage {
  return {
    type: "prepare",
    request: {
      map_source: {
        kind: "generated",
        request: generationRequest(wasm, scenario.generated, { presets, templates }, config.limits),
      },
      // The stress scene fields its own forces; the request still names two.
      factions: ["us", "eastern"],
      battle_seed: scenario.seed,
    },
    documents: { presets, templates, rules: JSON.stringify(rules) },
    stress: { kind: scenario.variant, late: false },
  };
}

export interface BenchmarkBattle {
  scenario: string;
  workload: BenchmarkScenario;
  prepared?: PreparedSession;
}

export async function prepareBenchmark(
  wasm: Wasm,
  workload: BenchmarkPreset,
  signal: AbortSignal,
  rules: GameRules,
): Promise<BenchmarkBattle> {
  const preparation = prepareBattle(benchmarkPreparation(wasm, workload, rules), () => {});
  signal.addEventListener("abort", preparation.cancel, { once: true });
  if (signal.aborted) preparation.cancel();
  const prepared = await preparation.battle;
  return {
    scenario: prepared.scenario,
    prepared,
    workload: {
      ...workload,
      tour: cityContactTour(prepared.report.size, prepared.report.extents.rendered),
    },
  };
}
