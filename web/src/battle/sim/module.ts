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
  if (!loaded) {
    // Init reads the download only when the module is not set up yet (a
    // test sets it up from disk): a fetch it never reads must not fail the
    // realm, and one it does read still fails init.
    const download = countedFetch(wasmUrl);
    download.catch(() => {});
    loaded = init({ module_or_path: download }).then((out) => ({ wasm, memory: out.memory }));
  }
  return loaded;
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
