// The water scene (specs/water-and-wetlands): the water reel
// (`fixtures/captures/water.json`) on the wetlands capture map, filmed by the
// graphics test with control plates and replayed by the native client from
// the same run. Each shot's plate, in both clients, must show the
// simulation's river bluer than its dry bank. The plates go to the evidence
// in one sheet beside RedGPU's frozen stills
// (`specs/water-and-wetlands/assets/`), for the side-by-side read; the look
// is judged there by eye, not here. Run it under the GPU lock.
import { execFile } from "node:child_process";
import { readdir, readFile } from "node:fs/promises";
import { promisify } from "node:util";
import { nativeToolPath } from "../scene.mjs";
import { loadMap } from "../src/maps/node.ts";
import { filmParityRun } from "./_parityRun.mjs";
import { decode } from "./_png.mjs";
import { writeSheet } from "./_sheet.mjs";
import { mapWater, waterAgainstBank, waterAndBank } from "./_water.mjs";

const ROOT = new URL("../../", import.meta.url);
const MAP = "wetlands";
/** The reel's shots, in order (the capture format names none). */
const SHOTS = ["close", "tactical", "low"];
/** The river reads as water when it is at least this much bluer than its
 *  bank (CIELAB b*): a loose floor that a missing water surface, showing
 *  the bed, falls under. */
const BLUER_MIN = 12;
const exec = promisify(execFile);

export async function run(ctx) {
  const runDir = await filmParityRun(ctx, ctx.url);
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
    const clients = [["browser", new URL(frame.plate, dir).pathname]];
    if (nativePlates.includes(frame.plate)) clients.push(["native", `${native}${frame.plate}`]);
    for (const [client, file] of clients) {
      const bytes = await readFile(file);
      const read = waterAgainstBank(decode(bytes), regions);
      ctx.check(
        `${client} ${SHOTS[k]}: the river reads as water, bluer than its bank`,
        read.bluer >= BLUER_MIN,
        `b* ${read.bluer.toFixed(1)} lower over ${read.water.count} water and ${read.bank.count} bank pixels`,
      );
      cells.push({ caption: `${client} · ${SHOTS[k]}`, png: bytes });
    }
  }
  const stills = new URL("specs/water-and-wetlands/assets/", ROOT);
  for (const file of (await readdir(stills)).filter((f) => f.startsWith("redgpu-")).sort())
    cells.push({ caption: `RedGPU · ${file}`, png: await readFile(new URL(file, stills)) });
  await writeSheet(ctx, "water.png", {
    title: "Water: ours beside RedGPU",
    columns: 2,
    cellWidth: 640,
    cells,
  });
}
