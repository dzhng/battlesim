// bun crop.mjs <in.png> <x> <y> <w> <h> <scale> <out.png>  (nearest-neighbour zoom)
import { createRequire } from "node:module";
import { readFileSync, writeFileSync } from "node:fs";
const { PNG } = createRequire("/Users/david/dev/battlegame/web/package.json")("pngjs");
const [inp, x, y, w, h, s, out] = process.argv.slice(2);
const src = PNG.sync.read(readFileSync(inp));
const [X, Y, Wd, Hd, S] = [x, y, w, h, s].map(Number);
const dst = new PNG({ width: Wd * S, height: Hd * S });
for (let j = 0; j < Hd * S; j++) for (let i = 0; i < Wd * S; i++) {
  const sx = Math.min(src.width - 1, X + Math.floor(i / S)), sy = Math.min(src.height - 1, Y + Math.floor(j / S));
  const si = (sy * src.width + sx) * 4, di = (j * Wd * S + i) * 4;
  for (let c = 0; c < 4; c++) dst.data[di + c] = src.data[si + c];
}
writeFileSync(out, PNG.sync.write(dst));
