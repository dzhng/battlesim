// bun run --cwd web scene -- geometry-scaling
// Production build only. Requires prepared WASM/assets and exclusive GPU access.
// Terrain growth control, not a battle/population scaling or automatic timing verdict.
import { hideHud, snapshot } from "./_lab.mjs";

const VIEWS = { close: 65, tactical: 260, wide: 2400 };
const GPU_SAMPLES = 120;

async function hold(page, target, distance) {
  await page.evaluate(({ target, distance }) => window.__lab.route.placeView(target, distance), {
    target,
    distance,
  });
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
  await page.waitForFunction(
    () => window.__lab.clearance().settled && !window.__lab.stats().scenery.dressing.pending,
    undefined,
    { timeout: 30000 },
  );
  let error = null;
  try {
    await page.evaluate(() => window.__lab.route.beginViewMeasurement());
    await page.waitForFunction((count) => window.__lab.stats().gpu?.frames >= count, GPU_SAMPLES, {
      timeout: 60000,
      polling: 250,
    });
  } catch (failure) {
    error = String(failure);
  }
  return page.evaluate(
    (error) => ({
      error,
      ...window.__lab.route.viewMeasurement(),
      input: window.__lab.route.scaling,
      clearance: window.__lab.clearance(),
      validGpuSamples: window.__lab.stats().gpu?.frames ?? 0,
    }),
    error,
  );
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1280, height: 800 } });
  const opened = performance.now();
  try {
    await ctx.openLab(page);
    const startupMs = performance.now() - opened;
    const identity = await page.evaluate(async () => {
      const local = window.__lab.route.exports;
      // Hash the admitted exports before any extension. All arms use this same object.
      const bytes = new TextEncoder().encode(JSON.stringify(local));
      const hash = await crypto.subtle.digest("SHA-256", bytes);
      return {
        map: window.__lab.route.mapId,
        localSha256: Array.from(new Uint8Array(hash), (b) => b.toString(16).padStart(2, "0")).join(
          "",
        ),
        adapter: window.__lab.adapter,
        graphics: JSON.parse(localStorage.getItem("battle.graphics")),
      };
    });
    const target = await page.evaluate(() => window.__lab.route.mapSize.map((v) => v / 2));
    const results = [];
    const controls = {};
    const hidden = await hideHud(page);
    try {
      for (const elevation of [0, 100]) {
        for (const copies of [0, 1, 4, 16, 0]) {
          const started = performance.now();
          await page.evaluate(
            ({ copies, elevation }) => window.__lab.route.setScalingArm(copies, elevation),
            { copies, elevation },
          );
          await page.waitForFunction(
            ({ copies, elevation }) => {
              const arm = window.__lab.route.scaling;
              return arm.copies === copies && arm.elevation === elevation;
            },
            { copies, elevation },
            { timeout: 30000 },
          );
          await page.evaluate(() => window.__lab.frame());
          const installMs = performance.now() - started;
          for (const [view, distance] of Object.entries(VIEWS)) {
            const result = await hold(page, target, distance);
            const name = `${results.length}-${elevation}m-${copies}-${view}`;
            const before = await page.evaluate(() => window.__lab.route.viewFrame());
            await snapshot(ctx, page, `${name}.png`);
            const after = await page.evaluate(() => window.__lab.route.viewFrame());
            const control = controls[view];
            const matched =
              !control ||
              JSON.stringify([
                result.camera,
                result.canvas,
                result.input.light,
                result.input.cameraConfig,
                result.simTime,
                result.stats?.width,
                result.stats?.height,
              ]) ===
                JSON.stringify([
                  control.camera,
                  control.canvas,
                  control.input.light,
                  control.input.cameraConfig,
                  control.simTime,
                  control.stats?.width,
                  control.stats?.height,
                ]);
            const sameSubmission =
              !control ||
              JSON.stringify(result.stats?.submitted) === JSON.stringify(control.stats?.submitted);
            results.push({
              name,
              view,
              copies,
              elevation,
              installMs,
              matched,
              sameSubmission,
              result,
              picture: { before, after },
            });
            if (!control) controls[view] = result;
            // Save partial evidence before failing a missing stream or a comparison.
            await ctx.writeEvidence("scaling.json", {
              identity,
              browser: ctx.browser,
              startupMs,
              target,
              minimumGpuSamples: GPU_SAMPLES,
              graphicsQuality: ctx.graphicsQuality,
              results,
              limits:
                "Remote terrain only; no worker/ticks. Traversal and CPU/GPU distributions require review; screenshots require visual acceptance. Timing is separate from screenshot readback.",
            });
            ctx.check(
              `${name} complete GPU window`,
              !result.error && result.validGpuSamples >= GPU_SAMPLES,
              result.error ?? String(result.validGpuSamples),
            );
            ctx.check(`${name} matched camera/light/dimensions/clock`, matched);
            // Leaf boundaries may change with a rebuilt hierarchy. Retain exact
            // submission equality as a diagnostic, not an invented acceptance bar.
            if (result.error) throw new Error(result.error);
          }
        }
      }
    } finally {
      await hidden.evaluate((element) => element.remove());
    }
  } finally {
    await page.close();
  }
}
