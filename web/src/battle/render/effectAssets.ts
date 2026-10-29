/** Authored source transport; Rust owns decoding, mip chains and GPU atlases. */
import sheets from "../../../../assets/effect-sheets.json";

const urls = import.meta.glob<string>("../../../../assets/third-party/effects/*.png", {
  query: "?url",
  import: "default",
  eager: true,
});

export async function loadEffectAtlasLayers(): Promise<Uint8Array[]> {
  return Promise.all(
    sheets.map(async (file) => {
      const url = urls[`../../../../assets/third-party/effects/${file}`];
      const response = await fetch(url);
      if (!response.ok) throw new Error(`effect flipbook ${file}: HTTP ${response.status}`);
      return new Uint8Array(await response.arrayBuffer());
    }),
  );
}
