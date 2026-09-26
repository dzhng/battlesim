// Copy the look frames under neutral names and cut zoomed crops for the critique.
import { mkdirSync, copyFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
const E = new URL("../evidence/spike02/", import.meta.url).pathname;
const OUT = E + "critique/";
mkdirSync(OUT, { recursive: true });
const frames = ["village-start", "street-oblique", "street-ground", "street-corner", "street-all", "endurance"];
const crops = [
  ["street-corner", "crop-corner", 1000, 280, 500, 330, 3],
  ["street-ground", "crop-ground-edge", 700, 380, 450, 180, 4],
  ["street-oblique", "crop-oblique-edge", 1100, 150, 350, 300, 3],
  ["endurance", "crop-endurance", 900, 420, 600, 400, 2],
  ["village-start", "crop-village", 880, 290, 480, 620, 2],
];
for (const [t, n] of [["A", 1], ["B", 2]]) {
  for (const f of frames) copyFileSync(`${E}look-${f}-${t}.png`, `${OUT}set${n}-${f}.png`);
  for (const [f, name, x, y, w, h, s] of crops) {
    execFileSync("bun", [new URL("crop.mjs", import.meta.url).pathname, `${E}look-${f}-${t}.png`, x, y, w, h, s, `${OUT}set${n}-${name}.png`].map(String));
  }
}
console.log("ok");
