// The browser's half of the frame-rate comparison with the native client
// (native's `--bench`): the graphics test's reel (`/graphics-test?scenes=`)
// without plates, on a production build at 1280×720, headless and paced to
// the display. For each scene it writes, after the first WARMUP_FRAMES drawn
// frames, the graphics test's own recording raw (each frame's
// requestAnimationFrame interval and main-thread time), the frame's GPU time
// from `timestamp-query`, and the in-game frame-rate counter's readings,
// to throwaway/perf/<PERF_RUN>/browser-<time>.json; `fps-compare.mjs`
// summarises it beside native's. FPS_SCENES=<map,…> picks the scenes in place
// of the route's two reel maps. Run it under the GPU lock; it waits on the
// route's stage, never on a timer.
import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { frameTimeSummary } from "../src/battle/benchmark/recording.ts";

const ROOT = new URL("../../", import.meta.url);
const VIEWPORT = { width: 1280, height: 720 };
/** The scenes, in place of the route's (`?scenes=`, both reel maps). */
const SCENES = process.env.FPS_SCENES;
const QUALITY = process.env.GRAPHICS_QUALITY;
const RUN = process.env.PERF_RUN ?? new Date().toISOString().slice(0, 10);
/** Drawn frames per scene left out of the timing; native's `--bench` takes
 *  the same count. */
const WARMUP_FRAMES = 60;
/** How far the in-game counter's mean reading may sit from the recording's
 *  average over the same scene: both count the same drawn frames. */
const COUNTER_AGREEMENT = 0.03;

/** In the page, from its start: every reading the in-game frame-rate
 *  counter shows, with the graphics test's scene at that moment. */
function watchCounter() {
  window.__counterReadings = [];
  new MutationObserver(() => {
    const label = document.querySelector(".frame-rate");
    const fps = Number.parseFloat(label?.textContent ?? "");
    if (!Number.isFinite(fps)) return;
    const scene = document.querySelector(".graphics-test-progress span")?.textContent ?? "";
    window.__counterReadings.push({ at: performance.now(), fps, scene });
  }).observe(document, { subtree: true, childList: true, characterData: true });
}

/** What else is drawing: a native run, or a browser besides this scene's
 *  own (the runner's Chromium can outlive its locked command while it tears
 *  down). Empty when the GPU is this run's alone. */
function contention() {
  const running = (...args) => {
    try {
      return execFileSync("pgrep", args, { encoding: "utf8" }).trim().split("\n");
    } catch {
      return []; // pgrep exits 1 when nothing matches
    }
  };
  const found = [];
  // By process name: a run still queued on the GPU lock draws nothing.
  if (running("-ix", "godot").length) found.push("a native run");
  const browsers = running(
    "-f",
    "^/[^ ]*Chrome for Testing\\.app/Contents/MacOS/Google Chrome for Testing( |$)",
  );
  if (browsers.length > 1) found.push(`${browsers.length} browsers`);
  return found;
}

/** macOS's power source; "unknown" where `pmset` is absent (another OS). */
function powerSource() {
  try {
    const batt = execFileSync("pmset", ["-g", "batt"], { encoding: "utf8" });
    return /drawing from '([^']+)'/.exec(batt)?.[1] ?? "unknown";
  } catch {
    return "unknown";
  }
}

