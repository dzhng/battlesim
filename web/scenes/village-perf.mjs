// Frame cost of the village on a production build: the live battle measured
// from the strategic, default and ground cameras. Writes the frame-cost row
// evidence; the benchmark (battle-look slice 10) replaces this scene.
import { lab } from "./_lab.mjs";
import { measure, timestampQuery } from "./_frameCost.mjs";

const SECONDS = Number(process.env.FRAME_COST_S ?? 10);
/** Wheel over the battlefield until the zoom limit (CameraController). */
const wheelTo = async (page, dy) => {
  await page.mouse.move(1300, 540);
  for (let k = 0; k < 20; k++) await page.mouse.wheel(0, dy);
};
/** Each camera from the opening framing: out to the strategic height, as
 *  opened, and in to the ground at the village. */
const CAMERAS = {
  strategic: (page) => wheelTo(page, 400),
  default: async () => {},
  ground: async (page) => {
    await lab(page, () =>
      window.__lab.setCamera({ ...window.__lab.camera(), target: [990, 790, 0], yaw: -1.2 }),
    );
    await wheelTo(page, -400);
  },
};

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 30, undefined, { timeout: 60000 });
  const opening = await lab(page, () => window.__lab.camera());
  await page.waitForTimeout(5000); // let the battle warm up
  const rows = {};
  for (const [name, frame] of Object.entries(CAMERAS)) {
    await lab(page, (c) => window.__lab.setCamera(c), opening);
    await frame(page);
    rows[name] = await measure(page, SECONDS);
    const r = rows[name];
    console.log(
      `METRIC village-perf ${name}: p50 ${r.p50.toFixed(1)} ms, p95 ${r.p95.toFixed(1)}, p99 ${r.p99.toFixed(1)}, sim ${r.simHz.toFixed(1)} Hz, publication ${r.publicationBytes} B, buffers ${(r.bufferBytes / 2 ** 20).toFixed(1)} MiB in ${r.buffers} buffers, ${r.textures} textures`,
    );
    ctx.check(`${name} camera renders frames`, r.frames > SECONDS * 20, `${r.frames} frames`);
  }
  const ts = await timestampQuery(page);
  console.log(`METRIC village-perf timestamp-query available: ${ts}`);
  await ctx.writeEvidence("frame-cost.json", {
    browser: ctx.browser,
    seconds: SECONDS,
    timestampQuery: ts,
    rows,
  });
}
