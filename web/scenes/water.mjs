// The water scene (specs/done/water-and-wetlands): the water reel
// (`fixtures/captures/water.json`) on the wetlands capture map, filmed by the
// graphics test with control plates and replayed by the native client from
// the same run: the river at three framings, then a squad standing in the
// swamp, from above and wading. Each shot's browser plate must show the
// simulation's water (river or swamp) apart from its dry bank, and native's
// must show it as the browser does. The
// plates go to the evidence
// in one sheet beside RedGPU's frozen stills
// (`specs/done/water-and-wetlands/assets/`), for the side-by-side read; the look
// is judged there by eye, not here. Run it under the GPU lock.
import { execFile } from "node:child_process";
import { readdir, readFile } from "node:fs/promises";
import { promisify } from "node:util";
import { nativeToolPath } from "../scene.mjs";
import { loadMap } from "../src/maps/node.ts";
import { filmParityRun } from "./_parityRun.mjs";
import { decode } from "./_png.mjs";
import { writeSheet } from "./_sheet.mjs";
import { deltaE } from "./_colour.mjs";
import { mapWater, waterAndBank, waterAndBankColours } from "./_water.mjs";

const ROOT = new URL("../../", import.meta.url);
const MAP = "wetlands";
/** The reel's shots, in order (the capture format names none). */
const SHOTS = ["close", "tactical", "low", "swamp", "wading", "puddles"];
/** The browser's water reads as water when its mean colour stands at least
 *  this far from its dry bank's (CIE76 ΔE): a loose floor. */
const APART_MIN = 10;
/** Native draws the water as the browser does when their mean colours over
 *  it lie within this (ΔE). Native replays the browser's frames, so the two
 *  agree to a hair; a native that drew no water would show its bed, which
 *  lies far past this from the water. */
const NATIVE_MAX = 3;
const exec = promisify(execFile);

export async function run(ctx) {
  // `WATER_LOOK=<row>` films one water look on every river (`?water_look=`),
  // `SWAMP_LOOK=<row>` one on every swamp (`?swamp_look=`).
  const pins = { water_look: process.env.WATER_LOOK, swamp_look: process.env.SWAMP_LOOK };
  const look = Object.entries(pins).filter(([, row]) => row !== undefined);
  const runDir = await filmParityRun(
    ctx,
    ctx.url + look.map(([k, row]) => `&${k}=${row}`).join(""),
  );
  if (!runDir) return;
  const dir = new URL(`${MAP}/`, runDir);
  const capture = JSON.parse(await readFile(new URL("capture.json", dir), "utf8"));
  const { lens } = JSON.parse(await readFile(new URL("manifest.json", dir), "utf8"));
  const frames = capture.frames.filter((f) => f.plate !== null);

  // Native replays the run: it must start, draw every plate, and count what
  // the browser counted (it exits with an error otherwise).
  const native = ctx.evidencePath("native/");
  const replay = await exec("cargo", ["build", "-q", "-p", "native-client"], { cwd: ROOT.pathname })
    .then(() =>
      exec(nativeToolPath("debug/native-client"), [
        "--run",
        runDir.pathname,
        "--scene",
        MAP,
        "--out",
        native,
      ]),
    )
    .then(
      ({ stdout }) => ({ ok: true, detail: stdout.trim().split("\n").at(-1) }),
      (error) => ({ ok: false, detail: (error.stderr || error.message).trim() }),
    );
  ctx.check("native starts and draws every plate of the water reel", replay.ok, replay.detail);
  const nativePlates = replay.ok ? await readdir(native) : [];

  const water = await mapWater(MAP);
  const { size } = loadMap(MAP).definition;
  const cells = [];
  for (const [k, frame] of frames.entries()) {
    const regions = waterAndBank(frame, lens.fovY, water, size);
    const browserBytes = await readFile(new URL(frame.plate, dir));
    cells.push({ caption: `browser · ${SHOTS[k]}`, png: browserBytes });
    // The puddle shot is of the presentation's own water (look only,
    // decision 29), none of the simulation's: both clients' plates go to the
    // sheet, judged by eye. Every other shot must find its water.
    if (SHOTS[k] === "puddles") {
      if (nativePlates.includes(frame.plate))
        cells.push({
          caption: `native · ${SHOTS[k]}`,
          png: await readFile(`${native}${frame.plate}`),
        });
      continue;
    }
    const browser = waterAndBankColours(decode(browserBytes), regions);
    const apart = deltaE(browser.water, browser.bank);
    ctx.check(
      `browser ${SHOTS[k]}: the water stands apart from its bank`,
      apart >= APART_MIN,
      `ΔE ${apart.toFixed(1)} over ${regions.water.length} water and ${regions.bank.length} bank pixels`,
    );
    if (!nativePlates.includes(frame.plate)) continue;
    const nativeBytes = await readFile(`${native}${frame.plate}`);
    cells.push({ caption: `native · ${SHOTS[k]}`, png: nativeBytes });
    // Native reads its own biome: a pinned look is the browser's alone.
    if (look.length) continue;
    const off = deltaE(waterAndBankColours(decode(nativeBytes), regions).water, browser.water);
    ctx.check(
      `native ${SHOTS[k]}: draws the water as the browser does`,
      off <= NATIVE_MAX,
      `ΔE ${off.toFixed(1)} from the browser's over the water`,
    );
  }
  const stills = new URL("specs/done/water-and-wetlands/assets/", ROOT);
  for (const file of (await readdir(stills)).filter((f) => f.startsWith("redgpu-")).sort())
    cells.push({ caption: `RedGPU · ${file}`, png: await readFile(new URL(file, stills)) });
  await writeSheet(ctx, "water.png", {
    title: "Water: ours beside RedGPU",
    columns: 2,
    cellWidth: 640,
    cells,
  });
}
