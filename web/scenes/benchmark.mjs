// The scripted benchmark's short run on a production build, reached the way a
// player reaches it: main menu → Benchmark → Short run. Asserts the run
// completes with every camera phase, the simulation advanced in contact, and
// the camera flew its keyframes; saves the report, the frame-cost.md row and
// screenshots of the menu, the run and the results as evidence.
import { writeFile } from "node:fs/promises";

const PREPARE_TIMEOUT_MS = 600_000;
/** Placement is exact: the tour's keyframes sit inside the rig's limits. */
const CAMERA_TOLERANCE = 1e-6;

const shot = async (ctx, page, name) =>
  writeFile(ctx.evidencePath(name), await page.screenshot({ fullPage: false }));

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  const origin = new URL(ctx.url).origin;
  await page.goto(`${origin}/`);
  const menu = await page.getByRole("navigation", { name: "Main menu" }).getByRole("link");
  const labels = await menu.allTextContents();
  ctx.check(
    "the main menu offers play, replay, benchmark and labs",
    ["Play village", "Watch replay", "Benchmark", "Labs"].every((l) => labels.includes(l)),
    labels.join(", "),
  );
  await shot(ctx, page, "menu.png");
  await page.getByRole("link", { name: "Benchmark" }).click();
  await page.waitForURL("**/benchmark");
  await page.getByRole("button", { name: /Short run/ }).waitFor();
  await shot(ctx, page, "start.png");

  const clicked = Date.now();
  await page.getByRole("button", { name: /Short run/ }).click();
  await page.waitForFunction(() => window.__lab?.ready || window.__lab?.error, undefined, {
    timeout: 60_000,
  });
  const error = await page.evaluate(() => window.__lab.error);
  if (error) throw new Error(`viewport failed: ${error}`);
  // Warm-up: the real simulation steps to the start tick.
  await page.waitForFunction(
    () =>
      /\d+ \/ \d+ s ·/.test(
        document.querySelector("[data-testid=benchmark-progress]")?.textContent ?? "",
      ),
    undefined,
    { timeout: PREPARE_TIMEOUT_MS, polling: 500 },
  );
  const preparedS = (Date.now() - clicked) / 1000;

  // During the timed run: rounds in flight (heavy contact), and a frame per phase.
  let rounds = 0;
  const seen = new Set();
  while (!(await page.evaluate(() => window.__benchmark?.stage === "results"))) {
    const now = await page.evaluate(() => ({
      text: document.querySelector("[data-testid=benchmark-progress] span")?.textContent ?? "",
      rounds: window.__lab?.route?.observation()?.projectiles?.length ?? 0,
    }));
    rounds = Math.max(rounds, now.rounds);
    const phase = /· (\w+)/.exec(now.text)?.[1];
    if (phase && !seen.has(phase)) {
      seen.add(phase);
      await page.waitForTimeout(1500); // into the phase, off its first keyframe
      await shot(ctx, page, `run-${phase}.png`);
    }
    await page.waitForTimeout(500);
  }

  await page.getByTestId("benchmark-results").waitFor();
  await page.waitForTimeout(300);
  await shot(ctx, page, "results.png");
  // The results scroll inside their own panel: capture its end too.
  await page.getByTestId("benchmark-results").evaluate((el) => el.scrollTo(0, el.scrollHeight));
  await shot(ctx, page, "results-end.png");
  const { report, row } = await page.evaluate(() => ({
    report: window.__benchmark.report,
    row: window.__benchmark.frameCostRow("10"),
  }));

  ctx.check(
    "the run completed its timed window",
    report.outcome.status === "complete",
    report.outcome.reason,
  );
  const phases = report.tour.phases.map((p) => p.name);
  const empty = report.phases.filter((p) => !p.frameMs?.count).map((p) => p.name);
  ctx.check(
    "the report has frames in every camera phase",
    phases.length === 6 && empty.length === 0,
    `${report.phases.map((p) => `${p.name} ${p.frameMs?.count ?? 0}`).join(", ")}`,
  );
  const sim = report.simulation;
  const runS = report.recordedMs / 1000;
  ctx.check(
    "the simulation advanced in real time from the contact tick",
    sim.startTick === report.scenario.startTick && sim.simulatedSeconds > 0.8 * runS,
    `ticks ${sim.startTick} → ${sim.endTick}: ${sim.simulatedSeconds.toFixed(1)} s simulated in ${runS.toFixed(1)} s`,
  );
  ctx.check(
    "rounds were in flight during the run (heavy contact)",
    rounds > 0,
    `up to ${rounds} visible rounds`,
  );

  // The camera drawn is the tour's framing, and the tour passes its keyframes.
  const frames = report.samples.frames;
  let drift = 0;
  for (const f of frames) {
    const a = f.camera;
    const b = f.intended;
    drift = Math.max(
      drift,
      Math.abs(a.target[0] - b.target[0]),
      Math.abs(a.target[1] - b.target[1]),
      Math.abs(a.distance - b.distance),
      Math.abs(a.yaw - b.yaw),
      Math.abs(a.pitch - b.pitch),
    );
  }
  let keyMiss = 0;
  for (const [t, x, y, distance, yaw, pitch] of report.tour.keyframes) {
    const at = t * report.durationMs;
    const near = frames.reduce((best, f) =>
      Math.abs(f.elapsedMs - at) < Math.abs(best.elapsedMs - at) ? f : best,
    );
    if (Math.abs(near.elapsedMs - at) > 100) continue; // the run's edge
    const c = near.camera;
    keyMiss = Math.max(
      keyMiss,
      Math.hypot(c.target[0] - x, c.target[1] - y) / distance,
      Math.abs(c.distance - distance) / distance,
      Math.abs(c.yaw - yaw),
      Math.abs(c.pitch - pitch),
    );
  }
  ctx.check(
    "the camera flew the tour: drawn = intended, and through every keyframe",
    drift < CAMERA_TOLERANCE && keyMiss < 0.05,
    `max drift ${drift.toExponential(1)}, worst keyframe miss ${keyMiss.toFixed(3)}`,
  );
  // Slice 12's frame-cost assertion, carried over from the deleted probe: the
  // battle frame reports its GPU time and texture bytes, here in every phase.
  const timestampQuery = await page.evaluate(
    async () => (await navigator.gpu.requestAdapter())?.features.has("timestamp-query") ?? false,
  );
  if (timestampQuery)
    ctx.check(
      "the battle frame reports its GPU time and texture bytes in every phase",
      report.phases.every((p) => p.gpu?.meanMs > 0) && report.memory.last?.textureBytes > 0,
      `${report.phases.map((p) => `${p.name} ${p.gpu?.meanMs.toFixed(2) ?? "—"} ms`).join(", ")}; textures ${report.memory.last?.textureBytes} B`,
    );

  const f = report.frameMs;
  console.log(
    `METRIC benchmark short: prepared in ${preparedS.toFixed(0)} s; ${frames.length} frames, ${report.averageFps.toFixed(1)} FPS, p50 ${f.p50.toFixed(2)} ms, p95 ${f.p95.toFixed(2)}, p99 ${f.p99.toFixed(2)}; GPU frame ${report.gpu?.meanMs.toFixed(2) ?? "—"} ms mean, worst window p95 ${report.gpu?.p95Ms.toFixed(2) ?? "—"}; CPU p95 ${report.cpuMs.p95.toFixed(2)} ms; step p95 ${report.ticks.stepMs?.p95.toFixed(2)} ms`,
  );
  for (const p of report.phases)
    console.log(
      `METRIC benchmark ${p.name}: p50 ${p.frameMs?.p50.toFixed(2)} p95 ${p.frameMs?.p95.toFixed(2)} p99 ${p.frameMs?.p99.toFixed(2)} GPU ${p.gpu?.meanMs.toFixed(2) ?? "—"}`,
    );
  console.log(`METRIC benchmark frame-cost row: ${row}`);
  await ctx.writeEvidence("report.json", report);
  await writeFile(ctx.evidencePath("frame-cost-row.md"), `${row}\n`);
}
