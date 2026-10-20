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

/** Timing and picture runs are separate: screenshot readback must not cost a timed frame. */
export async function measureTransitions(ctx, page, state, target) {
  const durationMs = 12_000;
  const legs = [
    { name: "close-tactical", to: VIEWS.tactical, pan: [80, 40], yaw: -1.17 },
    { name: "tactical-wide", to: VIEWS.wide, pan: [-80, -40], yaw: -1.97 },
    { name: "wide-close", to: VIEWS.close, pan: [0, 0], yaw: -1.57 },
  ];
  const results = {};
  await lab(page, ({ target, distance }) => window.__lab.route.placeView(target, distance), {
    target,
    distance: VIEWS.close,
  });
  await lab(
    page,
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
  await page.waitForFunction(() => window.__lab.clearance().settled, undefined, { timeout: 30000 });
  for (const leg of legs) {
    const destination = [target[0] + leg.pan[0], target[1] + leg.pan[1]];
    await lab(
      page,
      ({ destination, leg, durationMs }) =>
        window.__lab.route.transitionView(destination, leg.to, leg.yaw, durationMs),
      { destination, leg, durationMs },
    );
    let error = null;
    try {
      await page.waitForFunction(() => window.__lab.route.viewMeasurement()?.complete, undefined, {
        timeout: 60000,
        polling: 250,
      });
    } catch (failure) {
      error = String(failure);
    }
    const result = await lab(page, () => ({
      ...window.__lab.route.viewMeasurement(),
      digest: window.__lab.route.digest(),
      tick: window.__lab.route.tick(),
    }));
    results[leg.name] = { ...result, error };
    // Persist partial evidence before reporting a missing/slow sample stream.
    await ctx.writeEvidence(`transitions-${state}.json`, {
      browser: ctx.browser,
      adapter: await lab(page, () => window.__lab.adapter),
      graphics: await lab(page, () => JSON.parse(localStorage.getItem("battle.graphics"))),
      preparation: await lab(page, () => window.__lab.route.preparation()),
      target,
      durationMs,
      minimumGpuSamples: GPU_SAMPLES,
      results,
    });
    ctx.check(
      `${state} ${leg.name} has a complete moving GPU window`,
      !error && result.complete && result.validGpuSamples >= GPU_SAMPLES,
      JSON.stringify({ complete: result.complete, validGpuSamples: result.validGpuSamples, error }),
    );
    if (state.startsWith("paused"))
      ctx.check(
        `${leg.name} preserves paused simulation time`,
        result.startSimTime === result.simTime,
        JSON.stringify([result.startSimTime, result.simTime]),
      );
    if (error) throw new Error(error);
  }

  // Replay the same continuous path for HUD and clean geometry evidence. Live
  // captures have later simulation times; retain them rather than imply parity.
  const pictures = [];
  for (const clean of [false, true]) {
    const hidden = clean ? await hideHud(page) : null;
    try {
      await lab(page, ({ target, distance }) => window.__lab.route.placeView(target, distance), {
        target,
        distance: VIEWS.close,
      });
      await lab(
        page,
        () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
      );
      await page.waitForFunction(() => window.__lab.clearance().settled, undefined, {
        timeout: 30000,
      });
      for (const leg of legs) {
        const destination = [target[0] + leg.pan[0], target[1] + leg.pan[1]];
        await lab(
          page,
          ({ destination, leg, durationMs }) =>
            window.__lab.route.transitionView(destination, leg.to, leg.yaw, durationMs),
          { destination, leg, durationMs },
        );
        for (const fraction of [0.25, 0.5, 0.75]) {
          await page.waitForFunction(
            (elapsed) => window.__lab.route.viewMeasurement()?.frames.at(-1)?.elapsedMs >= elapsed,
            durationMs * fraction,
            { timeout: 30000, polling: 50 },
          );
          const name = `${state}-${leg.name}-${fraction}${clean ? "" : "-hud"}.png`;
          const before = await lab(page, () => window.__lab.route.viewFrame());
          await snapshot(ctx, page, name);
          const after = await lab(page, () => window.__lab.route.viewFrame());
          pictures.push({
            name,
            requestedFraction: fraction,
            before: {
              camera: before.camera,
              simTime: before.simTime,
              now: before.now,
            },
            after: {
              camera: after.camera,
              simTime: after.simTime,
              now: after.now,
            },
          });
        }
        await page.waitForFunction(
          () => window.__lab.route.viewMeasurement()?.complete,
          undefined,
          { timeout: 30000 },
        );
      }
    } finally {
      await hidden?.evaluate((element) => element.remove());
    }
  }
  await ctx.writeEvidence(`transition-pictures-${state}.json`, { timing: false, pictures });
}
