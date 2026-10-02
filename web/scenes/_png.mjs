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

/** The PNG of the `w` by `h` pixels of `png` from (x0, y0), clamped to the image. */
export function crop(png, x0, y0, w, h) {
  const left = Math.min(png.width - 1, Math.max(0, Math.round(x0)));
  const top = Math.min(png.height - 1, Math.max(0, Math.round(y0)));
  const out = new PNG({
    width: Math.max(1, Math.min(png.width - left, Math.round(w))),
    height: Math.max(1, Math.min(png.height - top, Math.round(h))),
  });
  PNG.bitblt(png, out, left, top, out.width, out.height, 0, 0);
  return PNG.sync.write(out);
}

/** The in-image pixel coordinates within `r` px (a square) of `p`. */
export function* around(png, p, r) {
  const [cx, cy] = [Math.round(p[0]), Math.round(p[1])];
  for (let y = Math.max(0, cy - r); y <= Math.min(png.height - 1, cy + r); y++)
    for (let x = Math.max(0, cx - r); x <= Math.min(png.width - 1, cx + r); x++) yield [x, y];
}

/** Whether any pixel within `r` px of `p` passes `test([r, g, b])`. */
export function anyNear(png, p, r, test) {
  for (const [x, y] of around(png, p, r)) if (test(pixel(png, x, y))) return true;
  return false;
}

/** The largest summed |ΔRGB| between two same-size shots within `r` px of `p`. */
export function mostChanged(a, b, p, r) {
  let most = 0;
  for (const [x, y] of around(a, p, r)) {
    const [pa, pb] = [pixel(a, x, y), pixel(b, x, y)];
    most = Math.max(
      most,
      pa.reduce((s, v, i) => s + Math.abs(v - pb[i]), 0),
    );
  }
  return most;
}
