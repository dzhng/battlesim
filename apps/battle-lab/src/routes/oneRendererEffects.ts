import { FLIPBOOKS, FLIPBOOK_SIZE } from "@packages/battle-renderer/src/effects/flipbooks";

export async function loadEffectAtlasLayers(): Promise<Uint8Array[]> {
  return Promise.all(
    FLIPBOOKS.map(async (book) => {
      const response = await fetch(book.url);
      if (!response.ok) throw new Error(`effect flipbook ${book.url}: HTTP ${response.status}`);
      const bitmap = await createImageBitmap(await response.blob(), {
        premultiplyAlpha: "premultiply",
        colorSpaceConversion: "none",
      });
      if (bitmap.width !== FLIPBOOK_SIZE || bitmap.height !== FLIPBOOK_SIZE) {
        bitmap.close();
        throw new Error(`effect flipbook ${book.url} is not ${FLIPBOOK_SIZE}²`);
      }
      const canvas = document.createElement("canvas");
      canvas.width = FLIPBOOK_SIZE;
      canvas.height = FLIPBOOK_SIZE;
      const context = canvas.getContext("2d", { willReadFrequently: true });
      if (!context) {
        bitmap.close();
        throw new Error("effect flipbook canvas context unavailable");
      }
      context.drawImage(bitmap, 0, 0);
      bitmap.close();
      const rgba = new Uint8Array(context.getImageData(0, 0, FLIPBOOK_SIZE, FLIPBOOK_SIZE).data);
      for (let i = 0; i < rgba.length; i += 4) {
        const alpha = rgba[i + 3];
        rgba[i] = Math.round((rgba[i] * alpha) / 255);
        rgba[i + 1] = Math.round((rgba[i + 1] * alpha) / 255);
        rgba[i + 2] = Math.round((rgba[i + 2] * alpha) / 255);
      }
      return rgba;
    }),
  );
}
