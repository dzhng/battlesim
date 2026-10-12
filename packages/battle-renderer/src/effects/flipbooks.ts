// The effect atlas's flipbooks: CC0 image sequences from Unity Labs' free VFX
// flipbooks (sources and licence in assets/README.md), converted from their
// TGA sheets to PNG. One layer each of a 2D array texture, in `LAYER` order
// (`crates/presentation/src/effects/frame.rs`).
import fireUrl from "../../../../assets/third-party/effects/explosion00_5x5.png?url";
import dustUrl from "../../../../assets/third-party/effects/cloud01_8x8.png?url";
/** The flipbook layers, and their frames, as the effects
 *  (`crates/presentation/src/effects/frame.rs`) name them in `Effect.b`. */
const LAYER = { fire: 0, dust: 1 } as const;
const LAYER_FRAMES = [25, 64] as const;

export interface Flipbook {
  url: string;
  /** Frames across and down the sheet, left to right, top to bottom. */
  columns: number;
  rows: number;
  frames: number;
}

/** The atlas's side in texels: every sheet is this square. */
export const FLIPBOOK_SIZE = 1024;

export const FLIPBOOKS: readonly Flipbook[] = [
  // Explosion00: a fireball that burns out into dark smoke.
  { url: fireUrl, columns: 5, rows: 5, frames: LAYER_FRAMES[LAYER.fire] },
  // Cloud01: a lit puff turning in place, for dust.
  { url: dustUrl, columns: 8, rows: 8, frames: LAYER_FRAMES[LAYER.dust] },
];
