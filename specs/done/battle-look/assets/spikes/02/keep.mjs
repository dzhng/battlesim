// Copy the evidence worth keeping to the main checkout's throwaway (survives worktree removal).
import { mkdirSync, copyFileSync, readdirSync } from "node:fs";
const SRC = new URL("../evidence/spike02/", import.meta.url).pathname;
const DST = "/Users/david/dev/battlegame/throwaway/evidence/spike02/";
mkdirSync(DST + "compare", { recursive: true });
mkdirSync(DST + "critique", { recursive: true });
const keep = [
  "look-street-oblique-A.png", "look-street-oblique-B.png", "look-street-corner-A.png", "look-street-corner-B.png",
  "look-street-ground-A.png", "look-street-ground-B.png", "look-village-start-A.png", "look-village-start-B.png",
  "look-endurance-A.png", "look-endurance-B.png", "look-street-all-A.png", "look-street-all-B.png",
  "look-street-oblique-A-mask.png", "look-street-oblique-B-mask.png",
  "check-far-4096x64x512.png", "check-near-4096x64x512.png", "pos-far-B.png", "pos-near-B.png",
  "diff-street.png", "diff-village.png", "diff-endurance.png", "probe-A-split1.png",
];
for (const f of keep) copyFileSync(SRC + f, DST + f);
for (const d of ["outA", "outB"]) {
  mkdirSync(`${DST}compare/${d}`, { recursive: true });
  for (const f of readdirSync(`${SRC}compare/${d}`)) copyFileSync(`${SRC}compare/${d}/${f}`, `${DST}compare/${d}/${f}`);
}
for (const f of readdirSync(SRC + "critique")) copyFileSync(SRC + "critique/" + f, DST + "critique/" + f);
console.log("kept", keep.length);
