import init, * as wasm from "@wasm/game_wasm.js";
import wasmUrl from "@wasm/game_wasm_bg.wasm?url";
import { countedFetch } from "../../downloads";
import type { SimModule } from "./authority";

export type Wasm = typeof wasm;

/** Wrap an initialised module in the authority's view of it. */
export function simModule(memory: WebAssembly.Memory): SimModule {
  return {
    memory,
    createBattle: (scenario, seed, script) =>
      script === undefined
        ? new wasm.Battle(scenario, seed)
        : wasm.Battle.scripted(scenario, seed, script),
    replayBattle: (scenario, replay) => wasm.Battle.from_replay(scenario, replay),
  };
}

let loaded: Promise<{ wasm: Wasm; memory: WebAssembly.Memory }> | null = null;

function load() {
  return (loaded ??= init({ module_or_path: countedFetch(wasmUrl) }).then((out) => ({
    wasm,
    memory: out.memory,
  })));
}

/** The simulation module, initialised once per realm (page or worker): the
 *  static-map queries and scenario builders the views use. */
export function loadWasm(): Promise<Wasm> {
  return load().then((m) => m.wasm);
}

/** The authority's view of the module, initialised once per realm. */
export function loadSimModule(): Promise<SimModule> {
  return load().then((m) => simModule(m.memory));
}
