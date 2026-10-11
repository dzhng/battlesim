/** The page's presentation module (`crates/presentation-wasm`): one side's
 * publications in, the game UI's observation out. Rust decodes; this hands
 * the result over as the page's view objects. */
import init, { Presentation } from "@wasm/presentation/presentation_wasm.js";
import wasmUrl from "@wasm/presentation/presentation_wasm_bg.wasm?url";
import { countedFetch } from "../../downloads";
import type { ObservationLayout, ObservationView } from "./observationView";

let loaded: Promise<unknown> | null = null;

/** Initialises the module once per realm; decoders need it first. */
export function loadPresentation(): Promise<unknown> {
  // Vitest runs in Node, where Vite's asset URL cannot be fetched (module.ts).
  loaded ??=
    typeof process !== "undefined" && process.versions?.node
      ? import("node:fs/promises")
          .then(({ readFile }) =>
            readFile(`${import.meta.dirname}/../../wasm/presentation/presentation_wasm_bg.wasm`),
          )
          .then((bytes) => init({ module_or_path: bytes }))
      : init({ module_or_path: countedFetch(wasmUrl) });
  return loaded;
}

/** One ordered side-publication stream. Retained observations are immutable;
 * unchanged fog and static groups share their views. */
export class ObservationDecoder {
  private readonly rust: Presentation;
  private last: ObservationView | null = null;

  constructor(layout: string | ObservationLayout) {
    this.rust = new Presentation(typeof layout === "string" ? layout : JSON.stringify(layout));
  }

  /** The next record's observation, or null for a stale one (in flight from
   * before `invalidate`). A malformed record throws and changes nothing. */
  decode(data: Float32Array | Uint32Array): ObservationView | null {
    const words = new Uint32Array(data.buffer, data.byteOffset, data.length);
    if (!this.rust.push_publication(words, 0)) return null;
    const view = JSON.parse(this.rust.observation()) as ObservationView;
    const last = this.last;
    view.corpses ??= last!.corpses;
    view.knownProps ??= last!.knownProps;
    view.fallenBodies ??= last!.fallenBodies;
    view.fog.bits = this.rust.fog_bits() ?? last!.fog.bits;
    view.groundPatch.runs = this.rust.ground_runs();
    return (this.last = view);
  }

  /** Records still in flight are read as stale; the next must open a new
   * epoch (for `side`, when named). */
  invalidate(side?: string) {
    this.rust.invalidate(side);
    this.last = null;
  }
}
