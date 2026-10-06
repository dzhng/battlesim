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
  if (loaded) return loaded;
  loaded = (async () => {
    // Vitest runs the browser module in Node, where Vite's root-relative asset
    // URL cannot be passed to native fetch. The browser path remains a counted
    // fetch so production download accounting is unchanged.
    const moduleOrPath =
      typeof process !== "undefined" && process.versions?.node
        ? await import("node:fs/promises").then(({ readFile }) =>
            readFile(new URL("../../wasm/game_wasm_bg.wasm", import.meta.url)),
          )
        : countedFetch(wasmUrl);
    const out = await init({ module_or_path: moduleOrPath });
    if (!out) throw new Error("WASM simulation module did not initialize");
    return { wasm, memory: out.memory };
  })();
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
