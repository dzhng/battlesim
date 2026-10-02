// A block of a generated town, drawn from the template art library: every
// building is its template's rows, as instances of kit modules, where the map
// puts it; a chunk near the camera is expanded into a bounded pool and the
// rest of the map draws at the coarsest tier; a building seen to fall draws
// its remains; and replacing the buildings returns what the old ones held.
//
// `BUILDING_COST=1` measures instead: the frame's GPU time with and without
// the buildings at the stations (paired, interleaved), on whatever map the
// lab's address names (`CITY_MAP=metro:large:1`).
import { writeFile } from "node:fs/promises";
import { aim, buildingsSettled, lab } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";

const HIDE_PANEL = "[data-testid=city-block-panel] { display: none !important; }";
/** The ground mask's two values: a pixel that is mostly ground, and one that is not. */
const isGround = ([r]) => r > 200;
const isBody = ([r]) => r < 55;
/** Straight down, so nothing standing hides the ground beside it. */
const TOP_DOWN = Math.PI / 2 - 0.03;
const STATIONS = ["street", "tactical", "wide", "overview"];
const delta = (a, b) => a.reduce((s, v, i) => s + Math.abs(v - b[i]), 0);

const stats = (page) => lab(page, () => window.__lab.stats().buildings);

/** Cut the camera to a station as the rig places it, and wait until it has
 *  come to rest and every near chunk is expanded. */
async function stand(page, id) {
  await lab(page, (id) => window.__lab.route.stand(id), id);
  await lab(page, () => window.__lab.frame());
  await page.waitForFunction(() => window.__lab.clearance().settled, undefined, { timeout: 15000 });
  await buildingsSettled(page);
}

async function shot(ctx, page, file) {
  await lab(page, () => window.__lab.frame());
  const png = await page.screenshot();
  await writeFile(ctx.evidencePath(file), png);
  return decode(png);
}

/** The frame's ground mask (fog's own channel: read with fog on, which a lab
 *  without fog input has as "all seen"). */
async function groundMask(ctx, page, file) {
  await lab(page, () => window.__lab.setFrameView("ground-mask"));
  const png = await page.screenshot();
  await writeFile(ctx.evidencePath(file), png);
  await lab(page, () => window.__lab.setFrameView("final"));
  return decode(png);
}

const tiers = (s) =>
  `instances ${s.tiers.join("/")}, triangles ${s.triangles.map((t) => Math.round(t)).join("/")}, ${s.draws} draws (+${s.casterDraws} a cascade, ${Math.round(s.casterTriangles)} triangles), ${s.residentChunks} of ${s.chunks} chunks resident, pool ${s.pool.used} of ${s.pool.capacity}`;

/** The frame's GPU time over `frames` forced redraws from now (the lab draws
 *  on demand, and setting the frame view starts the timer's window again). */
async function gpuMs(page, frames = 120) {
  return lab(
    page,
    async (frames) => {
      await window.__lab.setFrameView("final");
      for (let i = 0; i < frames; i++) await window.__lab.frame();
      return window.__lab.stats().gpu;
    },
    frames,
  );
}

/** `BUILDING_COST=1`: paired GPU time, buildings drawn against every other
 *  frame batch without them, at each station. */
