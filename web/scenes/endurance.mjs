// Slice 16: the stress battle in real time. Runs ENDURANCE_S seconds (60 by
// default), reset cycles, then the late state for ENDURANCE_LATE_S seconds
// (60 by default); the verdict's run sets both to 300. Budgets (validation.md) are measured and written as evidence, not
// asserted: the scene fails on broken contracts, not on slow hardware.
// ENDURANCE_GENERATED=1 uses the current Metro XL seed-4 world and
// city-arena-2 contact recipe; the default remains the saved 3 × 2 km field.
// VIEW_COST=1 with ENDURANCE_GENERATED=1 measures close/tactical/wide holds.
// VIEW_TRANSITIONS=1 adds separately timed moving windows and untimed picture replays.
// VIEW_COST=paused stops after pinned tick-90 holds; VIEW_COST_REPEATS repeats them.
// MODEL_COST=1 instead measures the models layer's GPU cost at 100 a side
// and with the late state's 800 fallen (run it alone,
// under the GPU lock). EFFECT_COST=1 measures the effect pass's the same way,
// in the firefight and in the late state's aftermath, its 100 wrecks
// burning.
import { decode, writeCrop } from "./_png.mjs";
import { lab, snapshot, until, openMenu } from "./_lab.mjs";
import { installResetRafFence, restartPresentedOpening } from "./_reset.mjs";
import { freshGpuWindow, measureViews, measureTransitions, pauseView } from "./_viewCost.mjs";
import { visuals } from "./_units.mjs";

const CORPSE_CAP = visuals.pose.corpses.max;
const LATE_CORPSES = 800;

const SECONDS = Number(process.env.ENDURANCE_S ?? 60);
const LATE_SECONDS = Number(process.env.ENDURANCE_LATE_S ?? 60);
const GENERATED = process.env.ENDURANCE_GENERATED === "1";
const PREPARE_TIMEOUT = GENERATED ? 300000 : 60000;
const CROP = GENERATED ? [5000, 5000, 0] : [1500, 1000, 0];
const telemetry = (page) => lab(page, () => window.__lab.route.telemetry());

