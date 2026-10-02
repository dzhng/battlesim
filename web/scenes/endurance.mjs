// Slice 16: the stress battle in real time. Runs ENDURANCE_S seconds (60 by
// default), reset cycles, then the late state for ENDURANCE_LATE_S seconds
// (60 by default); the verdict's run sets both to 300. Budgets (validation.md) are measured and written as evidence, not
// asserted: the scene fails on broken contracts, not on slow hardware.
// ENDURANCE_GENERATED=1 uses the current Metro Large seed-4 world and
// city-arena-1 contact recipe; the default remains the saved 3 × 2 km field.
// MODEL_COST=1 instead measures the models layer's GPU cost at 100 a side
// and with the late state's 20,000 fallen (run it alone,
// under the GPU lock). EFFECT_COST=1 measures the effect pass's the same way,
// in the firefight and in the late state's aftermath, its 2,000 wrecks
// burning.
import { decode, writeCrop } from "./_png.mjs";
import { lab, snapshot, until, openMenu, restart } from "./_lab.mjs";
import { game } from "./_units.mjs";

const CORPSE_CAP = game.presentation.pose.corpses.max;

const SECONDS = Number(process.env.ENDURANCE_S ?? 60);
const LATE_SECONDS = Number(process.env.ENDURANCE_LATE_S ?? 60);
const GENERATED = process.env.ENDURANCE_GENERATED === "1";
const PREPARE_TIMEOUT = GENERATED ? 300000 : 60000;
const CROP = GENERATED ? [5000, 5000, 0] : [1500, 1000, 0];
const telemetry = (page) => lab(page, () => window.__lab.route.telemetry());

/** Choose the bounded full-world arm without changing the saved default. */
async function openStressLab(ctx, page) {
  if (!GENERATED) {
    await ctx.openLab(page);
    return null;
  }
  await ctx.openLab(page, `${ctx.url}?generated=1`, PREPARE_TIMEOUT);
  await page.waitForFunction(() => window.__lab.route?.tick() > 30, undefined, {
    timeout: PREPARE_TIMEOUT,
  });
  const preparation = await lab(page, () => window.__lab.route.preparation?.() ?? null);
  ctx.check(
    "the full generated Metro Large seed4 world is loaded with the contact recipe",
    preparation?.identity.kind === "generated" &&
      preparation.identity.generation.seed === "4" &&
      preparation.request.map_source.request.type === "metro" &&
      preparation.request.map_source.request.size === "large" &&
      preparation.size[0] === 10000 &&
      preparation.size[1] === 10000 &&
      preparation.stress?.kind === "city-arena-1" &&
      preparation.stress.livingUnits.blue === 100 &&
      preparation.stress.livingUnits.red === 100,
    JSON.stringify(preparation),
  );
  await ctx.writeEvidence("preparation.json", preparation);
  return preparation;
}

