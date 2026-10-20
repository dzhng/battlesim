import { lab, snapshot, presented, hideHud } from "./_lab.mjs";

const VIEWS = { close: 65, tactical: 260, wide: 2400 };
const GPU_SAMPLES = 120;

/** Valid timestamp samples, not elapsed wall time, delimit a measurement arm. */
export async function freshGpuWindow(page) {
  await lab(page, () => {
    if (!window.__lab.stats().gpu) throw new Error("GPU timestamp-query unavailable");
    window.__lab.resetGpuTiming();
  });
  await page.waitForFunction((count) => window.__lab.stats().gpu?.frames >= count, GPU_SAMPLES, {
    timeout: 60000,
    polling: 250,
  });
  return lab(page, () => window.__lab.stats().gpu);
}

/** The three production camera holds, with their own CPU and GPU windows. */
export async function measureViews(ctx, page, state, target) {
  const results = {};
  for (const [name, distance] of Object.entries(VIEWS)) {
    await lab(page, ({ target, distance }) => window.__lab.route.placeView(target, distance), {
      target,
      distance,
    });
    // First apply the requested pose, then wait for that pose's clearance.
    await lab(
      page,
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    );
    await page.waitForFunction(() => window.__lab.clearance().settled, undefined, {
      timeout: 30000,
    });
    await lab(page, () => window.__lab.route.beginViewMeasurement());
    await page.waitForFunction((count) => window.__lab.stats().gpu?.frames >= count, GPU_SAMPLES, {
      timeout: 60000,
      polling: 250,
    });
    results[name] = await lab(page, () => ({
      ...window.__lab.route.viewMeasurement(),
      digest: window.__lab.route.digest(),
      tick: window.__lab.route.tick(),
    }));
    if (state.startsWith("paused"))
      ctx.check(
        `${name} hold preserves paused simulation time`,
        results[name].startSimTime === results[name].simTime,
        JSON.stringify([results[name].startSimTime, results[name].simTime]),
      );
    await snapshot(ctx, page, `${state}-${name}-hud.png`);
    const hidden = await hideHud(page);
    try {
      await snapshot(ctx, page, `${state}-${name}.png`);
    } finally {
      await hidden.evaluate((element) => element.remove());
    }
  }
  await ctx.writeEvidence(`views-${state}.json`, {
    browser: ctx.browser,
    adapter: await lab(page, () => window.__lab.adapter),
    graphics: await lab(page, () => JSON.parse(localStorage.getItem("battle.graphics"))),
    target,
    samples: GPU_SAMPLES,
    results,
  });
  return results;
}

export async function pauseView(page) {
  await lab(page, () => window.__lab.route.pause());
  await presented(page);
}