/** Choose the bounded full-world arm without changing the saved default. */
async function openStressLab(ctx, page) {
  const quality = process.env.ENDURANCE_QUALITY;
  if (quality) {
    if (!["low", "medium", "high"].includes(quality))
      throw new Error("ENDURANCE_QUALITY must name a production tier: low, medium, or high");
    await page.addInitScript(
      (mode) =>
        localStorage.setItem(
          "battle.graphics",
          JSON.stringify({ mode, detectedMode: null, lastResult: null }),
        ),
      quality,
    );
  }
  if (!GENERATED) {
    await ctx.openLab(page);
    return null;
  }
  const stressUrl = new URL(
    ctx.urlFor({ generated: 1, ...(process.env.VIEW_COST ? { views: 1 } : {}) }),
  );
  await ctx.openLab(page, stressUrl.href, PREPARE_TIMEOUT);
  await page.waitForFunction(() => window.__lab.route?.tick() > 30, undefined, {
    timeout: PREPARE_TIMEOUT,
  });
  const preparation = await lab(page, () => window.__lab.route.preparation?.() ?? null);
  ctx.check(
    "the full generated Metro XL seed4 world is loaded with the contact recipe",
    preparation?.identity.kind === "generated" &&
      preparation.identity.generation.seed === "4" &&
      preparation.request.map_source.request.type === "metro" &&
      preparation.request.map_source.request.size === "xl" &&
      preparation.size[0] === 10000 &&
      preparation.size[1] === 10000 &&
      preparation.stress?.kind === "city-arena-2" &&
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
  const firingUnitsSeen = new Set();
  const shotCounters = () =>
    lab(page, () =>
      window.__lab.route
        .observation()
        .own.map((u) => [u.id, u.weaponPoses.reduce((shots, p) => shots + p.shots, 0)]),
    );
  const checkContact = GENERATED && seconds >= 30;
  let previousShots = new Map(checkContact ? await shotCounters() : []);
  let failed = false;
  while (Date.now() - started < seconds * 1000) {
    await page.waitForTimeout(Math.min(10_000, seconds * 1000 - (Date.now() - started)));
    const t = await telemetry(page);
    if (checkContact) {
      const shots = await shotCounters();
      for (const [id, count] of shots)
        if (count > (previousShots.get(id) ?? 0)) firingUnitsSeen.add(id);
      previousShots = new Map(shots);
    }
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
    firingUnitsSeen: firingUnitsSeen.size,
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

/** Paired effects on/off, interleaved fresh GPU windows, median difference of
 *  the frame's GPU time per framing, over `target`. */
async function effectCostAt(page, label, target) {
  const batch = async (off) => {
    await lab(page, (off) => window.__lab.suppressEffects(off), off);
    return (await freshGpuWindow(page)).meanMs;
  };
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
  // last few minutes leave wrecks burning at once. (The late state's 100
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

/** Paired models on/off, interleaved fresh GPU windows, median difference of
 *  the frame's GPU time per framing. */
async function modelCostAt(page, label) {
  const batch = async (off) => {
    await lab(page, (off) => window.__lab.suppressModels(off), off);
    return (await freshGpuWindow(page)).meanMs;
  };
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
    out[name] = {
      target,
      offMs: median(off),
      modelsMs: median(on.map((v, i) => v - off[i])),
      models,
    };
  }
  console.log(
    `METRIC models ${label}: ` +
      Object.entries(out)
        .map(
          ([k, v]) =>
            `${k} +${v.modelsMs.toFixed(2)} ms (${v.models.instances} meshes, ${v.models.cards} cards)`,
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
  const late = await modelCostAt(page, `late state (${LATE_CORPSES} fallen)`);
  const adapter = await page.evaluate(() => window.__lab.adapter);
  await ctx.writeEvidence("model-cost.json", { adapter, live, late });
  // Presentation draws all realistic late-state corpses, up to `corpses.max`.
  ctx.check(
    "the models layer draws the late state's corpses as static instances",
    late.strategic.models.corpses === Math.min(CORPSE_CAP, LATE_CORPSES) &&
      late.strategic.models.skinned < 2000,
    JSON.stringify({
      corpses: late.strategic.models.corpses,
      skinned: late.strategic.models.skinned,
    }),
  );
  await page.close();
}

async function measureViewCost(ctx) {
  if (!GENERATED) throw new Error("VIEW_COST requires ENDURANCE_GENERATED=1");
  const page = await ctx.newPage();
  try {
    await openStressLab(ctx, page);
    await page.waitForFunction(
      () => window.__lab.route.tick() === 90 && window.__lab.route.viewReady(),
      undefined,
      { timeout: PREPARE_TIMEOUT },
    );
    await pauseView(page);
    const target = await densest(page);
    const repeats = Number(process.env.VIEW_COST_REPEATS ?? 1);
    if (!Number.isInteger(repeats) || repeats < 1 || repeats > 10)
      throw new Error("VIEW_COST_REPEATS must be an integer from 1 to 10");
    for (let repeat = 0; repeat < repeats; repeat++) {
      const state = repeats === 1 ? "paused" : `paused-${repeat + 1}`;
      await measureViews(ctx, page, state, target);
      if (process.env.VIEW_TRANSITIONS === "1") await measureTransitions(ctx, page, state, target);
    }
    if (process.env.VIEW_COST === "paused") return;
    await lab(page, () => window.__lab.route.releaseViewWarmup());
    await page.waitForFunction(() => window.__lab.route.viewWarmed());
    await measureViews(ctx, page, "live", target);
    if (process.env.VIEW_TRANSITIONS === "1") await measureTransitions(ctx, page, "live", target);
    await openMenu(page);
    await page.getByLabel(/Late state/).click();
    await page.waitForFunction(
      () =>
        window.__lab?.route?.late?.() &&
        window.__lab.route.tick() === 90 &&
        window.__lab.route.viewReady(),
      undefined,
      { timeout: PREPARE_TIMEOUT },
    );
    await lab(page, () => window.__lab.route.releaseViewWarmup());
    await page.waitForFunction(() => window.__lab.route.viewWarmed());
    await measureViews(ctx, page, "late", target);
    if (process.env.VIEW_TRANSITIONS === "1") await measureTransitions(ctx, page, "late", target);
  } finally {
    await page.close();
  }
}

export async function run(ctx) {
  if (process.env.VIEW_COST) {
    if (!["1", "paused"].includes(process.env.VIEW_COST))
      throw new Error("VIEW_COST must be 1 or paused");
    return measureViewCost(ctx);
  }
  if (process.env.MODEL_COST === "1") return measureModelCost(ctx);
  if (process.env.EFFECT_COST === "1") return measureEffectCost(ctx);
  const page = await ctx.newPage();
  await page.addInitScript(installResetRafFence);
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

  // Equivalent battle state also needs equivalent death-animation history.
  // Present each paused tick before counting the warmed resources.
  const resetStates = [];
  for (let i = 0; i < 3; i++) resetStates.push(await restartPresentedOpening(page, 90));
  const resetAllocations = resetStates.map((state) => state.gpu);
  ctx.check(
    "every reset cycle draws the buildings that stand",
    resetStates.every((state) => state.buildings.standing > 0 && state.buildings.drawn > 0),
    JSON.stringify(resetStates.map((state) => state.buildings)),
  );
  ctx.check(
    "reset cycles return live GPU buffers and textures to the same counts and bytes at tick 90",
    resetAllocations.every((a) => JSON.stringify(a) === JSON.stringify(resetAllocations[0])),
    JSON.stringify(resetAllocations),
  );
  ctx.check(
    "reset cycles present the same battle, clock, corpse IDs and camera",
    resetStates.every(
      (state) => JSON.stringify(state.presentation) === JSON.stringify(resetStates[0].presentation),
    ),
    JSON.stringify(resetStates),
  );

  // The late state: the full roster's 800 fallen soldiers and 100 wrecks.
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
    resetAllocations,
    resetStates,
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
    if (GENERATED && (name === "live" ? SECONDS : LATE_SECONDS) >= 30)
      ctx.check(
        `${name} generated contact involves multiple own units firing during measurement`,
        run.firingUnitsSeen >= 10,
        `${run.firingUnitsSeen} units with actual shot counters`,
      );
    const worst = run.samples.reduce(
      (w, s) => ({ p95: Math.max(w.p95, s.frames.p95), p99: Math.max(w.p99, s.frames.p99) }),
      { p95: 0, p99: 0 },
    );
    const last = run.samples.at(-1);
    console.log(
      `METRIC endurance ${name}: sim ${run.rate.toFixed(1)} Hz; worst 10 s window frame p95 ${worst.p95.toFixed(1)} ms, p99 ${worst.p99.toFixed(1)} ms; GPU frame ${last?.gpuFrame?.meanMs?.toFixed(1) ?? "n/a"} ms mean / ${last?.gpuFrame?.p95Ms?.toFixed(1) ?? "n/a"} ms p95; main-thread heap ${last?.heapMiB?.toFixed(0)} MiB; GPU ${(last?.gpu.bufferBytes / 2 ** 20).toFixed(1)} MiB in ${last?.gpu.buffers} buffers`,
    );
  }
}