async function shot(ctx, page, name, crop) {
  const png = await snapshot(ctx, page, `frame-${name}.png`);
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

/** Where the models' cost is read, over blue's densest soldiers: the
 *  camera's framings, plus a mid height where a wide view's soldiers are
 *  still meshes. */
const COST_FRAMINGS = {
  ground: { distance: 25, pitch: 0.22, yaw: -1.57 },
  default: { distance: 65, pitch: 0.85, yaw: -1.57 },
  mid: { distance: 200, pitch: 0.85, yaw: -1.57 },
  strategic: { distance: 2400, pitch: 0.95, yaw: -1.57 },
};

/** The own soldier with the most own soldiers within 100 m. */
const densest = (page) =>
  lab(page, () => {
    const all = window.__lab.route.observation().own.flatMap((u) => u.members);
    let best = all[0] ?? [1500, 1000, 0];
    let most = -1;
    for (const p of all) {
      const n = all.filter((q) => Math.hypot(q[0] - p[0], q[1] - p[1]) < 100).length;
      if (n > most) [best, most] = [p, n];
    }
    return [best[0], best[1]];
  });

/** The known wreck with the most known wrecks within 150 m. */
const densestWrecks = (page) =>
  lab(page, () => {
    const all = window.__lab.route
      .observation()
      .knownProps.filter((p) => p.kind.endsWith("_wreck"))
      .map((p) => p.center);
    let best = all[0] ?? [1500, 1000];
    let most = -1;
    for (const p of all) {
      const n = all.filter((q) => Math.hypot(q[0] - p[0], q[1] - p[1]) < 150).length;
      if (n > most) [best, most] = [p, n];
    }
    return [best[0], best[1]];
  });

/** Paired effects on/off, interleaved 1.5 s batches, median difference of
 *  the frame's GPU time per framing, over `target`. */
async function effectCostAt(page, label, target) {
  const batch = (off) =>
    lab(
      page,
      async (off) => {
        await window.__lab.suppressEffects(off);
        await new Promise((r) => setTimeout(r, 1500));
        return window.__lab.stats().gpu.meanMs;
      },
      off,
    );
  const median = (v) => [...v].sort((a, b) => a - b)[Math.floor(v.length / 2)];
  const out = {};
  for (const [name, framing] of Object.entries(COST_FRAMINGS)) {
    await lab(
      page,
      (f) => {
        const z = window.__lab.route.surfaceZ?.(f.target[0], f.target[1]) ?? 0;
        window.__lab.setCamera({ ...window.__lab.camera(), ...f, target: [...f.target, z] });
      },
      { ...framing, target },
    );
    const on = [];
    const off = [];
    for (let r = 0; r < 5; r++) {
      on.push(await batch(false));
      off.push(await batch(true));
    }
    await lab(page, () => window.__lab.suppressEffects(false));
    out[name] = {
      offMs: median(off),
      effectsMs: median(on.map((v, i) => v - off[i])),
      effects: await lab(page, () => window.__lab.route.effects()),
    };
  }
  console.log(
    `METRIC effects ${label}: ` +
      Object.entries(out)
        .map(
          ([k, v]) =>
            `${k} +${v.effectsMs.toFixed(2)} ms (${v.effects.instances} instances, ${v.effects.sources} smoke sources, ${v.effects.dropped} dropped)`,
        )
        .join("; "),
  );
  return out;
}

async function measureEffectCost(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await openStressLab(ctx, page);
  await page.evaluate(() => setInterval(() => performance.clearMeasures(), 500));
  await page.waitForFunction(() => window.__lab.route?.tick() > 900, undefined, {
    timeout: 300000,
  });
  const live = await effectCostAt(page, "100 a side", await densest(page));
  // The aftermath: fast-forward until the tanks and trucks that died in the
  // last few minutes leave wrecks burning at once. (The late state's 2,000
  // wrecks are part of its map: old, cold remains that never burn.)
  await lab(page, () => window.__lab.route.pause());
  const wrecked = await until(
    page,
    (o) => o.knownProps.filter((p) => p.kind.endsWith("_wreck")).length >= 24,
    30 * 60 * 12,
    150,
  );
  await lab(page, () => window.__lab.route.resume());
  const wrecks = await densestWrecks(page);
  const late = await effectCostAt(
    page,
    `aftermath (tick ${wrecked?.tick}, ${wrecked?.knownProps.filter((p) => p.kind.endsWith("_wreck")).length} wrecks known)`,
    wrecks,
  );
  await lab(
    page,
    (t) => {
      const z = window.__lab.route.surfaceZ?.(t[0], t[1]) ?? 0;
      window.__lab.setCamera({
        ...window.__lab.camera(),
        distance: 200,
        pitch: 0.6,
        yaw: -1.57,
        target: [t[0], t[1], z],
      });
    },
    wrecks,
  );
  await snapshot(ctx, page, "effects-aftermath-1920x1080.png");
  const adapter = await page.evaluate(() => window.__lab.adapter);
  await ctx.writeEvidence("effect-cost.json", { adapter, live, late });
  ctx.check(
    "the aftermath's burning wrecks stay inside the effect budget",
    Object.values(late).every((v) => v.effects.dropped === 0 && v.effects.sources > 5),
    JSON.stringify(Object.values(late).map((v) => v.effects)),
  );
  await page.close();
}

/** Paired models on/off, interleaved 1.5 s batches, median difference of
 *  the frame's GPU time per framing; the pose kernel timed on its own. */
async function modelCostAt(page, label) {
  const batch = (off) =>
    lab(
      page,
      async (off) => {
        await window.__lab.suppressModels(off);
        await new Promise((r) => setTimeout(r, 1500));
        return window.__lab.stats().gpu.meanMs;
      },
      off,
    );
  const median = (v) => [...v].sort((a, b) => a - b)[Math.floor(v.length / 2)];
  const out = {};
  const target = await densest(page);
  for (const [name, framing] of Object.entries(COST_FRAMINGS)) {
    const f = { ...framing, target };
    await lab(
      page,
      (f) => {
        const z = window.__lab.route.surfaceZ?.(f.target[0], f.target[1]) ?? 0;
        window.__lab.setCamera({ ...window.__lab.camera(), ...f, target: [...f.target, z] });
      },
      f,
    );
    const on = [];
    const off = [];
    for (let r = 0; r < 5; r++) {
      on.push(await batch(false));
      off.push(await batch(true));
    }
    await lab(page, () => window.__lab.suppressModels(false));
    const models = await lab(page, () => window.__lab.stats().models);
    const kernel = await lab(page, () => window.__lab.timePoseKernel(50));
    // The kernel as if every soldier on the field were a posed mesh.
    const all = await lab(page, () =>
      window.__lab.timePoseKernel(
        20,
        window.__lab.route.observation().own.reduce((n, u) => n + u.members.length, 0) * 2,
      ),
    );
    out[name] = {
      target,
      offMs: median(off),
      modelsMs: median(on.map((v, i) => v - off[i])),
      kernel,
      kernelAtScale: all,
      models,
    };
  }
  console.log(
    `METRIC models ${label}: ` +
      Object.entries(out)
        .map(
          ([k, v]) =>
            `${k} +${v.modelsMs.toFixed(2)} ms (${v.models.instances} meshes, ${v.models.cards} cards, kernel ${v.kernel ? `${v.kernel.ms.toFixed(3)} ms for ${v.kernel.bodies}` : "idle"}${v.kernelAtScale ? `, ${v.kernelAtScale.ms.toFixed(3)} ms for ${v.kernelAtScale.bodies}` : ""})`,
        )
        .join("; "),
  );
  return out;
}

async function measureModelCost(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await openStressLab(ctx, page);
  // React's development build records a measure per render; a battle running
  // in real time for minutes would exhaust that buffer.
  await page.evaluate(() => setInterval(() => performance.clearMeasures(), 500));
  await page.waitForFunction(() => window.__lab.route?.tick() > 900, undefined, {
    timeout: 300000,
  });
  const live = await modelCostAt(page, "100 a side");
  await openMenu(page);
  // The switch remounts the battle, menu and all: click, not check.
  await page.getByLabel(/Late state/).click();
  await page.waitForFunction(
    () => window.__lab?.route?.late?.() && window.__lab.route.tick() > 30,
    undefined,
    { timeout: 120000 },
  );
  await page.evaluate(() => setInterval(() => performance.clearMeasures(), 500));
  const late = await modelCostAt(page, "late state (20,000 fallen)");
  const adapter = await page.evaluate(() => window.__lab.adapter);
  await ctx.writeEvidence("model-cost.json", { adapter, live, late });
  // Of the 20,000 fallen, presentation draws the newest `corpses.max`.
  ctx.check(
    "the models layer draws the late state's newest corpses, to the cap, as static instances",
    late.strategic.models.corpses === CORPSE_CAP && late.strategic.models.skinned < 2000,
    JSON.stringify({
      corpses: late.strategic.models.corpses,
      skinned: late.strategic.models.skinned,
    }),
  );
  await page.close();
}

export async function run(ctx) {
  if (process.env.MODEL_COST === "1") return measureModelCost(ctx);
  if (process.env.EFFECT_COST === "1") return measureEffectCost(ctx);
  const page = await ctx.newPage();
  const preparation = await openStressLab(ctx, page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 30, undefined, {
    timeout: PREPARE_TIMEOUT,
  });
  await shot(ctx, page, "early-1280x800", CROP);
  const live = await soak(page, SECONDS);
  await shot(ctx, page, "late-run-1280x800", CROP);
  ctx.check(
    `the stress battle runs ${SECONDS} s in real time without failing`,
    !live.failed && live.ticks > 0,
    `${live.ticks} ticks in ${live.wall.toFixed(0)} s = ${live.rate.toFixed(1)} Hz`,
  );

  // Reset cycles keep the GPU allocations where they were.
  const before = (await telemetry(page)).gpu;
  for (let i = 0; i < 3; i++) {
    const was = await lab(page, () => window.__lab.route.tick());
    await restart(page);
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
  await openMenu(page);
  // The switch remounts the battle, menu and all: click, not check.
  await page.getByLabel(/Late state/).click();
  await page.waitForFunction(
    () => window.__lab?.route?.late?.() && window.__lab.route.tick() > 30,
    undefined,
    {
      timeout: GENERATED ? PREPARE_TIMEOUT : 120000,
    },
  );
  const latePreparation = await lab(page, () => window.__lab.route.preparation?.() ?? null);
  if (GENERATED)
    ctx.check(
      "late stress retains the same full generated map",
      latePreparation?.stress.late === true &&
        JSON.stringify(latePreparation.identity) === JSON.stringify(preparation.identity),
      JSON.stringify(latePreparation),
    );
  await shot(ctx, page, "late-state-1280x800", CROP);
  const late = await soak(page, LATE_SECONDS);
  await shot(ctx, page, "late-state-after-1280x800", CROP);
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
    preparation,
    latePreparation,
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
