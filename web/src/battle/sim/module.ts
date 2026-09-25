import init, { Battle, observation_layout } from "@wasm/game_wasm.js";
import type { SimModule } from "./authority";

/** Wrap an initialised module in the authority's view of it. */
export function simModule(memory: WebAssembly.Memory): SimModule {
  return {
    memory,
    observation_layout,
    createBattle: (scenario, seed) => new Battle(scenario, seed),
    replayBattle: (scenario, replay) => Battle.from_replay(scenario, replay),
  };
}

let loaded: Promise<SimModule> | null = null;

/** Load the simulation module once per realm (page or worker). */
export function loadSimModule(): Promise<SimModule> {
  return (loaded ??= init().then((out) => simModule(out.memory)));
}
