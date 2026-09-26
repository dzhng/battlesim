// Crop + nearest-neighbour upscale evidence PNGs for the critique.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { createRequire } from "node:module";
const require = createRequire("/Users/david/dev/battlegame/web/package.json");
const { PNG } = require("pngjs");
const EV = new URL("../evidence/spike-foundation/", import.meta.url).pathname;
const OUT = new URL("./critique/", import.meta.url).pathname;
mkdirSync(OUT, { recursive: true });
const crops = [
  ["spike-village.png", 560, 280, 640, 360, 2, "village-building-shadows"],
  ["spike-village.png", 980, 280, 440, 260, 3, "village-building-top"],
  ["spike-main-default.png", 1040, 560, 220, 240, 4, "soldier-shadows"],
  ["spike-main-default.png", 1720, 60, 200, 110, 4, "tank-shadow"],
  ["swim-street.png", 880, 420, 700, 260, 2, "street-shadow-edge"],
  ["spike-ground.png", 0, 200, 700, 200, 2, "forest-trunks"],
  ["spike-closest.png", 900, 190, 600, 170, 3, "closest-building-base"],
  ["spike-warno.png", 1380, 190, 540, 560, 2, "warno-forest"],
];
for (const [file, x, y, w, h, s, name] of crops) {
  const src = PNG.sync.read(readFileSync(EV + file));
  const out = new PNG({ width: w * s, height: h * s });
  for (let j = 0; j < h * s; j++)
    for (let i = 0; i < w * s; i++) {
      const si = ((y + Math.floor(j / s)) * src.width + x + Math.floor(i / s)) * 4;
      const di = (j * w * s + i) * 4;
      for (let k = 0; k < 4; k++) out.data[di + k] = src.data[si + k];
    }
  writeFileSync(`${OUT}${name}.png`, PNG.sync.write(out));
}
console.log("ok");
