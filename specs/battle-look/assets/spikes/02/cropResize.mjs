// bun cropResize.mjs <in.(png|jpg)> <x> <y> <w> <h> <outW> <outH> <out.png>  (box filter)
import { createRequire } from "node:module";
import { readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
const { PNG } = createRequire("/Users/david/dev/battlegame/web/package.json")("pngjs");
const [inp, x, y, w, h, ow, oh, out] = process.argv.slice(2);
let path = inp;
if (!inp.endsWith(".png")) {
  path = out + ".src.png";
  execFileSync("sips", ["-s", "format", "png", inp, "--out", path], { stdio: "ignore" });
}
const src = PNG.sync.read(readFileSync(path));
const [X, Y, Wd, Hd, OW, OH] = [x, y, w, h, ow, oh].map(Number);
const dst = new PNG({ width: OW, height: OH });
for (let j = 0; j < OH; j++) for (let i = 0; i < OW; i++) {
  const x0 = X + (i * Wd) / OW, x1 = X + ((i + 1) * Wd) / OW, y0 = Y + (j * Hd) / OH, y1 = Y + ((j + 1) * Hd) / OH;
  const acc = [0, 0, 0]; let n = 0;
  for (let sy = Math.floor(y0); sy < Math.ceil(y1); sy++) for (let sx = Math.floor(x0); sx < Math.ceil(x1); sx++) {
    const k = (sy * src.width + sx) * 4; acc[0] += src.data[k]; acc[1] += src.data[k + 1]; acc[2] += src.data[k + 2]; n++;
  }
  const d = (j * OW + i) * 4;
  dst.data[d] = acc[0] / n; dst.data[d + 1] = acc[1] / n; dst.data[d + 2] = acc[2] / n; dst.data[d + 3] = 255;
}
writeFileSync(out, PNG.sync.write(dst));
