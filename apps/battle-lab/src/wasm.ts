import init, * as wasm from "@wasm/game_wasm.js";

let ready: Promise<typeof wasm> | null = null;

/** The simulation module, initialised once per page. */
export function loadWasm(): Promise<typeof wasm> {
  return (ready ??= init().then(() => wasm));
}
