// Slice 16: the stress battle in real time. Runs ENDURANCE_S seconds (60 by
// default), reset cycles, then the late state for ENDURANCE_LATE_S seconds
// (60 by default); the verdict's run sets both to 300. Budgets (validation.md) are measured and written as evidence, not
// asserted: the scene fails on broken contracts, not on slow hardware.
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";

const SECONDS = Number(process.env.ENDURANCE_S ?? 60);
const LATE_SECONDS = Number(process.env.ENDURANCE_LATE_S ?? 60);
const lab = (page, fn, arg) => page.evaluate(fn, arg);
const telemetry = (page) => lab(page, () => window.__lab.route.telemetry());

async function shot(ctx, page, name, crop) {
  await page.evaluate(() => window.__lab.frame());
  const png = await page.screenshot();
  await writeFile(ctx.evidencePath(`frame-${name}.png`), png);
  if (crop) {
    const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), crop);
    await writeCrop(decode(png), ctx.evidencePath(`crop-${name}-3x.png`), at[0], at[1], 110, 70, 3);
  }
}

/** Run in real time for `seconds`, sampling telemetry every 10 s. */
async function soak(page, seconds) {
  await lab(page, () => window.__lab.route.resetFrames());
  const startTick = (await telemetry(page)).tick;
  const started = Date.now();
  const samples = [];
  let failed = false;
  while (Date.now() - started < seconds * 1000) {
    await page.waitForTimeout(Math.min(10_000, seconds * 1000 - (Date.now() - started)));
    const t = await telemetry(page);
    samples.push({ wallS: (Date.now() - started) / 1000, ...t });
    failed ||= t.status === "failed";
    await lab(page, () => window.__lab.route.resetFrames());
  }
  const end = await telemetry(page);
  const wall = (Date.now() - started) / 1000;
  return {
    samples,
    failed,
    ticks: end.tick - startTick,
    wall,
    rate: (end.tick - startTick) / wall,
  };
}

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 30, undefined, { timeout: 60000 });
  await shot(ctx, page, "early-1280x800", [1500, 1000, 0]);
  const live = await soak(page, SECONDS);
  await shot(ctx, page, "late-run-1280x800", [1500, 1000, 0]);
  ctx.check(
    `the stress battle runs ${SECONDS} s in real time without failing`,
    !live.failed && live.ticks > 0,
    `${live.ticks} ticks in ${live.wall.toFixed(0)} s = ${live.rate.toFixed(1)} Hz`,
  );

  // Reset cycles keep the GPU allocations where they were.
  const before = (await telemetry(page)).gpu;
  for (let i = 0; i < 3; i++) {
    const was = await lab(page, () => window.__lab.route.tick());
    await page.getByRole("button", { name: "Reset" }).click();
    // A fresh session starts from tick 0, then runs.
    await page.waitForFunction((t) => window.__lab.route.tick() < t, was, { timeout: 60000 });
    await page.waitForFunction(() => window.__lab.route.tick() > 10, undefined, { timeout: 60000 });
  }
  const after = (await telemetry(page)).gpu;
  ctx.check(
    "reset cycles return live GPU buffers and textures to the same count",
    after.buffers === before.buffers && after.textures === before.textures,
    `before ${JSON.stringify(before)} after ${JSON.stringify(after)}`,
  );

  // The late state: 20,000 fallen and 2,000 wrecks in the field.
  await page.getByLabel(/Late state/).check();
  await page.waitForFunction(
    () => window.__lab?.route?.late?.() && window.__lab.route.tick() > 30,
    undefined,
    {
      timeout: 120000,
    },
  );
  await shot(ctx, page, "late-state-1280x800", [1500, 1000, 0]);
  const late = await soak(page, LATE_SECONDS);
  await shot(ctx, page, "late-state-after-1280x800", [1500, 1000, 0]);
  ctx.check(
    "the late state runs without failing",
    !late.failed && late.ticks > 0,
    `${late.ticks} ticks in ${late.wall.toFixed(0)} s = ${late.rate.toFixed(1)} Hz`,
  );
  // The whole page's memory, the simulation worker included (the heap in
  // the telemetry is the main thread's only).
  const pageMemory = await Promise.race([
    lab(page, () => performance.measureUserAgentSpecificMemory?.().then((m) => m.bytes) ?? null),
    new Promise((resolve) => setTimeout(() => resolve(null), 60_000)),
  ]);
  await ctx.writeEvidence("telemetry.json", {
    pageMemoryBytes: pageMemory,
    browser: ctx.browser,
    adapter: await lab(page, () => window.__lab.adapter),
    seconds: SECONDS,
    live,
    late,
  });
  console.log(
    `METRIC endurance page memory with the worker (late state): ${pageMemory === null ? "n/a" : `${(pageMemory / 2 ** 20).toFixed(0)} MiB`}`,
  );
  for (const [name, run] of [
    ["live", live],
    ["late", late],
  ]) {
    const worst = run.samples.reduce(
      (w, s) => ({ p95: Math.max(w.p95, s.frames.p95), p99: Math.max(w.p99, s.frames.p99) }),
      { p95: 0, p99: 0 },
    );
    const last = run.samples.at(-1);
    console.log(
      `METRIC endurance ${name}: sim ${run.rate.toFixed(1)} Hz; worst 10 s window frame p95 ${worst.p95.toFixed(1)} ms, p99 ${worst.p99.toFixed(1)} ms; main-thread heap ${last?.heapMiB?.toFixed(0)} MiB; GPU ${(last?.gpu.bufferBytes / 2 ** 20).toFixed(1)} MiB in ${last?.gpu.buffers} buffers`,
    );
  }
}