async function cost(ctx, page) {
  const rows = [];
  for (const id of STATIONS) {
    await stand(page, id);
    const drawn = await stats(page);
    const differences = [];
    const absolute = [];
    for (let round = 0; round < 6; round++) {
      await lab(page, () => window.__lab.suppressBuildings(false));
      const on = await gpuMs(page);
      await lab(page, () => window.__lab.suppressBuildings(true));
      const off = await gpuMs(page);
      if (on && off) {
        differences.push(on.meanMs - off.meanMs);
        absolute.push(on.meanMs);
      }
    }
    await lab(page, () => window.__lab.suppressBuildings(false));
    const median = (list) => [...list].sort((a, b) => a - b)[Math.floor(list.length / 2)];
    const memory = await lab(page, () => window.__lab.stats().memory);
    rows.push({ id, drawn, median: median(differences), frame: median(absolute), differences });
    console.log(
      `METRIC city-block cost ${id}: buildings ${median(differences).toFixed(2)} ms of a ${median(absolute).toFixed(2)} ms frame (paired differences ${differences.map((d) => d.toFixed(2)).join(", ")}); ${tiers(drawn)}; select ${drawn.selectMs.toFixed(2)} ms; buffers ${(memory.bufferBytes / 2 ** 20).toFixed(1)} MiB, textures ${(memory.textureBytes / 2 ** 20).toFixed(1)} MiB`,
    );
  }
  await ctx.writeEvidence("cost.json", {
    adapter: await page.evaluate(() => window.__lab.adapter),
    map: await lab(page, () => window.__lab.route.map()),
    rows,
  });
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  const warnings = [];
  page.on("console", (m) => {
    if (m.type() === "warning" && /webgpu|validation|gpu\w*error/i.test(m.text()))
      warnings.push(m.text().slice(0, 200));
  });
  const asked = process.env.CITY_MAP?.split(":");
  await page.goto(
    asked ? `${ctx.url}?type=${asked[0]}&size=${asked[1]}&seed=${asked[2]}` : ctx.url,
  );
  await page.waitForFunction(
    () =>
      document.querySelector("[data-testid=error]") ||
      (window.__lab?.ready && window.__lab.route?.stations && window.__lab.stats),
    undefined,
    { timeout: 300000 },
  );
  await page.waitForFunction(
    async () => {
      await window.__lab.frame();
      return window.__lab.stats().buildings.buildings > 0;
    },
    undefined,
    { timeout: 120000, polling: 100 },
  );
  await page.addStyleTag({ content: HIDE_PANEL });
  const map = await lab(page, () => window.__lab.route.map());
  const built = await stats(page);
  console.log(
    `METRIC city-block ${map.type} ${map.size} seed ${map.seed}: ${built.buildings} buildings in ${built.chunks} chunks, ${built.coarse} coarse rows, scene built in ${built.buildMs.toFixed(1)} ms (development build)`,
  );
  if (process.env.BUILDING_COST === "1") return cost(ctx, page);

  // Every building of the map is a template reference, drawn from rows.
  const counts = await lab(page, () => ({
    ...window.__lab.route.counts(),
    references: window.__lab.route.buildings().length,
    structures: window.__lab.stats().structures,
    block: window.__lab.route.block(),
  }));
  ctx.check(
    "every building of the generated map has art and is drawn from its template's rows, none as a fitted model",
    counts.buildings > 100 &&
      counts.drawn === counts.buildings &&
      built.buildings === counts.buildings &&
      counts.references === counts.buildings &&
      // The whole map can draw at the coarsest tier: a row or more a building.
      built.coarse >= built.buildings &&
      counts.structures === 0 &&
      counts.block.apartment !== null &&
      counts.block.house !== null,
    JSON.stringify({ ...counts, coarse: built.coarse }),
  );

  // The stations: what each draws, and its picture.
  const at = {};
  for (const id of STATIONS) {
    await stand(page, id);
    await shot(ctx, page, `${id}-1920x1080.png`);
    at[id] = await stats(page);
    console.log(
      `METRIC city-block ${id}: ${tiers(at[id])}; select ${at[id].selectMs.toFixed(2)} ms, ${at[id].expandedRows} rows expanded, ${at[id].uploadBytes} bytes uploaded`,
    );
  }
  const drawnOnce = (s) => s.tiers.reduce((n, t) => n + t, 0) > 0 && s.pool.used <= s.pool.capacity;
  ctx.check(
    "near the block its chunks are expanded at the finest tiers; at the overview the whole map draws at the coarsest with nothing in the pool",
    STATIONS.every((id) => drawnOnce(at[id]) && at[id].refused === 0) &&
      at.street.tiers[0] > 0 &&
      at.tactical.tiers[0] > 0 &&
      at.tactical.residentChunks > 0 &&
      at.wide.residentChunks > at.tactical.residentChunks &&
      at.overview.residentChunks === 0 &&
      at.overview.pool.used === 0 &&
      at.overview.tiers[3] === built.coarse &&
      at.overview.tiers.slice(0, 3).every((t) => t === 0),
    JSON.stringify(Object.fromEntries(STATIONS.map((id) => [id, tiers(at[id])]))),
  );

  // A still camera does no work: frames at one station expand nothing more.
  await stand(page, "tactical");
  const still = await lab(page, async () => {
    const before = window.__lab.stats().buildings;
    for (let i = 0; i < 30; i++) await window.__lab.frame();
    const after = window.__lab.stats().buildings;
    return [before, after].map((s) => [s.totalExpandedRows, s.totalUploadBytes]);
  });
  ctx.check(
    "a still camera expands and uploads nothing",
    still[0][0] === still[1][0] && still[0][1] === still[1][1],
    JSON.stringify(still),
  );

  // A building is where the map puts it: from straight above, not ground at
  // the middle of each part's roof, ground beside the building.
  const placed = await lab(page, () => {
    const buildings = window.__lab.route.buildings();
    const boxes = buildings.flatMap((b) => b.parts);
    const inside = (b, x, y) => {
      const [dx, dy] = [x - b.center[0], y - b.center[1]];
      const [c, s] = [Math.cos(b.yaw), Math.sin(b.yaw)];
      return (
        Math.abs(dx * c + dy * s) <= b.half[0] + 2 && Math.abs(-dx * s + dy * c) <= b.half[1] + 2
      );
    };
    const block = window.__lab.route.block();
    return [block.apartment, block.house].map((index) => {
      const building = buildings[index];
      const box = building.parts[0];
      const reach = Math.hypot(box.half[0], box.half[1]) + 6;
      let ground = null;
      for (let k = 0; k < 16 && !ground; k++) {
        const [x, y] = [
          box.center[0] + reach * Math.cos((k * Math.PI) / 8),
          box.center[1] + reach * Math.sin((k * Math.PI) / 8),
        ];
        if (!boxes.some((b) => inside(b, x, y)) && !window.__lab.route.surfaceAt(x, y)?.forest)
          ground = [x, y];
      }
      return { ...building, ground };
    });
  });
  const where = [];
  for (const building of placed) {
    const box = building.parts[0];
    await aim(page, box.center, { distance: 90, pitch: TOP_DOWN });
    await buildingsSettled(page);
    const roofs = [];
    for (const part of building.parts)
      roofs.push(
        await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), [
          ...part.center,
          part.baseZ + 2 * part.half[2],
        ]),
      );
    const groundPx = await lab(
      page,
      (p) => window.__lab.projectToCss(p[0], p[1], window.__lab.route.surfaceZ(p[0], p[1])),
      building.ground,
    );
    const name = building.category;
    const mask = await groundMask(ctx, page, `placed-${name}-ground-mask.png`);
    const frame = await shot(ctx, page, `placed-${name}-1920x1080.png`);
    where.push({
      template: building.template,
      roofs: roofs.map((px) => pixel(mask, ...px)),
      ground: pixel(mask, ...groundPx),
      contrast: delta(pixel(frame, ...roofs[0]), pixel(frame, ...groundPx)),
    });
  }
  ctx.check(
    "an apartment building and a house are drawn where the map puts them: not ground at each part's roof, ground beside the building",
    where.every((w) => w.roofs.every(isBody) && isGround(w.ground) && w.contrast > 30),
    JSON.stringify(where),
  );

  // A pan across the town and a zoom through every tier keep the pool inside
  // its bound, and coming back to a station finds what it left.
  await stand(page, "tactical");
  const start = await stats(page);
  const tactical = await lab(page, () => window.__lab.camera());
  const worst = { used: 0, resident: 0, refused: 0, selectMs: 0, expanded: 0, draws: 0 };
  const note = (s) => {
    worst.used = Math.max(worst.used, s.pool.used);
    worst.resident = Math.max(worst.resident, s.residentChunks);
    worst.refused = Math.max(worst.refused, s.refused);
    worst.selectMs = Math.max(worst.selectMs, s.selectMs);
    worst.expanded = Math.max(worst.expanded, s.expandedRows);
    worst.draws = Math.max(worst.draws, s.draws);
  };
  const frames = [];
  // Across the town through the block, 30 m a step, at the tactical camera.
  for (let m = -600; m <= 600; m += 30)
    frames.push({ target: [tactical.target[0] + m, tactical.target[1] + m / 3], distance: 65 });
  // Out and back in over the block, a twelfth of an octave a step.
  for (let d = 25; d <= 4000; d *= 2 ** (1 / 4))
    frames.push({ target: tactical.target, distance: d });
  let bounded = true;
  for (const f of frames) {
    const s = await lab(
      page,
      async (f) => {
        window.__lab.setCamera({
          ...window.__lab.camera(),
          target: [f.target[0], f.target[1], window.__lab.route.surfaceZ(f.target[0], f.target[1])],
          distance: f.distance,
        });
        await window.__lab.frame();
        return window.__lab.stats().buildings;
      },
      f,
    );
    note(s);
    bounded &&= s.pool.used <= s.pool.capacity && s.tiers.every((t) => t >= 0);
  }
  await stand(page, "tactical");
  const back = await stats(page);
  ctx.check(
    "a pan across the town and a zoom through every tier keep the pool inside its bound, and the station draws the same on return",
    bounded &&
      worst.used > start.pool.used &&
      back.pool.used === start.pool.used &&
      back.residentChunks === start.residentChunks &&
      back.tiers.join() === start.tiers.join(),
    JSON.stringify({ worst, start: tiers(start), back: tiers(back), frames: frames.length }),
  );
  console.log(
    `METRIC city-block pan and zoom (${frames.length} views): pool peak ${worst.used} of ${start.pool.capacity} records, ${worst.resident} chunks resident at most, ${worst.refused} refused, worst view change ${worst.selectMs.toFixed(2)} ms for ${worst.expanded} rows, ${worst.draws} draws at most`,
  );

  // The side sees the apartment building fall: it leaves the intact rows and
  // its remains draw; everything else stays as it was.
  await lab(page, () => window.__lab.route.setFallen(true));
  await buildingsSettled(page);
  const fallen = await stats(page);
  await shot(ctx, page, "fallen-1920x1080.png");
  await lab(page, () => window.__lab.route.setFallen(false));
  await buildingsSettled(page);
  const restored = await stats(page);
  ctx.check(
    "a building seen to fall draws its remains and no longer its rows, and only its own chunk is expanded again",
    fallen.fallen === 1 &&
      fallen.ruins > 0 &&
      fallen.pool.used < start.pool.used &&
      fallen.residentChunks === start.residentChunks &&
      fallen.expandedRows < start.pool.used &&
      restored.fallen === 0 &&
      restored.ruins === 0 &&
      restored.pool.used === start.pool.used,
    JSON.stringify({
      ruins: fallen.ruins,
      pool: [start.pool.used, fallen.pool.used, restored.pool.used],
      expanded: fallen.expandedRows,
    }),
  );

  // Replacing the map's buildings returns what the old ones held: other
  // references in, the first back, and the device holds what it did.
  const baseline = await lab(page, () => window.__lab.allocations());
  await lab(page, () => window.__lab.route.setHalved(true));
  await buildingsSettled(page);
  const halved = {
    stats: await stats(page),
    live: await lab(page, () => window.__lab.allocations()),
  };
  await lab(page, () => window.__lab.route.setHalved(false));
  await buildingsSettled(page);
  const again = {
    stats: await stats(page),
    live: await lab(page, () => window.__lab.allocations()),
  };
  ctx.check(
    "replacing the buildings returns live GPU buffers and bytes to baseline",
    halved.stats.buildings === Math.ceil(built.buildings / 2) &&
      halved.live.buffers === baseline.buffers &&
      halved.live.bufferBytes < baseline.bufferBytes &&
      again.stats.buildings === built.buildings &&
      again.stats.pool.used === start.pool.used &&
      again.live.buffers === baseline.buffers &&
      again.live.bufferBytes === baseline.bufferBytes &&
      again.live.textures === baseline.textures &&
      again.live.textureBytes === baseline.textureBytes,
    `baseline ${JSON.stringify(baseline)} halved ${JSON.stringify(halved.live)} again ${JSON.stringify(again.live)}`,
  );
  // And a rebuilt frame holds what the first did.
  await lab(page, () => window.__lab.rebuild());
  await buildingsSettled(page);
  const first = await lab(page, () => window.__lab.allocations());
  await lab(page, () => window.__lab.rebuild());
  await buildingsSettled(page);
  const rebuilt = await lab(page, () => window.__lab.allocations());
  ctx.check(
    "rebuilding the frame returns live GPU buffers and textures, the buildings' included, to baseline",
    rebuilt.buffers === first.buffers &&
      rebuilt.bufferBytes === first.bufferBytes &&
      rebuilt.textures === first.textures &&
      rebuilt.textureBytes === first.textureBytes,
    `first ${JSON.stringify(first)} after ${JSON.stringify(rebuilt)}`,
  );

  ctx.check(
    "no WebGPU validation warning was logged",
    warnings.length === 0,
    warnings.slice(0, 3).join(" | "),
  );
  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [1920, 1080],
    dpr: 1,
    map,
    built,
    stations: at,
    worst,
    memory: await lab(page, () => window.__lab.stats().memory),
    models: await lab(page, () => {
      const m = window.__lab.stats().models;
      return { textureLayers: m.textureLayers, textureBytes: m.textureBytes };
    }),
  });
}
