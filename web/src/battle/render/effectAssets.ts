/** Authored source transport; Rust owns decoding, mip chains and GPU atlases. */
import { FLIPBOOKS } from "@packages/battle-renderer/src/effects/flipbooks";

export async function loadEffectAtlasLayers(): Promise<Uint8Array[]> {
  return Promise.all(
    FLIPBOOKS.map(async (book) => {
      const response = await fetch(book.url);
      if (!response.ok) throw new Error(`effect flipbook ${book.url}: HTTP ${response.status}`);
      return new Uint8Array(await response.arrayBuffer());
    }),
  );
}