export async function run(ctx) {
  const started = performance.now();
  const page = await ctx.newPage({ viewport: VIEWPORT });
  await page.addInitScript(watchCounter);
  const url = new URL(ctx.url);
  if (SCENES) url.searchParams.set("scenes", SCENES);
  if (QUALITY) url.searchParams.set("quality", QUALITY);
  await page.goto(url.href);
  await page.waitForFunction(() => window.__graphicsTest?.stage, undefined, { timeout: 60_000 });
  const refusal = await page.evaluate(() =>
    window.__graphicsTest.stage === "refused"
      ? document.querySelector("[role=alert]")?.textContent
      : null,
  );
  if (refusal) throw new Error(`the graphics test refused the run: ${refusal}`);
  const power = powerSource();
  const before = contention();
  await page.getByRole("button", { name: "Run graphics test" }).click();

  let shown = "";
  for (;;) {
    const state = await page.evaluate(() => ({
      stage: window.__graphicsTest?.stage,
      progress: document.querySelector(".graphics-test-progress span")?.textContent ?? "",
    }));
    if (state.stage === "results") break;
    if (state.progress && state.progress !== shown) {
      shown = state.progress;
      console.log(`  ${shown} (${((performance.now() - started) / 1000).toFixed(0)} s)`);
    }
    await page.waitForTimeout(1000);
  }
  const report = await page.evaluate(() => {
    const r = window.__graphicsTest.report;
    return {
      outcome: r.outcome,
      identity: r.identity,
      timingComparable: r.capture.frameTimingComparable,
      scenes: r.scenes.map((s) => ({ map: s.scene.map, gpu: s.gpu })),
      frames: r.samples.frames.map((f) => [f.phase, f.intervalMs, f.cpuMs]),
      counter: window.__counterReadings,
    };
  });
  ctx.check(
    "the reel played every scene",
    report.outcome.status === "complete",
    report.outcome.reason,
  );
  ctx.check("the run's frame timing is comparable (no plates)", report.timingComparable);
  const after = contention();
  ctx.check(
    "nothing else drew on the GPU at the timing's start or end",
    before.length === 0 && after.length === 0,
    `start: ${before.join(", ") || "none"}; end: ${after.join(", ") || "none"}`,
  );
  if (report.outcome.status !== "complete") return;

  const scenes = report.scenes.map(({ map, gpu }) => {
    const frames = report.frames.filter(([phase]) => phase === map).slice(WARMUP_FRAMES);
    // The counter reads in whole seconds from the scene's first frame; its
    // first reading spans the warm-up, so it is left out as the frames are.
    const counter = report.counter.filter((r) => r.scene.endsWith(map)).slice(1);
    return {
      scene: map,
      warmupFrames: WARMUP_FRAMES,
      intervalsMs: frames.map(([, ms]) => ms),
      cpuMs: frames.map(([, , cpu]) => cpu),
      // The battle frame's rolling window (its last 240 measured frames),
      // read every 2 s: the mean of those windows' means, and the worst p95.
      gpu,
      counterFps: counter.map((r) => r.fps),
    };
  });
  for (const s of scenes) {
    const recorded = frameTimeSummary(s.intervalsMs);
    const counted = s.counterFps.reduce((sum, v) => sum + v, 0) / s.counterFps.length;
    ctx.check(
      `${s.scene}: the in-game counter's readings agree with the recording's average`,
      Math.abs(counted / recorded.averageFps - 1) <= COUNTER_AGREEMENT,
      `counter mean ${counted.toFixed(1)} over ${s.counterFps.length} readings; recording ${recorded.averageFps.toFixed(1)} FPS over ${recorded.frames} frames`,
    );
    console.log(
      `METRIC graphics-fps ${s.scene}: ${recorded.frames} frames, ${recorded.averageFps.toFixed(1)} FPS average, ${recorded.low1Fps.toFixed(1)} 1% low, frame ms p95 ${recorded.frameMs.p95.toFixed(2)}, GPU ms ${s.gpu ? s.gpu.meanMs.toFixed(2) : "unavailable"}`,
    );
  }
  const dir = new URL(`throwaway/perf/${RUN}/`, ROOT);
  await mkdir(dir, { recursive: true });
  const file = new URL(`browser-${new Date().toISOString().replace(/[:.]/g, "-")}.json`, dir);
  const conditions = {
    power,
    build: `browser ${ctx.fixture.build ?? "dev"}`,
    adapter: report.identity.adapters.join(", "),
    userAgent: report.identity.userAgent,
    pace: "display (headless rAF)",
    profile: QUALITY ?? "high/current-client-settings",
  };
  await writeFile(
    file,
    `${JSON.stringify({ client: "browser", conditions, viewport: VIEWPORT, scenes })}\n`,
  );
  console.log(`METRIC graphics-fps: → ${file.pathname}`);
}
