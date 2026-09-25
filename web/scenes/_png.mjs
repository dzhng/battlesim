import { writeFile } from "node:fs/promises";
import { PNG } from "pngjs";

export const decode = (buffer) => PNG.sync.read(buffer);

export function pixel(png, x, y) {
  const i = (Math.round(y) * png.width + Math.round(x)) * 4;
  return [png.data[i], png.data[i + 1], png.data[i + 2]];
}

/** Save a nearest-neighbour `scale`× crop centred on (cx, cy), clamped to the image. */
export async function writeCrop(png, path, cx, cy, halfW, halfH, scale = 3) {
  const x0 = Math.min(png.width - 1, Math.max(0, Math.round(cx - halfW))),
    y0 = Math.min(png.height - 1, Math.max(0, Math.round(cy - halfH)));
  const w = Math.max(1, Math.min(png.width - x0, Math.round(halfW * 2))),
    h = Math.max(1, Math.min(png.height - y0, Math.round(halfH * 2)));
  const out = new PNG({ width: w * scale, height: h * scale });
  for (let y = 0; y < h * scale; y++) {
    for (let x = 0; x < w * scale; x++) {
      const s = ((y0 + Math.floor(y / scale)) * png.width + x0 + Math.floor(x / scale)) * 4;
      png.data.copy(out.data, (y * w * scale + x) * 4, s, s + 4);
    }
  }
  await writeFile(path, PNG.sync.write(out));
}
