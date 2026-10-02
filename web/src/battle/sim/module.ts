import { publicWorldOf, type PublicWorldData } from "./publicWorld";
import init, * as wasm from "@wasm/game_wasm.js";
import type { SimModule } from "./authority";

export type Wasm = typeof wasm;

/** Wrap an initialised module in the authority's view of it. */
export function simModule(memory: WebAssembly.Memory): SimModule {
  let publicWorld: PublicWorldData | null = null;
  const prepare = (scenario: string) => {
    const prepared = wasm.PreparedWorld.from_scenario(scenario);
    try {
      publicWorld = publicWorldOf(prepared, prepared.layout());
      return prepared;
    } catch (error) {
      prepared.free();
      throw error;
    }
  };
  return {
    memory,
    takePublicWorld() {
      const world = publicWorld;
      publicWorld = null;
      return world;
    },
    createBattle(scenario, seed, script) {
      const prepared = prepare(scenario);
      return script === undefined
        ? prepared.into_battle(scenario, seed)
        : prepared.into_scripted(scenario, seed, script);
    },
    replayBattle: (scenario, replay) => prepare(scenario).into_replay(scenario, replay),
  };
}

let loaded: Promise<{ wasm: Wasm; memory: WebAssembly.Memory }> | null = null;

function load() {
  return (loaded ??= init().then((out) => ({ wasm, memory: out.memory })));
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
