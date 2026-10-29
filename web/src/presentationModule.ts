/** One WebAssembly instance per realm, shared by presentation, render and audio. */
import init from "./wasm/renderer_browser/renderer_browser.js";
import wasmUrl from "./wasm/renderer_browser/renderer_browser_bg.wasm?url";
import { countedFetch } from "./downloads";
let loaded: Promise<{ memory: WebAssembly.Memory }> | null = null;
let memory: WebAssembly.Memory | null = null;

/** Initialises the module once per realm; decoders need it first. */
export function loadPresentationModule(): Promise<{ memory: WebAssembly.Memory }> {
  // Vitest runs in Node, where Vite's asset URL cannot be fetched (module.ts).
  loaded ??= (
    typeof process !== "undefined" && process.versions?.node
      ? import("node:fs/promises")
          .then(({ readFile }) =>
            readFile(`${import.meta.dirname}/wasm/renderer_browser/renderer_browser_bg.wasm`),
          )
          .then((bytes) => init({ module_or_path: bytes }))
      : init({ module_or_path: countedFetch(wasmUrl) })
  ).then((output) => {
    memory = output.memory;
    return output;
  });
  return loaded;
}

export function presentationMemory(): WebAssembly.Memory | null {
  return memory;
}
