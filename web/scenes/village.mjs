// Slice 15: the village battle. Blue plays through the production controls;
// the encounter status, variant, seed, pause/reset and replay export work.
// Battle-look slice 09: the camera tour, from the opening framing out to the
// strategic height and in to the ground, through the real wheel.
// Battle-look slice 19: the tree-line tour, the forests drawn as trees, and
// the scenery's GPU resources returned on rebuild.
// Battle-look slice 18: the grass field at those framings (GRASS_COST=1 also
// measures its GPU cost; run it alone, under the GPU lock).
// Battle-look slice 23: soldiers as posed models, by detail tier and as
// impostor cards, never fogged, picked by the simulation's boxes, and the
// fallen as static corpses.
// Battle-look slice 24: vehicles, buildings and wrecks as their appearances:
// every vehicle a posed model following its published weapon poses, the
// village's houses fitted to their boxes, and a tank firing (recoil).
// Battle-look slice 35: order markers and the Space overlay (a real
// right-drag's facing, a reverse move's marker, Space held at the default and
// ground cameras over the fog), and no enemy plan in the observation.
// Battle-look slice 25: combat effects in the firefight, a burst read at its
// moment and after, from the effects' own frame and the pass inspector's
// world view.
import { readFile } from "node:fs/promises";
import { lab, obs, advance, until, snapshot } from "./_lab.mjs";
import { decode, pixel, writeCrop } from "./_png.mjs";
import { checkOverlayIsolation, paintOnly } from "./_overlays.mjs";
import { cleanupTour, woodsTour } from "./_battleLook.mjs";

const village = JSON.parse(
  await readFile(new URL("../../fixtures/village.json", import.meta.url), "utf8"),
);
const CAMERA = village.presentation.camera;
/** Ground paint is drawn at the ground itself (the ground and its blades
 *  read it at their own point), so checks read the marks at the surface. */
const MARK_LIFT_M = 0;
/** Order marks (the scheme's overlay layer, `yellow-orders`) are drawn at
 *  the orders' height over the surface: checks read them there. */
const ORDER_MARK_LIFT_M = village.presentation.overlay.orders.lift_m;
/** The tour's fixed tick: every framing shows the same battle state. */
const TOUR_TICK = 90;

const text = (page, id) => page.getByTestId(id).innerText();

async function shot(ctx, page, name) {
  await snapshot(ctx, page, `frame-${name}.png`);
}

/** Battle-look slice 16: the road is drawn where the simulation has it. Top
 *  down over the first road's straight run, ground a metre inside its edge
 *  reads as road and ground a metre and a half outside reads as verge. */
async function checkRoadEdges(ctx, page) {
  const road = village.map.roads[0];
  const [[ax, ay], [bx, by]] = road.points;
  const half = road.width_m / 2;
  const [mx, my] = [(ax + bx) / 2, (ay + by) / 2];
  const len = Math.hypot(bx - ax, by - ay);
  const [nx, ny] = [-(by - ay) / len, (bx - ax) / len];
  const at = (off) => [mx + nx * off, my + ny * off];
  await lab(page, (c) => window.__lab.setCamera({ ...window.__lab.camera(), ...c }), {
    target: [mx, my, 0],
    distance: 60,
    pitch: 1.5,
  });
  const shot = decode(await snapshot(ctx, page, "road-edges-1920x1080.png"));
  const greenness = async (off) => {
    const [x, y] = at(off);
    const css = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], 0), [x, y]);
    const [r, g, b] = pixel(shot, css[0], css[1]);
    return g - (r + b) / 2;
  };
  const centre = await greenness(0);
  const inside = [await greenness(half - 1), await greenness(-(half - 1))];
  const outside = [await greenness(half + 1.5), await greenness(-(half + 1.5))];
  ctx.check(
    "the road is drawn where the simulation has it: road inside its edge, verge outside",
    inside.every((g) => g < centre + 6) && outside.every((g) => g > centre + 12),
    JSON.stringify({ centre, inside, outside }),
  );
}

/** Battle-look slice 18: the grass field, read back at a framing. */
const grassClumps = (page) => lab(page, () => window.__lab.grass().clumps());
const grassCounts = (page) => lab(page, () => window.__lab.grass().counts());

/** How far `p` lies outside the road network (negative on a road). */
function offRoad(p) {
  let best = Infinity;
  for (const road of village.map.roads) {
    for (let k = 1; k < road.points.length; k++) {
      const [a, b] = [road.points[k - 1], road.points[k]];
      const ab = [b[0] - a[0], b[1] - a[1]];
      const t = Math.max(
        0,
        Math.min(1, ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / (ab[0] ** 2 + ab[1] ** 2)),
      );
      const off = Math.hypot(p[0] - a[0] - ab[0] * t, p[1] - a[1] - ab[1] * t);
      best = Math.min(best, off - road.width_m / 2);
    }
  }
  return best;
}

const insideRect = (p, [x, y, w, h]) => p[0] > x && p[0] < x + w && p[1] > y && p[1] < y + h;
const underProp = (p) =>
  village.map.props.some(({ center: [cx, cy], yaw, half_extents: [hx, hy] }) => {
    const [dx, dy] = [p[0] - cx, p[1] - cy];
    const [c, s] = [Math.cos(yaw), Math.sin(yaw)];
    return Math.abs(dx * c + dy * s) < hx && Math.abs(-dx * s + dy * c) < hy;
  });

/** The grass field's contract at the tour's framings: seated on the
 *  simulation's triangles, never on roads, props, forests or water, the same
 *  clumps in the same places frame to frame and after a pan, and within its
 *  buffers. */
async function checkGrass(ctx, page, name, minRelief = 0) {
  const clumps = await grassClumps(page);
  const counts = await grassCounts(page);
  const heights = await lab(
    page,
    (roots) => roots.map(([x, y]) => window.__lab.route.surfaceZ(x, y)),
    clumps.map((c) => c.root),
  );
  const seatedWorst = clumps.reduce((m, c, i) => Math.max(m, Math.abs(c.root[2] - heights[i])), 0);
  const misplaced = clumps.filter(
    (c) =>
      offRoad(c.root) < 0 ||
      underProp(c.root) ||
      village.map.forests.some((f) => insideRect(c.root, f.rect)),
  );
  const tiers = [0, 1].map((t) => clumps.filter((c) => c.tier === t).length);
  const relief = clumps.reduce((m, c) => Math.max(m, c.root[2]), 0);
  await ctx.writeEvidence(`grass-${name}.json`, {
    counts,
    tiers,
    seatedWorst,
    relief,
    misplaced: misplaced.length,
    kinds: Object.fromEntries(
      [...new Set(clumps.map((c) => c.kind))].map((k) => [
        k,
        clumps.filter((c) => c.kind === k).length,
      ]),
    ),
  });
  ctx.check(
    `${name}: grass is seated on the simulation's triangles, within a centimetre`,
    clumps.length > 1000 && seatedWorst < 0.01 && relief >= minRelief,
    JSON.stringify({ clumps: clumps.length, seatedWorst, relief }),
  );
  ctx.check(
    `${name}: no grass stands on a road, a building or in a forest`,
    misplaced.length === 0,
    JSON.stringify(misplaced.slice(0, 5)),
  );
  ctx.check(
    `${name}: the field fits its buffers: every clump found is drawn`,
    counts.nearFound === counts.near && counts.farFound === counts.far,
    JSON.stringify(counts),
  );
  return clumps;
}

/** Roots as sorted keys, for set comparisons. */
const rootKeys = (clumps) => clumps.map((c) => c.root.map((v) => v.toFixed(3)).join(",")).sort();

/** The same framing draws the same clumps; a small pan keeps the clumps the
 *  two framings share exactly where they were (no reshuffle), and panning
 *  back draws exactly what was drawn before. */
async function checkGrassResidency(ctx, page) {
  const before = await grassClumps(page);
  await snapshot(ctx, page, "grass-residency-a.png");
  const again = await grassClumps(page);
  const start = await lab(page, () => window.__lab.camera());
  const pan = (dx) =>
    lab(
      page,
      (c) =>
        window.__lab.setCamera({
          ...c.start,
          target: [c.start.target[0] + c.dx, c.start.target[1], c.start.target[2]],
        }),
      { start, dx },
    );
  await pan(1.3);
  await page.evaluate(() => window.__lab.frame());
  const moved = await grassClumps(page);
  await pan(0);
  await page.evaluate(() => window.__lab.frame());
  const back = await grassClumps(page);
  const [a, b, m, z] = [before, again, moved, back].map(rootKeys);
  const same = (x, y) => x.length === y.length && x.every((k, i) => k === y[i]);
  const shared = new Set(a);
  const kept = m.filter((k) => shared.has(k)).length;
  ctx.check(
    "grass residency: the same framing draws the same clumps",
    a.length > 1000 && same(a, b),
    JSON.stringify({ a: a.length, b: b.length }),
  );
  ctx.check(
    "grass residency: a 1.3 m pan keeps most clumps exactly in place, and panning back restores them",
    kept / m.length > 0.9 && same(a, z),
    JSON.stringify({ before: a.length, moved: m.length, kept, back: z.length }),
  );
}

/** GRASS_COST=1: the grass field's GPU cost at each tour framing, paired
 *  on/off in interleaved batches (Metal overlaps passes, so there is no
 *  per-pass split). Run it alone, under the GPU lock. */
async function measureGrassCost(ctx, page) {
  const framings = {
    ground: { distance: CAMERA.zoom_min, pitch: CAMERA.pitch_curve[0][1] },
    default: { distance: CAMERA.default.distance, pitch: 0.85 },
    strategic: { distance: CAMERA.zoom_max, pitch: 0.85 },
  };
  const batch = (off) =>
    lab(
      page,
      async (off) => {
        await window.__lab.suppressGrass(off);
        await window.__lab.setFrameView("final");
        await new Promise((r) => setTimeout(r, 1500));
        return window.__lab.stats().gpu;
      },
      off,
    );
  const median = (v) => [...v].sort((a, b) => a - b)[Math.floor(v.length / 2)];
  const result = { framings: {} };
  await lab(page, () => window.__lab.reset());
  for (const [name, f] of Object.entries(framings)) {
    await lab(page, (f) => window.__lab.setCamera({ ...window.__lab.camera(), ...f }), f);
    const rows = { on: [], off: [] };
    for (let r = 0; r < 6; r++) {
      rows.off.push((await batch(true)).meanMs);
      rows.on.push((await batch(false)).meanMs);
    }
    result.framings[name] = {
      offMs: median(rows.off),
      grassMs: median(rows.on.map((v, i) => v - rows.off[i])),
      counts: await grassCounts(page),
      samples: rows,
    };
  }
  await lab(page, () => window.__lab.suppressGrass(false));
  result.adapter = await page.evaluate(() => window.__lab.adapter);
  await ctx.writeEvidence("grass-cost.json", result);
  ctx.check(
    "kill gate: the grass costs at most 4 ms a frame at ground zoom",
    result.framings.ground.grassMs <= 4,
    JSON.stringify(
      Object.fromEntries(
        Object.entries(result.framings).map(([k, v]) => [k, +v.grassMs.toFixed(3)]),
      ),
    ),
  );
}

/** GLOW_COST=1 (in the orders tour, Space held at the default camera): the
 *  overlays' halo's GPU cost, paired on/off in interleaved batches of 240
 *  forced redraws (the GPU mean's window). Run it alone, under the GPU lock. */
async function measureGlowCost(ctx, page) {
  const batch = (off) =>
    lab(
      page,
      async (off) => {
        await window.__lab.setOverlayGlowStrength(off ? 0 : null);
        for (let k = 0; k < 240; k++) await window.__lab.frame();
        return window.__lab.stats().gpu;
      },
      off,
    );
  const median = (v) => [...v].sort((a, b) => a - b)[Math.floor(v.length / 2)];
  const rows = { on: [], off: [] };
  for (let r = 0; r < 4; r++) {
    rows.off.push((await batch(true)).meanMs);
    rows.on.push((await batch(false)).meanMs);
  }
  await lab(page, () => window.__lab.setOverlayGlowStrength(null));
  const result = {
    offMs: median(rows.off),
    glowMs: median(rows.on.map((v, i) => v - rows.off[i])),
    samples: rows,
    adapter: await page.evaluate(() => window.__lab.adapter),
  };
  await ctx.writeEvidence("glow-cost.json", result);
  ctx.check(
    "the overlay glow's cost is measured",
    Number.isFinite(result.glowMs),
    JSON.stringify(result),
  );
}

/** PAINT_COST=1 (in the orders tour, Space held at the default camera): the
 *  ground paint's marks' GPU cost, paired on/off in interleaved batches of
 *  240 forced redraws. Off still clears the paint target and the ground
 *  still reads it, so this is the marks' draw; the target and the reads are
 *  the fixed part. Run it alone, under the GPU lock. */
async function measurePaintCost(ctx, page) {
  const batch = (off) =>
    lab(
      page,
      async (off) => {
        await window.__lab.suppressPaint(off);
        for (let k = 0; k < 240; k++) await window.__lab.frame();
        return window.__lab.stats().gpu;
      },
      off,
    );
  const median = (v) => [...v].sort((a, b) => a - b)[Math.floor(v.length / 2)];
  const rows = { on: [], off: [] };
  for (let r = 0; r < 4; r++) {
    rows.off.push((await batch(true)).meanMs);
    rows.on.push((await batch(false)).meanMs);
  }
  await lab(page, () => window.__lab.suppressPaint(false));
  const result = {
    offMs: median(rows.off),
    paintMs: median(rows.on.map((v, i) => v - rows.off[i])),
    samples: rows,
  };
  await ctx.writeEvidence("paint-cost.json", result);
  ctx.check(
    "the ground paint's cost is measured",
    Number.isFinite(result.paintMs),
    JSON.stringify(result),
  );
}

/** Near, default and far at 1920×1080: the framings the reference crops judge. */
async function tour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  // Grass kinds are appearances, installed once the catalog loads.
  await page.waitForFunction(() => window.__lab.stats?.().grass.enabled, undefined, {
    timeout: 30000,
  });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, TOUR_TICK - (await lab(page, () => window.__lab.route.tick())));
  const camera = () => lab(page, () => window.__lab.camera());
  const tourShot = (name) => snapshot(ctx, page, `tour-${name}-1920x1080.png`);
  // Wheel over the battlefield, clear of the panel, until the zoom limit.
  const wheel = async (dy) => {
    await page.mouse.move(1300, 540);
    for (let k = 0; k < 20; k++) await page.mouse.wheel(0, dy);
  };
  const [lowest, highest] = [CAMERA.pitch_curve[0], CAMERA.pitch_curve.at(-1)];

  const opening = await camera();
  ctx.check(
    "the battle opens at the fixture's default framing",
    opening.distance === CAMERA.default.distance &&
      opening.target[0] === CAMERA.default.target[0] &&
      opening.target[1] === CAMERA.default.target[1],
    JSON.stringify(opening),
  );
  await tourShot("default");
  await checkGrass(ctx, page, "default");
  await checkGrassResidency(ctx, page);
  // Whatever is overlay (contacts, the x-ray) keeps exactly its own colours
  // over the finished, fogged and graded world. Rings, zone, border and
  // orders are painted in the world since the 27e follow-ups, so this frame
  // may hold no overlay at all (decisions.md, 27e follow-ups).
  const isolation = await checkOverlayIsolation(ctx, page, "overlay-default");
  ctx.check(
    "overlays keep their own colours over the finished frame",
    isolation.isolated,
    JSON.stringify(isolation),
  );

  await wheel(400);
  const far = await camera();
  ctx.check(
    "wheeling out stops at the strategic height, pitched by the curve",
    far.distance === CAMERA.zoom_max &&
      highest[0] === CAMERA.zoom_max &&
      Math.abs(far.pitch - highest[1]) < 1e-9,
    JSON.stringify(far),
  );
  await tourShot("strategic");
  await checkRoadEdges(ctx, page);

  await lab(page, () => window.__lab.reset());
  await wheel(-400);
  const near = await camera();
  ctx.check(
    "wheeling in stops at ground level, pitched by the curve, the target on the ground",
    near.distance === CAMERA.zoom_min &&
      lowest[0] === CAMERA.zoom_min &&
      Math.abs(near.pitch - lowest[1]) < 1e-9 &&
      near.target[2] ===
        (await lab(page, (t) => window.__lab.route.surfaceZ(t[0], t[1]), near.target)),
    JSON.stringify(near),
  );
  await tourShot("ground");
  await checkGrass(ctx, page, "ground");
  // On the ridge's flank, where the triangles tilt and climb metres.
  await lab(
    page,
    (c) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        ...c,
        target: [c.target[0], c.target[1], window.__lab.route.surfaceZ(c.target[0], c.target[1])],
      }),
    { target: [610, 560], distance: 45, pitch: 0.45 },
  );
  await snapshot(ctx, page, "grass-ridge-1920x1080.png");
  await checkGrass(ctx, page, "ridge", 5);
  // The near-to-far traverse: the default framing from the ground out past
  // where grass gives way to the painted ground, pitched by the curve.
  await lab(page, () => window.__lab.reset());
  for (const distance of [25, 40, 65, 110, 180, 300]) {
    await lab(
      page,
      (d) => {
        const curve = d.curve;
        let pitch = curve.at(-1)[1];
        for (let k = 1; k < curve.length; k++)
          if (d.distance <= curve[k][0]) {
            const [[d0, p0], [d1, p1]] = [curve[k - 1], curve[k]];
            pitch = p0 + ((p1 - p0) * (d.distance - d0)) / (d1 - d0);
            break;
          }
        window.__lab.setCamera({ ...window.__lab.camera(), distance: d.distance, pitch });
      },
      { distance, curve: CAMERA.pitch_curve },
    );
    await snapshot(ctx, page, `grass-traverse-${distance}-1920x1080.png`);
  }
  if (process.env.GRASS_COST === "1") await measureGrassCost(ctx, page);
  await page.close();
}

/** The forests' drawn tree lines, at fixed framings (tick 90). */
const TREE_TOUR = {
  // Up the road that runs north through the east forest, WARNO's central road.
  road: { target: [1150, 560], distance: 210, pitch: 0.5, yaw: -Math.PI / 2 },
  // The west forest's south edge, from the road beside it.
  edge: { target: [790, 850], distance: 120, pitch: 0.42, yaw: -Math.PI / 2 },
  // The same edge from the ground, the camera's closest zoom.
  ground: { target: [790, 860], distance: 25, pitch: 0.22, yaw: -Math.PI / 2 },
  // Straight down on the west forest's south-west corner.
  top: { target: [712, 892], distance: 90, pitch: 1.5, yaw: -Math.PI / 2 },
};

async function treeTour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, TOUR_TICK - (await lab(page, () => window.__lab.route.tick())));
  const seen = {};
  for (const [name, framing] of Object.entries(TREE_TOUR)) {
    await lab(
      page,
      (c) => {
        const z = window.__lab.route.surfaceZ(c.target[0], c.target[1]);
        window.__lab.setCamera({ ...window.__lab.camera(), ...c, target: [...c.target, z] });
      },
      framing,
    );
    await snapshot(ctx, page, `trees-${name}-1920x1080.png`);
    seen[name] = await lab(page, () => window.__lab.stats().scenery);
  }

  // Every forest tree is drawn at some tier in every framing (the forest is
  // never culled: it casts shadows into view); scenery past the map is drawn
  // up the road, and culled straight down.
  const drawn = (p) => p.tiers.reduce((a, b) => a + b, 0);
  ctx.check(
    "the forests and the scenery past the map are drawn as trees, tiered by distance",
    Object.values(seen).every(
      (s) => s.forest.placed > 400 && drawn(s.forest) === s.forest.placed,
    ) &&
      seen.road.backdrop.placed > 2000 &&
      drawn(seen.road.backdrop) > 0 &&
      drawn(seen.top.backdrop) < drawn(seen.road.backdrop) &&
      seen.ground.forest.tiers[0] > 0,
    JSON.stringify(seen),
  );
  // The world only: the HUD's full-width bars (slice 27e) cover the frame's
  // top and bottom edges.
  const hud = await page.addStyleTag({
    content: "[data-testid=battle-panel], .ro-layer { display: none !important; }",
  });
  const top = decode(await snapshot(ctx, page, "trees-top-check-1920x1080.png"));
  await hud.evaluate((e) => e.remove());
  const [x0, y0] = village.map.forests[0].rect;
  const luminance = async (x, y) => {
    const css = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), [
      x,
      y,
      await lab(page, (q) => window.__lab.route.surfaceZ(q[0], q[1]), [x, y]),
    ]);
    const [r, g, b] = pixel(top, css[0], css[1]);
    return 0.3 * r + 0.5 * g + 0.2 * b;
  };
  const inside = (await luminance(x0 + 22, y0 + 22)) + (await luminance(x0 + 30, y0 + 14));
  // The west sample stands in the field, 14 m out from the wood's west edge.
  const outside = (await luminance(x0 - 14, y0 + 40)) + (await luminance(x0 + 22, y0 - 14));
  ctx.check(
    "straight down, the forest's crowns read darker than the field beside it",
    inside < outside,
    JSON.stringify({ inside, outside }),
  );

  // Rebuilding the frame returns every allocation, the scenery's included
  // (after one rebuild, so the per-tier buffers have this view's capacity).
  await lab(page, () => window.__lab.rebuild());
  const baseline = await lab(page, () => window.__lab.allocations());
  for (let i = 0; i < 2; i++) await lab(page, () => window.__lab.rebuild());
  const after = await lab(page, () => window.__lab.allocations());
  ctx.check(
    "rebuilding the frame returns live GPU buffers and textures, trees included, to baseline",
    after.buffers === baseline.buffers &&
      after.bufferBytes === baseline.bufferBytes &&
      after.textures === baseline.textures,
    `baseline ${JSON.stringify(baseline)} after ${JSON.stringify(after)}`,
  );
  await page.close();
}

/** A squad's framings: the camera's closest zoom, the Defilade default, and
 *  far enough that soldiers are impostor cards. */
const SQUAD_FRAMINGS = {
  ground: { distance: 25, pitch: 0.22 },
  default: { distance: 65, pitch: 0.85 },
  far: { distance: 420, pitch: 0.85 },
};

/** Frame `framing` on world point `at`. */
const frameOn = (page, at, framing) =>
  lab(
    page,
    ([p, f]) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        ...f,
        yaw: -Math.PI / 2,
        target: [p[0], p[1], window.__lab.route.surfaceZ(p[0], p[1])],
      }),
    [at, framing],
  );

async function soldierTour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, TOUR_TICK - (await lab(page, () => window.__lab.route.tick())));
  const o = await obs(page);
  const squads = o.own.filter((u) => u.members.length > 0);
  const soldiers = squads.reduce((n, u) => n + u.members.length, 0);
  // The squad nearest the opening framing's target.
  const [tx, ty] = CAMERA.default.target;
  const squad = squads.reduce((a, b) =>
    Math.hypot(a.position[0] - tx, a.position[1] - ty) <=
    Math.hypot(b.position[0] - tx, b.position[1] - ty)
      ? a
      : b,
  );
  const seen = {};
  for (const [name, framing] of Object.entries(SQUAD_FRAMINGS)) {
    await frameOn(page, squad.position, framing);
    await snapshot(ctx, page, `soldiers-${name}-1920x1080.png`);
    seen[name] = await lab(page, () => window.__lab.stats().models);
  }
  const meshes = (s) => s.tiers.reduce((a, b) => a + b, 0);
  // Soldiers' own tiers (vehicles and props share the layer since slice 24).
  const finest = (s) => s.bodyTiers.findIndex((n) => n > 0);
  ctx.check(
    "every drawn soldier is a posed model: finer tiers near, impostor cards far, only meshes posed",
    // Slice 24: vehicles and props are models too, so soldiers are counted
    // as the posed bodies, their cards and the bodies culled.
    Object.values(seen).every(
      (s) =>
        s.skinned <= meshes(s) &&
        s.atlasLayers >= 6 &&
        s.skinned + s.cards + s.culledBodies === soldiers,
    ) &&
      finest(seen.ground) <= 1 &&
      finest(seen.default) >= finest(seen.ground) &&
      seen.far.cards === soldiers,
    JSON.stringify({ soldiers, seen }),
  );

  // Units are drawn by identification: never fogged. In the fog mask a
  // soldier is white wherever the camera sees him, his back to every eye included.
  await frameOn(page, squad.position, SQUAD_FRAMINGS.ground);
  await lab(page, () => window.__lab.setFrameView("fog-mask"));
  const mask = decode(await snapshot(ctx, page, "soldiers-fog-mask-1920x1080.png"));
  await lab(page, () => window.__lab.setFrameView("final"));
  let dark = 0;
  let sampled = 0;
  for (const m of squad.members) {
    const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1.1), m);
    if (!at || at[0] < 0 || at[1] < 0 || at[0] >= 1920 || at[1] >= 1080) continue;
    for (let dy = -4; dy <= 4; dy += 2)
      for (let dx = -3; dx <= 3; dx++) {
        sampled++;
        if (pixel(mask, at[0] + dx, at[1] + dy)[0] < 128) dark++;
      }
  }
  ctx.check(
    "soldiers are never fogged: the fog mask is white across every soldier in view",
    sampled > 50 && dark === 0,
    JSON.stringify({ sampled, dark }),
  );

  // Picking keeps the simulation's boxes: a soldier's torso picks his body's.
  await frameOn(page, squad.position, SQUAD_FRAMINGS.default);
  await page.evaluate(() => window.__lab.frame());
  const m = squad.members[0];
  const torso = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1), m);
  const picked = await lab(page, (p) => window.__lab.pickAt(p[0], p[1]), torso);
  const box = await lab(page, (k) => window.__lab.instances()[k], picked);
  ctx.check(
    "clicking a soldier picks his box",
    picked >= 0 &&
      box.half[0] === village.physics.soldier_radius_m &&
      Math.hypot(box.x - m[0], box.y - m[1]) < 1.5,
    JSON.stringify({ picked, box, member: m }),
  );

  // Contact: every blue unit attack-moves on the village, and the fallen lie
  // as static corpses.
  await lab(
    page,
    (o) =>
      window.__lab.route.command({
        kind: "attack_move",
        units: o.own.map((u) => u.id),
        gesture: 1,
        goal: [1000, 800],
      }),
    o,
  );
  // The Defilade framing on the squad nearest a tank (the reference crop's
  // infantry beside armour), with and without the HUD's marks.
  const besideTank = async (name) => {
    const o = await obs(page);
    const tanks = o.own.filter((u) => u.kind === "tank");
    const near = (u) =>
      Math.min(
        ...tanks.map((t) =>
          Math.hypot(t.position[0] - u.position[0], t.position[1] - u.position[1]),
        ),
      );
    const squad = o.own.filter((u) => u.members.length > 0).sort((a, b) => near(a) - near(b))[0];
    if (!squad) return;
    await frameOn(page, squad.position, SQUAD_FRAMINGS.default);
    await snapshot(ctx, page, `soldiers-${name}-1920x1080.png`);
    await lab(page, () => window.__lab.setFrameView("world"));
    await snapshot(ctx, page, `soldiers-${name}-world-1920x1080.png`);
    await lab(page, () => window.__lab.setFrameView("final"));
  };
  // Eight seconds into the advance, then in contact.
  await advance(page, 240);
  await besideTank("advance");
  const fight = await until(
    page,
    (o) => o.corpses.length >= 2 && o.own.some((u) => u.members.length > 0),
    30 * 240,
    30,
  );
  if (fight) await besideTank("contact");
  // Two seconds on, the first deaths have played out.
  if (fight) await advance(page, 60);
  const after = await obs(page);
  const fallen = after.corpses[0];
  if (fallen) {
    await frameOn(page, fallen.position, { distance: 30, pitch: 0.6 });
    await snapshot(ctx, page, "soldiers-fallen-1920x1080.png");
  }
  // The presentation clock runs on wall time: under load the death may still
  // be playing, so draw frames until the fallen lie static (or give up).
  if (fight)
    await page
      .waitForFunction(
        () => (window.__lab.frame(), window.__lab.stats().models.corpses >= 1),
        undefined,
        {
          timeout: 10_000,
          polling: 100,
        },
      )
      .catch(() => {});
  const lying = await lab(page, () => window.__lab.stats().models);
  ctx.check(
    "the fallen lie as static corpses, drawn and never posed",
    !!fight &&
      lying.corpses >= 1 &&
      lying.corpses <= after.corpses.length &&
      lying.instances > lying.skinned,
    JSON.stringify({ tick: after.tick, fallen: after.corpses.length, lying }),
  );
  await page.close();
}

const isVehicle = (u) => ["tank", "supply", "jeep"].includes(u.kind);
const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));

/** Frame a world point from `distance` metres at `pitch`, looking along `yaw`. */
const frameAt = (page, at, distance, pitch, yaw) =>
  lab(
    page,
    (c) => {
      const z = window.__lab.route.surfaceZ(c.at[0], c.at[1]);
      window.__lab.setCamera({
        ...window.__lab.camera(),
        ...c.view,
        target: [c.at[0], c.at[1], z],
      });
    },
    { at, view: { distance, pitch, yaw } },
  );

/** Slice 24: vehicles, buildings and wrecks are appearances placed, fitted and
 *  articulated from what the side knows. */
async function vehicleTour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, TOUR_TICK - (await lab(page, () => window.__lab.route.tick())));
  await lab(page, () => window.__lab.frame());
  let o = await obs(page);
  let posed = await lab(page, () => window.__lab.route.vehicles());
  const stats = await lab(page, () => window.__lab.stats());
  const own = o.own.filter(isVehicle);
  const enemy = o.identified.filter(isVehicle);
  ctx.check(
    "every own and identified vehicle is drawn as its appearance, and nothing as a proxy",
    posed.length === own.length + enemy.length &&
      posed.every(
        (v) => v.articulation && ["tank", "supply_truck", "jeep"].includes(v.appearance),
      ) &&
      stats.instances === 0,
    JSON.stringify({
      posed: posed.length,
      own: own.length,
      enemy: enemy.length,
      proxies: stats.instances,
    }),
  );
  // Each own tank's turret is on its cannon's published bearing, relative to the hull.
  const turrets = own
    .filter((u) => u.kind === "tank")
    .map((u) => {
      const v = posed.find(
        (p) => Math.hypot(p.position[0] - u.position[0], p.position[1] - u.position[1]) < 1e-3,
      );
      const gun = u.weaponPoses.find((w) => w.mount === 0);
      return v && gun
        ? Math.abs(wrap(v.articulation.turret_yaw - (gun.bearing - u.yaw)))
        : Infinity;
    });
  ctx.check(
    "each tank's turret follows its cannon's published bearing",
    turrets.length > 0 && turrets.every((d) => d < 1e-3),
    JSON.stringify(turrets),
  );
  // Slice 37: the fences and sandbags are drawn apart too (bodies a vehicle
  // can shove), so the houses are the structures standing on a house's box.
  const houses = village.map.props.filter((p) => p.kind === "building");
  const structures = (await lab(page, () => window.__lab.route.structures())).filter((s) =>
    houses.some((h) => s.position[0] === h.center[0] && s.position[1] === h.center[1]),
  );
  ctx.check(
    "the village's houses stand as their appearances, each fitted to its box",
    structures.length === houses.length &&
      structures.every(
        (s, i) =>
          s.state === "intact" &&
          s.position[0] === houses[i].center[0] &&
          s.position[1] === houses[i].center[1] &&
          s.scale.every((k) => Math.abs(k - 1) < 1e-6),
      ),
    JSON.stringify(structures),
  );
  const tank = own.find((u) => u.kind === "tank");
  // The nearest tank from its right front, as WARNO frames its nearest tank.
  await frameAt(page, tank.position, 20, 0.42, tank.yaw - Math.PI * 0.3);
  await snapshot(ctx, page, "vehicles-tank-1920x1080.png");
  await frameAt(page, tank.position, 65, 0.85, -1.57);
  await snapshot(ctx, page, "vehicles-default-1920x1080.png");
  // The village's farms from the western approach.
  await frameAt(page, [1000, 812], 150, 0.55, 0);
  await snapshot(ctx, page, "vehicles-village-1920x1080.png");

  // Drive and fire: the tanks attack-move on the village; run on until an own
  // tank's cannon fires, then look at it on the tick of the shot.
  await lab(
    page,
    (units) =>
      window.__lab.route.command({ kind: "attack_move", units, gesture: 1, goal: [900, 800] }),
    own.filter((u) => u.kind === "tank").map((u) => u.id),
  );
  const shots = (f) =>
    Object.fromEntries(
      f.own.filter((u) => u.kind === "tank").map((u) => [u.id, u.weaponPoses[0]?.shots ?? 0]),
    );
  const start = shots(o);
  o = await until(
    page,
    (f) =>
      f.own.some((u) => u.kind === "tank" && (u.weaponPoses[0]?.shots ?? 0) > (start[u.id] ?? 0)),
    30 * 240,
    1,
  );
  const shooter = o?.own.find(
    (u) => u.kind === "tank" && (u.weaponPoses[0]?.shots ?? 0) > (start[u.id] ?? 0),
  );
  if (shooter) {
    await frameAt(page, shooter.position, 26, 0.35, shooter.weaponPoses[0].bearing + Math.PI * 0.6);
    await lab(page, () => window.__lab.frame());
    posed = await lab(page, () => window.__lab.route.vehicles());
    const fired = posed.find(
      (p) =>
        Math.hypot(p.position[0] - shooter.position[0], p.position[1] - shooter.position[1]) < 0.5,
    );
    await snapshot(ctx, page, "vehicles-fire-1920x1080.png");
    ctx.check(
      "a tank's shot recoils its gun on the tick it fires",
      !!fired && fired.articulation.recoil > 0,
      JSON.stringify({ tick: o.tick, fired }),
    );
  } else ctx.check("a tank's shot recoils its gun on the tick it fires", false, "no tank fired");
  await page.close();
}

/** Slice 25: the firefight's combat effects. The battle at a fixed tick, the
 *  first burst after it framed at its moment and as it grows, and every
 *  effect drawn from what the side's publications carried. */
async function effectTour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, 30 - (await lab(page, () => window.__lab.route.tick())));
  await lab(page, () => {
    const o = window.__lab.route.observation();
    window.__lab.route.command({
      kind: "attack_move",
      units: o.own.map((u) => u.id),
      gesture: 1,
      goal: [1000, 800],
    });
  });
  await advance(page, 600 - (await lab(page, () => window.__lab.route.tick())));
  // The first burst in the open: one under a wood's canopy is hidden by the crowns.
  const open = (b) => !village.map.forests.some((f) => insideRect(b.point, f.rect));
  const o = await until(page, (f) => f.blasts.some(open), 30 * 60, 1);
  if (!o) {
    ctx.check("a burst in the firefight is drawn as a fireball", false, "no blast by tick 2400");
    await page.close();
    return;
  }
  const burst = o.blasts.find(open).point;
  await frameAt(page, burst, 70, 0.55, -1.2);
  const frames = {};
  for (const [age, step] of [
    [0, 0],
    [3, 3],
    [8, 5],
  ]) {
    await advance(page, step);
    await snapshot(ctx, page, `effects-burst-${age}-1920x1080.png`);
    await lab(page, () => window.__lab.setFrameView("world"));
    frames[age] = decode(await snapshot(ctx, page, `effects-burst-${age}-world-1920x1080.png`));
    await lab(page, () => window.__lab.setFrameView("final"));
  }
  const [bx, by] = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 4), burst);
  let fire = 0;
  for (let dy = -60; dy <= 60; dy += 2)
    for (let dx = -60; dx <= 60; dx += 2) {
      const [r, g, b] = pixel(frames[3], Math.round(bx + dx), Math.round(by + dy));
      if (r > 200 && g > 90 && b < 0.8 * g) fire++;
    }
  const stats = await lab(page, () => ({
    frame: window.__lab.stats().effects,
    effects: window.__lab.route.effects(),
  }));
  ctx.check(
    "a burst in the firefight is drawn as a fireball where it was published",
    fire > 40 && stats.frame.instances > 0 && stats.effects.dropped === 0,
    JSON.stringify({ tick: o.tick, burst, firePixels: fire, ...stats }),
  );

  // Slice 40: the live battle is heard once the player first clicks, and a
  // pause silences its transients while its loops hold.
  await page.click('[data-testid="battle-panel"] strong');
  await lab(page, () => window.__lab.route.resume());
  // The sound bank is synthesised after the gesture; under load that takes
  // seconds, so wait for the state rather than a fixed time.
  const heard = () => {
    const s = window.__lab.route.sound();
    return !!s?.running && s.started > 0 && s.loops > 0;
  };
  await page.waitForFunction(heard, undefined, { timeout: 30000 }).catch(() => {});
  const live = await lab(page, () => window.__lab.route.sound());
  await lab(page, () => window.__lab.route.pause());
  const silenced = () => {
    const s = window.__lab.route.sound();
    return !!s?.held && s.transients === 0;
  };
  await page.waitForFunction(silenced, undefined, { timeout: 10000 }).catch(() => {});
  const held = await lab(page, () => window.__lab.route.sound());
  ctx.check(
    "the battle is heard after the first click; a pause silences its transients",
    !!live?.running &&
      live.started > 0 &&
      live.loops > 0 &&
      !!held?.held &&
      held.transients === 0 &&
      held.loops > 0,
    JSON.stringify({ live, held }),
  );
  await page.close();
}

/** Pixels whose colour differs by more than 16 levels inside a box. */
function changedIn(a, b, [x0, y0, x1, y1]) {
  let n = 0;
  for (let y = Math.max(0, y0); y < Math.min(a.height, y1); y += 2)
    for (let x = Math.max(0, x0); x < Math.min(a.width, x1); x += 2) {
      const p = pixel(a, x, y);
      const q = pixel(b, x, y);
      if (Math.abs(p[0] - q[0]) + Math.abs(p[1] - q[1]) + Math.abs(p[2] - q[2]) > 16) n++;
    }
  return n;
}

/** Slice 26: the aftermath. The first wreck blue learns burns and smokes
 *  over it, paused and playing; a moving tank kicks dust behind it; pause
 *  holds the smoke and reset clears it. `SMOKE_GIF=1` also writes the
 *  frames of a burning wreck and of a tank's dust, tick by tick. */
async function smokeTour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, 30 - (await lab(page, () => window.__lab.route.tick())));
  await lab(page, () => {
    const o = window.__lab.route.observation();
    window.__lab.route.command({
      kind: "attack_move",
      units: o.own.filter((u) => u.kind !== "tank").map((u) => u.id),
      gesture: 1,
      goal: [1000, 800],
    });
    // Tanks drive on (a plain move, still firing at will): an attack-move
    // halts them at the first target, and the wood's AT is one from the start.
    window.__lab.route.command({
      kind: "move",
      units: o.own.filter((u) => u.kind === "tank").map((u) => u.id),
      gesture: 2,
      goal: [1000, 800],
      route: "shortest",
    });
  });
  // A tank under way, framed from behind its shoulder, kicking up dust.
  const before = await obs(page);
  await advance(page, 90);
  const moving = await obs(page);
  const tank = moving.own.find((u) => {
    const was = before.own.find((b) => b.id === u.id);
    return (
      u.kind === "tank" &&
      was &&
      Math.hypot(u.position[0] - was.position[0], u.position[1] - was.position[1]) > 3
    );
  });
  if (tank) {
    await frameAt(page, tank.position, 32, 0.4, tank.yaw + Math.PI * 0.75);
    if (process.env.SMOKE_GIF === "1")
      for (let k = 0; k < 90; k++) {
        await advance(page, 1);
        const t = (await obs(page)).own.find((u) => u.id === tank.id);
        await frameAt(page, t.position, 32, 0.4, tank.yaw + Math.PI * 0.75);
        await snapshot(ctx, page, `gif-dust-${String(k).padStart(3, "0")}.png`);
      }
    const now = (await obs(page)).own.find((u) => u.id === tank.id);
    await frameAt(page, now.position, 32, 0.4, tank.yaw + Math.PI * 0.75);
    const shotPng = decode(await snapshot(ctx, page, "smoke-dust-1920x1080.png"));
    await lab(page, () => window.__lab.suppressEffects(true));
    const bare = decode(await snapshot(ctx, page, "smoke-dust-bare-1920x1080.png"));
    await lab(page, () => window.__lab.suppressEffects(false));
    // Behind the tank: the side of it away from where it heads.
    const back = [
      now.position[0] - Math.cos(now.yaw) * 6,
      now.position[1] - Math.sin(now.yaw) * 6,
      now.position[2] + 1,
    ];
    const [bx, by] = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), back);
    const dust = changedIn(shotPng, bare, [bx - 90, by - 90, bx + 90, by + 90]);
    ctx.check(
      "a moving tank kicks up dust behind it",
      dust > 200,
      JSON.stringify({ tick: moving.tick, tank: tank.id, dustPixels: dust }),
    );
  } else ctx.check("a moving tank kicks up dust behind it", false, "no tank under way");

  const o = await until(
    page,
    (f) => f.knownProps.some((p) => p.kind.endsWith("_wreck")),
    30 * 150,
    15,
  );
  if (!o) {
    ctx.check("a known wreck burns and smokes", false, "no wreck by tick 4600");
    await page.close();
    return;
  }
  const wreck = o.knownProps.find((p) => p.kind.endsWith("_wreck"));
  const at = [wreck.center[0], wreck.center[1], wreck.baseZ];
  const view = [70, 0.5, -1.2];
  const shots = {};
  for (const [age, step] of [
    [2, 60],
    [10, 240],
  ]) {
    await advance(page, step);
    await frameAt(page, at, ...view);
    shots[age] = decode(await snapshot(ctx, page, `smoke-wreck-${age}s-1920x1080.png`));
    await lab(page, () => window.__lab.setFrameView("world"));
    await snapshot(ctx, page, `smoke-wreck-${age}s-world-1920x1080.png`);
    await lab(page, () => window.__lab.setFrameView("final"));
  }
  if (process.env.SMOKE_GIF === "1")
    for (let k = 0; k < 90; k++) {
      await advance(page, 1);
      await snapshot(ctx, page, `gif-wreck-${String(k).padStart(3, "0")}.png`);
    }
  await lab(page, () => window.__lab.suppressEffects(true));
  const bare = decode(await snapshot(ctx, page, "smoke-wreck-bare-1920x1080.png"));
  await lab(page, () => window.__lab.suppressEffects(false));
  const [wx, wy] = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 8), at);
  const column = changedIn(shots[10], bare, [wx - 200, wy - 300, wx + 200, wy + 150]);
  const stats = await lab(page, () => window.__lab.route.effects());
  ctx.check(
    "a known wreck burns and smokes over it",
    column > 1500 && stats.sources > 0 && stats.dropped === 0,
    JSON.stringify({ tick: o.tick, wreck, columnPixels: column, ...stats }),
  );

  // Paused, the smoke holds still: the same frame twice.
  const held = decode(await snapshot(ctx, page, "smoke-paused-a-1920x1080.png"));
  await page.waitForTimeout(500);
  const again = decode(await snapshot(ctx, page, "smoke-paused-b-1920x1080.png"));
  const drift = changedIn(held, again, [wx - 200, wy - 300, wx + 200, wy + 150]);
  ctx.check("paused, the smoke holds still", drift === 0, JSON.stringify({ drift }));

  // Playing, it rises on.
  await lab(page, () => window.__lab.route.resume());
  await page.waitForTimeout(1500);
  await lab(page, () => window.__lab.route.pause());
  await frameAt(page, at, ...view);
  const playing = decode(await snapshot(ctx, page, "smoke-playing-1920x1080.png"));
  const rose = changedIn(held, playing, [wx - 200, wy - 300, wx + 200, wy + 150]);
  ctx.check("playing, the smoke moves on", rose > 200, JSON.stringify({ rose }));

  // Reset: a new battle, no smoke left from the old one.
  await lab(page, () => window.__lab.route.reset());
  await page.waitForFunction(
    () => window.__lab.route.tick() > 3 && window.__lab.route.tick() < 60,
    undefined,
    {
      timeout: 30000,
    },
  );
  await lab(page, () => window.__lab.frame());
  const cleared = await lab(page, () => ({
    effects: window.__lab.route.effects(),
    frame: window.__lab.stats().effects,
  }));
  ctx.check(
    "reset clears every effect",
    cleared.effects.sources === 0 && cleared.effects.live === 0,
    JSON.stringify(cleared),
  );
  await page.close();
}

/** Pixels of the overlays-on-black capture the overlay lights, and how many
 *  are cover-icon yellow or green (`light`, `medium`/`heavy`). */
/** The overlay's ink over black, by hue: the orders' yellow, and the cover
 *  ramp's white (light), mint (medium) and green (heavy). */
function overlayInk(png) {
  let lit = 0,
    yellow = 0,
    white = 0,
    mint = 0,
    green = 0;
  for (let i = 0; i < png.data.length; i += 4) {
    const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
    if (r + g + b < 60) continue;
    lit++;
    if (r > b + 50 && g > b + 30) yellow++;
    else if (r > 170 && g > 170 && b > 170 && Math.max(r, g, b) - Math.min(r, g, b) < 30) white++;
    else if (g > r + 70 && g > b + 60) green++;
    else if (g > r + 25 && b > r + 5 && g > b + 15) mint++;
  }
  return { lit, yellow, white, mint, green };
}

/** Slice 35: Total War markers (D2), the Space overlay (D2+), right-drag
 *  facing (Q9) and a reverse move's marker (Q31), own units only. */
async function orderTour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  let o = await obs(page);
  const rifle = o.own.find((u) => u.kind === "rifle");
  const tank = o.own.find((u) => u.kind === "tank");
  const surface = (p) => lab(page, (q) => window.__lab.route.surfaceZ(q[0], q[1]), p);
  const toCss = async (p) =>
    lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), [...p, await surface(p)]);
  const toMark = async (p) =>
    lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), [
      ...p,
      (await surface(p)) + ORDER_MARK_LIFT_M,
    ]);

  // A real right-drag: press at the goal, release north-east of it.
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id]);
  await page.waitForFunction((id) => window.__lab.route.selected()[0] === id, rifle.id);
  const goal = [rifle.position[0] + 30, rifle.position[1]];
  const at = [goal[0] - 12, goal[1]];
  await frameAt(page, at, CAMERA.default.distance, 0.85, CAMERA.default.yaw);
  await lab(page, () => window.__lab.frame());
  const press = await toCss(goal);
  const release = await toCss([goal[0] + 10, goal[1] + 10]);
  await page.mouse.move(press[0], press[1]);
  await page.mouse.down({ button: "right" });
  await page.mouse.move(release[0], release[1], { steps: 6 });
  await page.mouse.up({ button: "right" });
  o = await until(page, (x) => x.own.find((u) => u.id === rifle.id)?.goal, 60, 1);
  const moving = o?.own.find((u) => u.id === rifle.id);
  const wanted = Math.PI / 4;
  ctx.check(
    "a right-drag sends a move whose final marker faces the drag",
    !!moving && Math.abs(wrap(moving.finalFacing - wanted)) < 0.25,
    JSON.stringify({ facing: moving?.finalFacing, wanted }),
  );
  ctx.check(
    "each soldier's published spot is where his final marker stands",
    !!moving &&
      moving.memberOrders.length === moving.members.length &&
      moving.memberOrders.every((m) => Math.hypot(m.spot[0] - goal[0], m.spot[1] - goal[1]) < 25),
    JSON.stringify(moving?.memberOrders),
  );

  // A tank reversing to a point behind it: held facing and the reverse marker.
  const behind = [
    tank.position[0] - Math.cos(tank.yaw) * 25,
    tank.position[1] - Math.sin(tank.yaw) * 25,
  ];
  await lab(
    page,
    (c) =>
      window.__lab.route.command({
        kind: "move",
        units: [c.id],
        gesture: 3501,
        goal: c.behind,
        route: "shortest",
        direction: "reverse",
      }),
    { id: tank.id, behind },
  );
  o = await until(page, (x) => x.own.find((u) => u.id === tank.id)?.route.length > 0, 60, 1);
  const backing = o?.own.find((u) => u.id === tank.id);
  ctx.check(
    "a reverse move's final marker keeps the hull's facing",
    !!backing &&
      backing.direction === "reverse" &&
      Math.abs(wrap(backing.finalFacing - tank.yaw)) < 0.2,
    JSON.stringify({ facing: backing?.finalFacing, yaw: tank.yaw, direction: backing?.direction }),
  );
  await advance(page, 45);
  await lab(page, () => window.__lab.route.select([]));
  o = await obs(page);

  // Nothing of the enemy's plan is published: seen enemies carry no order.
  const leaks = o.identified.flatMap((e) =>
    ["goal", "route", "queue", "memberOrders", "finalFacing", "direction", "area"].filter(
      (k) => k in e,
    ),
  );
  ctx.check("no enemy destination, route or facing is published", leaks.length === 0, `${leaks}`);

  // Space held: every own unit's markers, routes and cover icons.
  const cameras = {
    default: { distance: CAMERA.default.distance, pitch: 0.85 },
    ground: { distance: CAMERA.zoom_min, pitch: CAMERA.pitch_curve[0][1] },
    strategic: { distance: 1100, pitch: 0.85 },
  };
  // The order marks are overlay (`yellow-orders`): read them over black,
  // the callouts (DOM) hidden.
  const ordersOnly = async (name) => {
    await page.evaluate(() => {
      document.querySelector("[data-testid=readouts]").style.visibility = "hidden";
    });
    await lab(page, () => window.__lab.setFrameView("overlays-on-black"));
    const png = decode(await snapshot(ctx, page, `${name}-overlay.png`));
    await lab(page, () => window.__lab.setFrameView("final"));
    await page.evaluate(() => {
      document.querySelector("[data-testid=readouts]").style.visibility = "";
    });
    return png;
  };
  const without = overlayInk(await ordersOnly("orders-default-nospace"));
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders());
  await lab(page, () => window.__lab.frame());
  for (const [name, view] of Object.entries(cameras)) {
    await frameAt(page, at, view.distance, view.pitch, CAMERA.default.yaw);
    // Lines keep their width on screen: the overlay is rebuilt for the new
    // zoom a frame after the camera moves.
    await lab(page, () => window.__lab.frame());
    await lab(page, () => window.__lab.frame());
    await snapshot(ctx, page, `orders-space-${name}-1920x1080.png`);
  }
  // Into the fog: the other squad sent to ground blue cannot see.
  const other = o.own.find((u) => u.kind === "rifle" && u.id !== rifle.id);
  const fogged = await lab(
    page,
    (from) => {
      const f = window.__lab.route.observation().fog;
      const seen = (x, y) => {
        const i = Math.floor(x / f.cellM),
          j = Math.floor(y / f.cellM);
        if (i < 0 || j < 0 || i >= f.nx || j >= f.ny) return true;
        const k = j * f.nx + i;
        return (f.bits[k >> 5] & (1 << (k & 31))) !== 0;
      };
      for (let r = 40; r <= 700; r += 20)
        for (let a = 0; a < 16; a++) {
          const x = from[0] + Math.cos((a / 16) * 2 * Math.PI) * r,
            y = from[1] + Math.sin((a / 16) * 2 * Math.PI) * r;
          if (!seen(x, y) && !seen(x + 8, y) && !seen(x, y + 8) && !seen(x - 8, y - 8))
            return [x, y];
        }
      return null;
    },
    other?.position,
  );
  if (other && fogged) {
    await lab(
      page,
      (c) =>
        window.__lab.route.command({
          kind: "move",
          units: [c.id],
          gesture: 3502,
          goal: c.goal,
          route: "shortest",
          facing: 0,
        }),
      { id: other.id, goal: fogged },
    );
    await advance(page, 3);
    await frameAt(
      page,
      fogged,
      cameras.default.distance,
      cameras.default.pitch,
      CAMERA.default.yaw,
    );
    await snapshot(ctx, page, "orders-space-fog-1920x1080.png");
  }
  ctx.check("a squad's markers are framed in the fog", !!fogged, JSON.stringify(fogged));
  await frameAt(page, at, cameras.default.distance, cameras.default.pitch, CAMERA.default.yaw);
  const inked = await ordersOnly("orders-default-space");
  const withSpace = overlayInk(inked);
  o = await obs(page);
  const tiers = new Set(
    o.own.flatMap((u) => u.memberOrders.flatMap((m) => [m.coverNow, m.coverThere])),
  );
  ctx.check(
    "holding Space draws every own unit's markers",
    withSpace.lit - without.lit > 5000,
    JSON.stringify({ without, withSpace }),
  );
  ctx.check(
    "cover icons appear in their tiers' colours",
    // The cover ramp (27e follow-ups): light white, medium mint, heavy green.
    (!tiers.has("light") || withSpace.white > without.white) &&
      (!tiers.has("medium") || withSpace.mint > without.mint) &&
      (!tiers.has("heavy") || withSpace.green > without.green),
    JSON.stringify({ tiers: [...tiers], withSpace }),
  );
  // The routes Space draws are the published ones: the middle of each
  // unit's first leg in view is inked.
  const inView = [];
  for (const u of o.own.filter((u) => u.route.length)) {
    const [x, y] = u.route[0];
    const p = await toMark([(u.position[0] + x) / 2, (u.position[1] + y) / 2]);
    if (p && p[0] > 4 && p[1] > 4 && p[0] < 1916 && p[1] < 1076) inView.push(p);
  }
  const inkedAt = (p) => {
    for (let dy = -3; dy <= 3; dy++)
      for (let dx = -3; dx <= 3; dx++) {
        const i = ((Math.round(p[1]) + dy) * inked.width + Math.round(p[0]) + dx) * 4;
        if (inked.data[i] + inked.data[i + 1] + inked.data[i + 2] >= 30) return true;
      }
    return false;
  };
  ctx.check(
    "with Space, each published route in view is drawn",
    inView.length > 0 && inView.every(inkedAt),
    JSON.stringify(inView),
  );
  // Slice 27: a route lies over the grass, solid: along the middle of each
  // first leg in view, the ribbon's centre line is ink at every pixel (the
  // blades once poked through it as dark speckle). Squads only: a vehicle's
  // route stops short of its ring.
  // The HUD's bars (slice 27e: a top bar and a bottom command bar).
  const panels = await page.evaluate(() =>
    [...document.querySelectorAll("[data-occludes-readouts]")].map((e) =>
      e.getBoundingClientRect().toJSON(),
    ),
  );
  const underPanel = (p) =>
    panels.some(
      (panel) =>
        p[0] >= panel.left - 2 &&
        p[0] <= panel.right + 2 &&
        p[1] >= panel.top - 2 &&
        p[1] <= panel.bottom + 2,
    );
  // Each sample across the line against the line's own ink around it: a
  // blade over the line dims it before it blackens it. The line is about 2 px
  // wide (slice 27e), so a sample's ink is the sum of the five pixels across
  // it, which holds wherever its centre falls between pixel rows; and the
  // ink it is held to is the median of its neighbours along the line (six
  // each way), since 4× MSAA's quarter-sample steps at its edges move whole
  // drape segments by up to 15%, which a blade's fleck does not.
  const inkAt = (x, y) => {
    const i = (Math.round(y) * inked.width + Math.round(x)) * 4;
    return inked.data[i] + inked.data[i + 1] + inked.data[i + 2];
  };
  const along = [];
  for (const u of o.own.filter((u) => u.route.length && u.members.length)) {
    const [x, y] = u.route[0];
    const length = Math.hypot(x - u.position[0], y - u.position[1]);
    const a = await toMark([u.position[0], u.position[1]]);
    const b = await toMark([x, y]);
    if (!a || !b) continue;
    const n = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
    const across = [-(b[1] - a[1]) / n, (b[0] - a[0]) / n];
    // The route runs from the edge of the circle the squad stands in to the
    // edge of its area ring (slice 27e; both drawn at the fixture's scale):
    // sample only the line between them.
    const scale = village.presentation.overlay.orders.area_draw_scale;
    const here = Math.max(
      ...u.members.map((m) => Math.hypot(m[0] - u.position[0], m[1] - u.position[1])),
    );
    // Past the circle round the soldiers and its arrowhead, where the route
    // leaves along the facing (27e follow-ups).
    const rim = (here + 0.85) * scale;
    const first = Math.max(0.05, (rim + Math.min(0.6 * rim, 1.5) + 0.5) / length);
    const area = u.route.length === 1 && u.area ? u.area.radius * scale + 1 : 0;
    const last = Math.min(0.95, (length - area) / length);
    for (let s = first; s <= last; s += 0.1 / Math.max(1, length)) {
      const q = [u.position[0] + (x - u.position[0]) * s, u.position[1] + (y - u.position[1]) * s];
      const p = await toMark(q);
      if (!p || p[0] < 4 || p[1] < 4 || p[0] > 1916 || p[1] > 1076 || underPanel(p)) continue;
      along.push({
        unit: u.id,
        p: p.map(Math.round),
        ink: [-2, -1, 0, 1, 2].reduce(
          (sum, k) => sum + inkAt(p[0] + across[0] * k, p[1] + across[1] * k),
          0,
        ),
      });
    }
  }
  const median = [...along].sort((a, b) => a.ink - b.ink)[along.length >> 1]?.ink ?? 0;
  // Only where a sample has its six neighbours each way on its own route: a
  // one-sided window at a route's end can straddle an MSAA step unevenly.
  const holes = along.filter((a, k) => {
    const around = along.slice(k - 6, k + 7);
    if (k < 6 || around.length < 13 || around.some((b) => b.unit !== a.unit)) return false;
    const inks = around.map((b) => b.ink).sort((x, y) => x - y);
    // Painted in the lit world (27e follow-ups), a route takes the soldiers'
    // shadows, which dim it by up to about a sixth; a blade over it hides it
    // (decisions.md, 27e follow-ups).
    return a.ink < 0.75 * inks[6];
  });
  const samples = along.length;
  ctx.check(
    "a route is drawn solid over the grass, never speckled by its blades",
    samples > 50 && holes.length / samples < 0.01,
    JSON.stringify({
      samples,
      median,
      holes: holes.length,
      at: holes.slice(0, 8),
      from: along[0]?.p,
      to: along.at(-1)?.p,
    }),
  );
  if (process.env.GLOW_COST === "1") await measureGlowCost(ctx, page);
  if (process.env.PAINT_COST === "1") await measurePaintCost(ctx, page);
  const isolation = await checkOverlayIsolation(ctx, page, "orders-space");
  ctx.check(
    "the Space overlay composites after post, untouched by fog and grade",
    isolation.isolated,
    JSON.stringify(isolation),
  );
  await page.keyboard.up("Space");
  await page.waitForFunction(() => !window.__lab.route.showOrders());
  ctx.check("releasing Space hides the overlay again", true);
  // The selection's callouts, names and orders without Space (evidence).
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id, tank.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 2);
  await frameAt(page, at, cameras.default.distance, cameras.default.pitch, CAMERA.default.yaw);
  await lab(page, () => window.__lab.frame());
  await snapshot(ctx, page, "orders-selected-default-1920x1080.png");
  await checkSelectionYellow(ctx, page, rifle.id, tank.id);
  await checkRimJoin(ctx, page, rifle.id);
  await checkPaintedLight(ctx, page, tank.id);
  // The top bar's scenario picker, open (evidence for the chrome).
  await page.getByRole("button", { name: "Scenario" }).click();
  await snapshot(ctx, page, "orders-scenario-picker-1920x1080.png");
  await page.keyboard.press("Escape");
  if (process.env.GLOW_SHEET === "1") await glowSheet(ctx, page, rifle.id, tank.id);
  if (process.env.MARKS_SHEET)
    await marksSheet(ctx, page, [rifle.id, tank.id, other?.id], fogged, process.env.MARKS_SHEET);
  await page.close();
}

/** The selection is yellow (27e): a selected squad's circle where it stands
 *  (27e follow-ups) and a selected vehicle's marker, on the overlay alone,
 *  while the squad's destination area ring stays the order colour. */
async function checkSelectionYellow(ctx, page, squadId, vehicleId) {
  const o = await obs(page);
  const squad = o.own.find((u) => u.id === squadId);
  const vehicle = o.own.find((u) => u.id === vehicleId);
  const { area_draw_scale: scale, vehicle_marker_margin_m: margin } =
    village.presentation.overlay.orders;
  const vehicleR = village.physics[`${vehicle.kind}_half_extents_m`][0] + margin;
  // The ground marks alone: the callouts (DOM, over the canvas) hidden.
  const readouts = (shown) =>
    page.evaluate((v) => {
      document.querySelector("[data-testid=readouts]").style.visibility = v ? "" : "hidden";
    }, shown);
  // The paint (orders) as its rise over the ground, and the overlay (the
  // selection's own circles, after tone mapping) over black.
  await readouts(false);
  const paint = await paintOnly(ctx, page, "orders-selected");
  await lab(page, () => window.__lab.setFrameView("overlays-on-black"));
  const overlay = decode(await snapshot(ctx, page, "orders-selected-overlay.png"));
  await lab(page, () => window.__lab.setFrameView("final"));
  await readouts(true);
  const nearIn = (png) => (css, test) => {
    for (let dy = -2; dy <= 2; dy++)
      for (let dx = -2; dx <= 2; dx++) {
        const [x, y] = [Math.round(css[0]) + dx, Math.round(css[1]) + dy];
        if (x < 0 || y < 0 || x >= png.width || y >= png.height) continue;
        const i = (y * png.width + x) * 4;
        if (test(png.data[i], png.data[i + 1], png.data[i + 2])) return true;
      }
    return false;
  };
  const near = nearIn(paint);
  const nearOverlay = nearIn(overlay);
  // `yellow-orders`: the orders' true yellow over black, and the
  // selection's amber as its paint's warm rise (red well over green).
  const yellow = (r, g, b) => r > 150 && g > 0.8 * r && b < 0.6 * r;
  const warm = (r, g, b) => r > b + 25 && r > g * 1.1;
  // The selection's amber, were it on the overlay (it is paint): red well
  // over green, unlike the orders' yellow.
  const amberOver = (r, g, b) => r > 150 && g < 0.75 * r && b < 0.5 * r;
  const inked = (r, g, b) => r + g + b > 90;
  // The selection's overlay is drawn at the orders' height over the ground.
  const ORDER_LIFT_M = village.presentation.overlay.orders.lift_m;
  // Samples round a circle: how many are inked, and how many of those in
  // the selection's colour; on the overlay (the selection) or the paint.
  const circle = async (c, radius, onOverlay = true) => {
    const out = { inked: 0, yellow: 0 };
    const look = onOverlay ? nearOverlay : near;
    for (let k = 0; k < 32; k++) {
      const a = (k / 32) * 2 * Math.PI;
      const q = [c[0] + Math.cos(a) * radius, c[1] + Math.sin(a) * radius];
      const lift = onOverlay ? ORDER_LIFT_M : MARK_LIFT_M;
      const z = (await lab(page, (w) => window.__lab.route.surfaceZ(w[0], w[1]), q)) + lift;
      const p = await lab(page, (w) => window.__lab.projectToCss(w[0], w[1], w[2]), [...q, z]);
      if (!p || !look(p, inked)) continue;
      out.inked++;
      if (look(p, onOverlay ? amberOver : warm)) out.yellow++;
    }
    return out;
  };
  const here = Math.max(
    ...squad.members.map((m) => Math.hypot(m[0] - squad.position[0], m[1] - squad.position[1])),
  );
  const standing = await circle(squad.position, (here + 0.45 + 0.4) * scale, false);
  // The circle's arrowhead, on its rim at the squad's current facing.
  const radius = (here + 0.45 + 0.4) * scale;
  const tipAt = [
    squad.position[0] + Math.cos(squad.yaw) * (radius + 1),
    squad.position[1] + Math.sin(squad.yaw) * (radius + 1),
  ];
  const tipCss = await lab(
    page,
    (w) => window.__lab.projectToCss(w[0], w[1], window.__lab.route.surfaceZ(w[0], w[1]) + w[2]),
    [...tipAt, MARK_LIFT_M],
  );
  const facing = !!tipCss && near(tipCss, warm);
  // No route runs inside a unit's own circle: it leaves from the rim. The
  // orders' yellow on the overlay, sampled on a grid well inside each circle
  // (the selected soldiers' markers are amber paint, not overlay).
  const order = yellow;
  const routeInside = async (c, r) => {
    let hits = 0;
    for (let dy = -0.75; dy <= 0.75; dy += 0.125)
      for (let dx = -0.75; dx <= 0.75; dx += 0.125) {
        if (Math.hypot(dx, dy) > 0.75) continue;
        const q = [c[0] + dx * r, c[1] + dy * r];
        const p = await lab(
          page,
          (w) =>
            window.__lab.projectToCss(w[0], w[1], window.__lab.route.surfaceZ(w[0], w[1]) + w[2]),
          [...q, ORDER_MARK_LIFT_M],
        );
        if (p && nearOverlay(p, order)) hits++;
      }
    return hits;
  };
  // Nor over its arrowhead: along the unit's facing, from the rim to near
  // the tip (the head is 0.6 of the radius, at most 1.5 m).
  const routeOnArrow = async (u, r) => {
    const head = Math.min(0.6 * r, 1.5);
    let hits = 0;
    for (let t = 0.15; t <= 0.8; t += 0.13) {
      const q = [
        u.position[0] + Math.cos(u.yaw) * (r + head * t),
        u.position[1] + Math.sin(u.yaw) * (r + head * t),
      ];
      const p = await lab(
        page,
        (w) =>
          window.__lab.projectToCss(w[0], w[1], window.__lab.route.surfaceZ(w[0], w[1]) + w[2]),
        [...q, ORDER_MARK_LIFT_M],
      );
      if (p && nearOverlay(p, order)) hits++;
    }
    return hits;
  };
  const inside = {
    squad: await routeInside(squad.position, radius),
    vehicle: await routeInside(vehicle.position, vehicleR),
    squadArrow: await routeOnArrow(squad, radius),
    vehicleArrow: await routeOnArrow(vehicle, vehicleR),
  };
  ctx.check(
    "no route is drawn inside a unit's own circle marker or over its arrowhead, squad or vehicle",
    Object.values(inside).every((n) => n === 0),
    JSON.stringify(inside),
  );
  const area =
    squad.goal && squad.area
      ? await circle(squad.area.anchor, squad.area.radius * scale, true)
      : null;
  const marker = await circle(vehicle.position, vehicleR, false);
  ctx.check(
    "the selection is in its warm colour: a selected squad's circle (its arrowhead at its facing) and a vehicle's marker; the squad's area ring is not",
    !!squad.goal &&
      standing.inked >= 8 &&
      facing &&
      standing.yellow >= standing.inked * 0.6 &&
      marker.yellow >= 8 &&
      !!area &&
      area.inked >= 8 &&
      area.yellow <= area.inked * 0.15,
    JSON.stringify({ standing, facing, marker, area, moving: !!squad.goal }),
  );
}

/** Paint and overlay marks lie at one height (27e follow-ups): a squad's
 *  route joins its circle exactly where the circle ends, along the route's
 *  first leg on screen. Selected, the circle is amber paint and the route
 *  yellow overlay: the circle's last pixel and the route's first meet within
 *  a pixel. Unselected, both are overlay: the line from the circle into the
 *  route has no gap. */
async function checkRimJoin(ctx, page, squadId) {
  const hideReadouts = (hidden) =>
    page.evaluate((h) => {
      document.querySelector("[data-testid=readouts]").style.visibility = h ? "hidden" : "";
    }, hidden);
  const overlayShot = async (name) => {
    await hideReadouts(true);
    await lab(page, () => window.__lab.setFrameView("overlays-on-black"));
    const png = decode(await snapshot(ctx, page, name));
    await lab(page, () => window.__lab.setFrameView("final"));
    await hideReadouts(false);
    return png;
  };
  const squad = (await obs(page)).own.find((u) => u.id === squadId);
  if (!squad?.route.length) {
    ctx.check("a squad's route joins its circle at the rim", false, "no route");
    return;
  }
  const css = (q) =>
    lab(
      page,
      (w) => window.__lab.projectToCss(w[0], w[1], window.__lab.route.surfaceZ(w[0], w[1])),
      q,
    );
  const [from, to] = [await css(squad.position), await css(squad.route[0])];
  const d = Math.hypot(to[0] - from[0], to[1] - from[1]);
  const at = (png, t) => {
    const [x, y] = [
      Math.round(from[0] + ((to[0] - from[0]) * t) / d),
      Math.round(from[1] + ((to[1] - from[1]) * t) / d),
    ];
    const i = (y * png.width + x) * 4;
    return [png.data[i], png.data[i + 1], png.data[i + 2]];
  };
  const yellow = ([r, g, b]) => r > 150 && g > 0.8 * r && b < 0.6 * r;
  // Selected: the circle (amber paint) outward, then the route (overlay).
  await lab(page, (id) => window.__lab.route.select([id]), squadId);
  const paint = await paintOnly(ctx, page, "rim-join-selected");
  const over = await overlayShot("rim-join-selected-overlay.png");
  const firstRoute = (() => {
    for (let t = 0; t < d; t += 0.5) if (yellow(at(over, t))) return t;
    return null;
  })();
  const circleEnd = (() => {
    let last = null;
    // Any of the amber's ink, antialiased too: the arrowhead ends in a point
    // whose last pixels are faint.
    const faint = ([r, g, b]) => r > b + 10 && r > g * 1.05;
    for (let t = 0; t < (firstRoute ?? d); t += 0.5) if (faint(at(paint, t))) last = t;
    return last;
  })();
  const selected = {
    circleEnd,
    firstRoute,
    gap: firstRoute !== null && circleEnd !== null ? firstRoute - circleEnd : null,
  };
  // Unselected (Space held, so its orders show): circle and route both
  // overlay, one run of yellow.
  await lab(page, () => window.__lab.route.select([]));
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders());
  const plain = await overlayShot("rim-join-unselected-overlay.png");
  await page.keyboard.up("Space");
  // The run of yellow that holds the route just past where it began when
  // selected: it must reach back over the circle's end, unbroken.
  let start = null;
  const ref = (firstRoute ?? 0) + 10;
  // Any of the yellow's ink, antialiased too: the arrowhead meets the route
  // at its tip, a point.
  const ink = ([r, g, b]) => r > 40 && g > 0.7 * r && b < 0.6 * r;
  if (firstRoute !== null && ink(at(plain, ref))) {
    start = ref;
    for (let t = ref; t >= 0 && ink(at(plain, t)); t -= 0.5) start = t;
  }
  const reachesCircle = start !== null && circleEnd !== null && start <= circleEnd - 1;
  await lab(page, (id) => window.__lab.route.select([id]), squadId);
  ctx.check(
    "a squad's route joins its circle at the rim: selected (paint circle, overlay route) within a pixel; unselected with no gap",
    selected.gap !== null && Math.abs(selected.gap) <= 1.5 && reachesCircle,
    JSON.stringify({ selected, unselected: { runFrom: start } }),
  );
}

/** Painted ground marks sit under the effects (27e follow-ups): on a moving
 *  tank's marker ring at the ground camera, its dust, drawn over the ring,
 *  hides part of it (some of the ring shows less ink than with the effects
 *  held off). The readouts scene checks that they take cast shadows. */
async function checkPaintedLight(ctx, page, vehicleId) {
  // The tank's marker as paint: the selection's amber (`yellow-orders`: the
  // orders themselves are overlay, over the dust by design).
  const selectedBefore = await lab(page, () => window.__lab.route.selected());
  await lab(page, (id) => window.__lab.route.select([id]), vehicleId);
  const vehicle = (await obs(page)).own.find((u) => u.id === vehicleId);
  await frameAt(
    page,
    vehicle.position,
    CAMERA.zoom_min,
    CAMERA.pitch_curve[0][1],
    CAMERA.default.yaw,
  );
  await lab(page, () => window.__lab.frame());
  const r =
    village.physics[`${vehicle.kind}_half_extents_m`][0] +
    village.presentation.overlay.orders.vehicle_marker_margin_m;
  const at = [];
  for (let k = 0; k < 64; k++) {
    const a = (k / 64) * 2 * Math.PI;
    const q = [vehicle.position[0] + Math.cos(a) * r, vehicle.position[1] + Math.sin(a) * r];
    const z = (await lab(page, (w) => window.__lab.route.surfaceZ(w[0], w[1]), q)) + MARK_LIFT_M;
    const p = await lab(page, (w) => window.__lab.projectToCss(w[0], w[1], w[2]), [...q, z]);
    if (p) at.push(p.map(Math.round));
  }
  // Per sample: the paint's rise (ink), the painted pixel's light, and the
  // ground's light under it, each the most round the sample.
  const read = (png) =>
    at.map(([x, y]) => {
      let ink = 0,
        mark = 0,
        ground = 0;
      for (let dy = -2; dy <= 2; dy++)
        for (let dx = -2; dx <= 2; dx++) {
          const i = ((y + dy) * png.width + x + dx) * 4;
          const sum = png.data[i] + png.data[i + 1] + png.data[i + 2];
          const u = png.under.data;
          const under = u[i] + u[i + 1] + u[i + 2];
          ground = Math.max(ground, under);
          if (sum > ink) [ink, mark] = [sum, sum + under];
        }
      return { ink, mark, ground };
    });
  const fxPaint = await paintOnly(ctx, page, "paint-light");
  // The tank's silhouette: where the frame (without paint) changes when the
  // models are held off, inside its hull's projected box.
  await lab(page, async () => {
    await window.__lab.suppressPaint(true);
    await window.__lab.suppressModels(true);
  });
  const bare = decode(await snapshot(ctx, page, "paint-light-no-models.png"));
  await lab(page, async () => {
    await window.__lab.suppressModels(false);
    await window.__lab.suppressPaint(false);
  });
  const body = (i) =>
    Math.abs(fxPaint.under.data[i] - bare.data[i]) +
      Math.abs(fxPaint.under.data[i + 1] - bare.data[i + 1]) +
      Math.abs(fxPaint.under.data[i + 2] - bare.data[i + 2]) >
    120;
  const withFx = read(fxPaint);
  // Nothing is painted on the tank: no paint inside its hull's projected
  // box (shrunk 3 px from its edge), from its own ring or the area ring
  // behind it. Ground paint lies only on the ground layers.
  const [hx, hy, hz] = village.physics[`${vehicle.kind}_half_extents_m`];
  const gz = await lab(page, (w) => window.__lab.route.surfaceZ(w[0], w[1]), vehicle.position);
  const [c, s] = [Math.cos(vehicle.yaw), Math.sin(vehicle.yaw)];
  const corners = [];
  for (const sx of [-1, 1])
    for (const sy of [-1, 1])
      for (const z of [gz, gz + 2 * hz]) {
        const w = [
          vehicle.position[0] + c * sx * hx - s * sy * hy,
          vehicle.position[1] + s * sx * hx + c * sy * hy,
          z,
        ];
        const p = await lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), w);
        if (p) corners.push(p);
      }
  // The corners' convex hull (monotone chain), shrunk toward its centre.
  const pts = [...corners].sort((m, n) => m[0] - n[0] || m[1] - n[1]);
  const cross = (o, a, b) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const half = (list) => {
    const h = [];
    for (const p of list) {
      while (h.length >= 2 && cross(h[h.length - 2], h[h.length - 1], p) <= 0) h.pop();
      h.push(p);
    }
    return h.slice(0, -1);
  };
  const hull = [...half(pts), ...half([...pts].reverse())];
  const mid = hull.reduce((m, p) => [m[0] + p[0] / hull.length, m[1] + p[1] / hull.length], [0, 0]);
  const shrunk = hull.map((p) => {
    const d = Math.hypot(p[0] - mid[0], p[1] - mid[1]) || 1;
    return [p[0] - ((p[0] - mid[0]) / d) * 3, p[1] - ((p[1] - mid[1]) / d) * 3];
  });
  const inside = (x, y) =>
    shrunk.every((p, k) => cross(p, shrunk[(k + 1) % shrunk.length], [x, y]) >= 0);
  const xs = shrunk.map((p) => p[0]),
    ys = shrunk.map((p) => p[1]);
  let onTank = 0,
    area = 0;
  const hits = [];
  for (let y = Math.floor(Math.min(...ys)); y <= Math.ceil(Math.max(...ys)); y++)
    for (let x = Math.floor(Math.min(...xs)); x <= Math.ceil(Math.max(...xs)); x++) {
      if (x < 0 || y < 0 || x >= fxPaint.width || y >= fxPaint.height || !inside(x, y)) continue;
      const i = (y * fxPaint.width + x) * 4;
      // The body well inside its edge: body 3 px round too (not its shadow's
      // or an MSAA edge's change).
      const at = (dx, dy) => body(((y + dy) * fxPaint.width + x + dx) * 4);
      if (
        ![
          [0, 0],
          [3, 0],
          [-3, 0],
          [0, 3],
          [0, -3],
        ].every(([dx, dy]) => at(dx, dy))
      )
        continue;
      area++;
      if (fxPaint.data[i] + fxPaint.data[i + 1] + fxPaint.data[i + 2] > 90) {
        onTank++;
        if (hits.length < 12 && onTank % 40 === 1) hits.push([x, y]);
      }
    }
  ctx.check(
    "no mark is painted on the tank at the ground camera: none inside its silhouette",
    area > 2000 && onTank === 0,
    JSON.stringify({ area, onTank, hits, hull: shrunk.map((p) => p.map(Math.round)) }),
  );
  await lab(page, () => window.__lab.suppressEffects(true));
  const noFx = read(await paintOnly(ctx, page, "paint-light-nofx"));
  await lab(page, () => window.__lab.suppressEffects(false));
  // The most any sample loses to the effects over it.
  const hidden = Math.max(...noFx.map((s, k) => (s.ink > 45 ? 1 - withFx[k].ink / s.ink : 0)));
  ctx.check(
    "the effects draw over painted marks: dust hides part of a moving tank's marker",
    hidden > 0.07,
    JSON.stringify({ hidden }),
  );
  await lab(page, (ids) => window.__lab.route.select(ids), selectedBefore);
}

/** MARKS_SHEET=<tag>: the ground marks' look in fixed frames, for a
 *  side-by-side between two builds at the same battle state: a selected
 *  squad and a reversing tank (its dust over its marker, its hull's shadow on
 *  it) at the default and ground cameras, and the third squad's destination
 *  in fog. Each frame is labelled with MARKS_LABEL. Writes
 *  `marks-<tag>-<frame>.png`. */
async function marksSheet(ctx, page, ids, fogged, tag) {
  // The squad and the tank selected; the third squad's orders unselected.
  const select = (u) => lab(page, (v) => window.__lab.route.select(v), u);
  await select([ids[0], ids[1]]);
  const o = await obs(page);
  const pair = o.own.filter((u) => u.id === ids[0] || u.id === ids[1]);
  const mid = [
    pair.reduce((s, u) => s + u.position[0], 0) / pair.length,
    pair.reduce((s, u) => s + u.position[1], 0) / pair.length,
  ];
  const tank = o.own.find((u) => u.id === ids[1]);
  const frames = {
    default: [mid, CAMERA.default.distance, 0.85],
    ground: [tank.position, CAMERA.zoom_min, CAMERA.pitch_curve[0][1]],
    space: [mid, CAMERA.default.distance, 0.85],
    ...(fogged ? { fog: [fogged, CAMERA.default.distance, 0.85] } : {}),
  };
  const label = process.env.MARKS_LABEL ?? tag;
  for (const [name, [at, distance, pitch]] of Object.entries(frames)) {
    // Space held: every own unit's marks; the fog frame: the third squad's.
    if (name === "space") {
      await page.keyboard.down("Space");
      await page.waitForFunction(() => window.__lab.route.showOrders());
    } else if (name === "fog") {
      await page.keyboard.up("Space");
      await select(ids.filter((id) => id !== undefined));
    }
    await frameAt(page, at, distance, pitch, CAMERA.default.yaw);
    await page.evaluate((text) => {
      let tag = document.getElementById("marks-sheet-tag");
      if (!tag) {
        tag = document.createElement("div");
        tag.id = "marks-sheet-tag";
        tag.style.cssText =
          "position:fixed;z-index:99;left:50%;top:50%;transform:translate(-630px,-350px);" +
          "font:700 30px ui-monospace,monospace;color:#fff;background:rgb(0 0 0 / 0.65);padding:6px 14px";
        document.body.append(tag);
      }
      tag.textContent = text;
    }, `${label} · ${name}`);
    await lab(page, () => window.__lab.frame());
    await lab(page, () => window.__lab.frame());
    await snapshot(ctx, page, `marks-${tag}-${name}.png`);
  }
  await page.evaluate(() => document.getElementById("marks-sheet-tag")?.remove());
}

/** GLOW_SHEET=1: the ground marks' glow options (27e follow-ups), a selected
 *  squad and a moving tank at the default camera, one tile each; the
 *  callouts keep their glow in every tile. Writes `glow-sheet-<k>.png` and
 *  where to crop them (`glow-sheet.json`). */
async function glowSheet(ctx, page, squadId, vehicleId) {
  const o = await obs(page);
  const units = o.own.filter((u) => u.id === squadId || u.id === vehicleId);
  const mid = [
    units.reduce((s, u) => s + u.position[0], 0) / units.length,
    units.reduce((s, u) => s + u.position[1], 0) / units.length,
  ];
  await frameAt(page, mid, CAMERA.default.distance, 0.85, CAMERA.default.yaw);
  await lab(page, () => window.__lab.frame());
  const today = 2.2;
  const options = [
    ["A", "no ground glow", 0],
    ["B", "faint ground glow (25%)", today * 0.25],
    ["C", "half ground glow (50%)", today * 0.5],
    ["D", "today's glow (reference)", today],
  ];
  for (const [k, words, strength] of options) {
    await lab(page, (s) => window.__lab.setOverlayGlowStrength(s), strength);
    await page.evaluate((text) => {
      let tag = document.getElementById("glow-sheet-tag");
      if (!tag) {
        tag = document.createElement("div");
        tag.id = "glow-sheet-tag";
        tag.style.cssText =
          "position:fixed;z-index:99;left:50%;top:50%;transform:translate(-470px,-260px);" +
          "font:700 22px ui-monospace,monospace;color:#fff;background:rgb(0 0 0 / 0.6);padding:4px 10px";
        document.body.append(tag);
      }
      tag.textContent = text;
    }, `${k} · ${words}`);
    await snapshot(ctx, page, `glow-sheet-${k}.png`);
  }
  await page.evaluate(() => document.getElementById("glow-sheet-tag")?.remove());
  await lab(page, () => window.__lab.setOverlayGlowStrength(null));
  const centre = await lab(page, (q) => window.__lab.projectToCss(q[0], q[1], 0), mid);
  await ctx.writeEvidence("glow-sheet.json", { centre, options });
}

/** Slice 37: the village's field works (teeth, sandbags, fences)
 *  at the opening framing's distance, pitch and yaw, and every one drawn
 *  standing on the ground. */
const WORKS = {
  "road-block": [895, 790],
  "north-house": [950, 743],
  square: [1012, 796],
  "garden-fence": [945, 900],
  "wood-edge": [640, 890],
  "blue-start": [110, 775],
};

async function worksTour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  const { distance, yaw } = village.presentation.camera.default;
  for (const [name, at] of Object.entries(WORKS)) {
    await frameAt(page, at, distance, 0.85, yaw);
    await snapshot(ctx, page, `works-${name}-1920x1080.png`);
  }
  const apart = await lab(page, () =>
    window.__lab.route
      .structures()
      .map((s) => ({ ...s, ground: window.__lab.route.surfaceZ(s.position[0], s.position[1]) })),
  );
  // Modular kinds draw one model per module along the box, so a body is
  // drawn when a model of its appearance stands inside its footprint.
  const look = { tooth: "dragon_tooth", fence: "fence", sandbags: "sandbags" };
  const works = village.map.props.filter((p) => look[p.kind]);
  const drawn = works.map((p) =>
    apart.some(
      (s) =>
        s.appearance === look[p.kind] &&
        Math.hypot(s.position[0] - p.center[0], s.position[1] - p.center[1]) <=
          Math.hypot(p.half_extents[0], p.half_extents[1]),
    ),
  );
  const off = apart.filter((s) => Math.abs(s.position[2] - s.ground) > 0.05);
  ctx.check(
    "every tooth, fence panel and sandbag section is drawn as its appearance, on the ground",
    works.length > 0 && drawn.every(Boolean) && off.length === 0,
    JSON.stringify({
      works: works.length,
      missing: works.filter((_, i) => !drawn[i]).map((p) => [p.kind, ...p.center]),
      off: off.map((s) => [s.appearance, ...s.position, s.ground]),
    }),
  );
  await page.close();
}

/** Slice 27 (muzzle flash): a flash's core projects this close to the drawn
 *  muzzle, in CSS px at the default camera. */
const MUZZLE_PX = 3;
/** The flash's own light, added over the frame without effects, within
 *  `FLASH_BOX_PX` of the drawn muzzle: at least this bright (0..255). */
const FLASH_BOX_PX = 6;
const FLASH_LIGHT_MIN = 60;
/** Slice 27 (per-mount muzzles): a hull's round starts this close to its
 *  flash, in metres. The drawn muzzle leads or trails the simulation's by
 *  the gun's pitch and recoil and a tick's drive; the phantom muzzle this
 *  replaced was metres off for an HMG turned from the cannon. */
const TRACER_M = 1;

/** Slice 27 (muzzle flash): every flash sits on the muzzle as the model draws
 *  it, not where the simulation starts the round: a tank's cannon (recoiling)
 *  and its cupola HMG, driving and standing, the jeep's HMG and a rifleman's
 *  rifle. One tank drives down the road while the rest attack-move; at each
 *  kind's first shot the default camera frames the shooter, and the flash's
 *  core and the drawn muzzle socket (posed from the model instance, apart
 *  from the flashes' own muzzles) must project within `MUZZLE_PX`, with the
 *  flash's light on screen there. */
async function muzzleTour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, 30 - (await lab(page, () => window.__lab.route.tick())));
  await lab(page, () => {
    const o = window.__lab.route.observation();
    const driver = o.own.find((u) => u.kind === "tank");
    window.__lab.route.command({
      kind: "move",
      units: [driver.id],
      gesture: 1,
      goal: [driver.position[0] + 80, driver.position[1]],
      route: "shortest",
    });
    window.__lab.route.command({
      kind: "attack_move",
      units: o.own.filter((u) => u.id !== driver.id).map((u) => u.id),
      gesture: 2,
      goal: [1000, 800],
    });
  });
  const wanted = new Set([
    "tank cannon, driving",
    "tank cannon, standing",
    "tank HMG, driving",
    "jeep HMG",
    "rifle",
  ]);
  const found = {};
  let prev = await obs(page);
  for (let t = 0; t < 30 * 120 && wanted.size > 0; t++) {
    await advance(page, 1);
    let o = await obs(page);
    let shot = null;
    for (const u of o.own) {
      const was = prev.own.find((q) => q.id === u.id);
      if (!was || shot) continue;
      const driving = Math.hypot(u.position[0] - was.position[0], u.position[1] - was.position[1]);
      for (const w of u.weaponPoses) {
        const before = was.weaponPoses.find((x) => x.mount === w.mount)?.shots ?? w.shots;
        if (w.shots <= before) continue;
        const name =
          u.kind === "tank"
            ? `tank ${w.mount === 0 ? "cannon" : "HMG"}, ${driving > 0.01 ? "driving" : "standing"}`
            : u.kind === "jeep"
              ? "jeep HMG"
              : u.kind === "rifle" && w.mount === 0
                ? "rifle"
                : null;
        if (wanted.has(name)) shot = { name, unit: u, mount: w.mount };
      }
    }
    prev = o;
    if (!shot) continue;
    const { name, unit } = shot;
    // The shooter's drawn muzzle: a hull's mount's node, or the rifleman
    // whose new round starts next tick (the simulation flies a round from
    // the tick after it fires).
    let near = unit.position;
    let socket = shot.mount === 0 && unit.kind === "tank" ? "muzzle" : "hmg_muzzle";
    if (name === "rifle") {
      await advance(page, 1);
      o = prev = await obs(page);
      const squad = o.own.find((u) => u.id === unit.id);
      const round = o.projectiles.find(
        (s) =>
          squad.memberIds.includes(s.shooterMember) &&
          squad.members.some((m) => Math.hypot(m[0] - s.path[0][0], m[1] - s.path[0][1]) < 2),
      );
      if (!round) continue;
      near = round.path[0];
      socket = "muzzle";
    }
    wanted.delete(name);
    await frameAt(page, near, CAMERA.default.distance, 0.85, CAMERA.default.yaw);
    const slug = name.replace(/[^a-z]+/gi, "-").toLowerCase();
    const lit = decode(await snapshot(ctx, page, `muzzle-${slug}-1920x1080.png`));
    const drawn = await lab(page, () => ({
      sockets: window.__lab.route.muzzleSockets(),
      glows: window.__lab.route.effectInstances().filter((e) => e.shape === 1 && e.rays === 0),
    }));
    await lab(page, () => window.__lab.suppressEffects(true));
    const bare = decode(await snapshot(ctx, page, `muzzle-${slug}-noeffects-1920x1080.png`));
    await lab(page, () => window.__lab.suppressEffects(false));
    const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1], (a[2] ?? 0) - (b[2] ?? 0));
    const nearest = (list, p) => list.reduce((a, b) => (dist(b.at, p) < dist(a.at, p) ? b : a));
    const candidates = drawn.sockets.filter((s) => s.name === socket);
    const muzzle = candidates.length ? nearest(candidates, near).at : null;
    const glow = muzzle && drawn.glows.length ? nearest(drawn.glows, muzzle).at : null;
    const px = (p) => p && lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), p);
    const [muzzlePx, glowPx] = [await px(muzzle), await px(glow)];
    let light = 0;
    if (muzzlePx)
      for (let dy = -FLASH_BOX_PX; dy <= FLASH_BOX_PX; dy++)
        for (let dx = -FLASH_BOX_PX; dx <= FLASH_BOX_PX; dx++) {
          const [x, y] = [muzzlePx[0] + dx, muzzlePx[1] + dy];
          if (x < 0 || y < 0 || x >= lit.width || y >= lit.height) continue;
          const [a, b] = [pixel(lit, x, y), pixel(bare, x, y)];
          light = Math.max(light, (a[0] + a[1] + a[2] - b[0] - b[1] - b[2]) / 3);
        }
    const off =
      muzzlePx && glowPx ? Math.hypot(muzzlePx[0] - glowPx[0], muzzlePx[1] - glowPx[1]) : null;
    if (muzzlePx)
      await writeCrop(
        lit,
        ctx.evidencePath(`muzzle-${slug}-crop.png`),
        muzzlePx[0],
        muzzlePx[1],
        60,
        30,
        6,
      );
    found[name] = { tick: o.tick, off, light: Math.round(light), muzzle, glow };
    ctx.check(
      `a ${name} flash sits on the drawn muzzle`,
      off !== null && off <= MUZZLE_PX && light >= FLASH_LIGHT_MIN,
      JSON.stringify(found[name]),
    );
    // A hull's round flies first on the next tick, from the simulation's
    // muzzle for that mount: its tracer starts where the flash is.
    if (name === "rifle" || !muzzle) continue;
    const endKey = (p) => p.join(",");
    const flying = new Set(
      o.projectiles.filter((s) => s.hit === "none").map((s) => endKey(s.path.at(-1))),
    );
    await advance(page, 1);
    prev = await obs(page);
    const hmg = name.includes("HMG");
    const starts = prev.projectiles.filter(
      (s) =>
        s.own &&
        s.shooterMember === null &&
        !flying.has(endKey(s.path[0])) &&
        (hmg ? s.kind === "hmg" : s.kind.startsWith("tank_")),
    );
    const tracer = starts.length
      ? starts.map((s) => s.path[0]).reduce((a, b) => (dist(b, muzzle) < dist(a, muzzle) ? b : a))
      : null;
    const apart = tracer && dist(tracer, muzzle);
    ctx.check(
      `a ${name} tracer starts at its flash`,
      apart !== null && apart <= TRACER_M,
      JSON.stringify({ muzzle, tracer, apart }),
    );
  }
  ctx.check(
    "every kind of shot fired in the battle: tank cannon and HMG, driving and standing, jeep HMG, rifle",
    wanted.size === 0,
    JSON.stringify({ missing: [...wanted], found: Object.keys(found) }),
  );
  await page.close();
}

/** The tours, by name: `VILLAGE_TOURS=effects,smoke` runs only those. */
const TOURS = {
  orders: orderTour,
  muzzle: muzzleTour,
  works: worksTour,
  camera: tour,
  trees: treeTour,
  soldiers: soldierTour,
  vehicles: vehicleTour,
  effects: effectTour,
  smoke: smokeTour,
  woods: woodsTour,
  cleanup: cleanupTour,
};

export async function run(ctx) {
  const only = process.env.VILLAGE_TOURS?.split(",");
  if (only) {
    for (const name of only) await TOURS[name](ctx);
    return;
  }
  for (const visit of Object.values(TOURS)) await visit(ctx);
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await shot(ctx, page, "start-1280x800");
  ctx.check(
    "the status names the variant and seed",
    /Ordinary ambush · seed \d+/.test(await text(page, "status")),
    await text(page, "status"),
  );
  ctx.check(
    "the encounter status is published and running",
    /^Hold the village: 0\/\d+ s held · in progress$/.test(await text(page, "encounter")),
    await text(page, "encounter"),
  );

  // Select the tanks and right-click the ground: an accepted move.
  const o = await obs(page);
  const tanks = o.own.filter((u) => u.kind === "tank").map((u) => u.id);
  await lab(page, (ids) => window.__lab.route.select(ids), tanks);
  await page.waitForFunction((n) => window.__lab.route.selected().length === n, tanks.length);
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [360, 800, 0], distance: 600 }),
  );
  const spot = await lab(page, () => window.__lab.projectToCss(420, 800, 0));
  await page.mouse.click(spot[0], spot[1], { button: "right" });
  await page.waitForFunction(() => window.__lab.route.acks().length > 0, undefined, {
    timeout: 5000,
  });
  const ack = (await lab(page, () => window.__lab.route.acks()))[0];
  ctx.check(
    "right-clicking the ground moves the selected tanks",
    /move/.test(ack.label) && ack.ack.error === null,
    JSON.stringify(ack),
  );
  await advance(page, 30 * 20);
  const moved = await obs(page);
  await shot(ctx, page, "advance-1280x800");
  ctx.check(
    "the tanks drove toward the order",
    moved.own
      .filter((u) => tanks.includes(u.id))
      .every((u) => Math.hypot(u.position[0] - 420, u.position[1] - 800) < 150),
    JSON.stringify(moved.own.filter((u) => tanks.includes(u.id)).map((u) => u.position)),
  );
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [420, 800, 0], distance: 350 }),
  );
  await shot(ctx, page, "tanks-close-1280x800");

  // Each selected tank's callout carries the unit card's name; a
  // destination carries no text (27e follow-ups: its marker and route say
  // whose it is).
  const tags = await lab(page, () => ({
    names: [...document.querySelectorAll(".ro-unit.ro-selected .ro-name")].map((n) => ({
      unit: Number(n.parentElement.dataset.unit),
      text: n.textContent,
    })),
    panel: [...document.querySelectorAll("[data-testid=selection-panel] [data-unit] strong")].map(
      (n) => n.textContent,
    ),
    layer: [...(document.querySelector("[data-testid=readouts]")?.children ?? [])]
      .filter((e) => !e.matches(".ro-unit, .ro-leaders"))
      .map((e) => e.className),
  }));
  ctx.check(
    "each selected tank's callout shows the unit card's name, and no destination shows text",
    tags.layer.length === 0 &&
      tanks.every((id) => {
        const name = tags.names.find((n) => n.unit === id)?.text;
        return !!name && tags.panel.includes(name);
      }),
    JSON.stringify(tags),
  );

  // Zoomed out, where the two tanks' callouts would pile up, none sits under
  // the bars and none overprints another.
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [300, 800, 0], distance: 1150 }),
  );
  await shot(ctx, page, "tanks-far-1280x800");
  const placed = await lab(page, () => {
    const box = (e) => {
      const r = e.getBoundingClientRect();
      return { x0: r.left, x1: r.right, y0: r.top, y1: r.bottom };
    };
    const shown = (sel) =>
      [...document.querySelectorAll(sel)].filter((e) => e.style.display !== "none").map(box);
    return {
      panels: [...document.querySelectorAll("[data-occludes-readouts]")].map(box),
      readouts: shown(".ro-unit"),
    };
  });
  const overlap = (a, b) => a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1;
  ctx.check(
    "no readout sits under the HUD's bars or overprints another",
    placed.readouts.length > 0 &&
      placed.readouts.every(
        (b, k, all) =>
          placed.panels.every((p) => !overlap(b, p)) &&
          all.every((c, j) => j === k || !overlap(b, c)),
      ),
    JSON.stringify(placed),
  );

  // Sound: at a tick where blue hears something, the newest caption says
  // what, how far and from where, for the unit that heard it.
  const heard = await until(page, (o) => o.audible.length > 0, 30 * 120, 1);
  const cue = heard?.audible.at(-1);
  const listener = cue && heard.own.find((u) => u.id === cue.listener);
  const caption = heard && (await text(page, "captions")).split("\n")[0];
  ctx.check(
    "when blue hears something, a caption names the sound, its range and the listener",
    !!listener &&
      caption.startsWith("Heard ") &&
      caption.includes(`, ${cue.band}, `) &&
      caption.includes(` of ${listener.kind} #${listener.id}`),
    heard ? `tick ${heard.tick}: ${JSON.stringify(cue)} → ${caption}` : "never heard",
  );
  // Twenty seconds of fire later, repeats have collapsed into a few rows.
  await advance(page, 30 * 20);
  await page.evaluate(() => window.__lab.frame());
  const rows = await lab(page, () =>
    [...document.querySelectorAll("[data-testid=captions] li[data-count]")].map((li) => ({
      text: li.textContent.replace(/ ×\d+$/, ""),
      count: Number(li.dataset.count),
    })),
  );
  ctx.check(
    "repeated sounds collapse into at most three counted rows",
    rows.length <= 3 && new Set(rows.map((r) => r.text)).size === rows.length,
    JSON.stringify(rows),
  );

  // Export: the file names its variant and carries the accepted commands.
  const file = await lab(page, () => window.__lab.route.exportReplay());
  ctx.check(
    "the replay export records its variant and commands",
    file?.variant === "ordinary" && JSON.parse(file.replay).accepted?.length >= 1,
    file && file.replay.slice(0, 120),
  );

  // Pause holds the tick; resume continues.
  const held = await lab(page, () => window.__lab.route.tick());
  await page.waitForTimeout(400);
  ctx.check(
    "paused, the battle does not advance",
    (await lab(page, () => window.__lab.route.tick())) === held,
  );

  // Reset starts again from the seed with an empty log.
  await page.getByRole("button", { name: "Reset" }).click();
  await page.waitForFunction(
    () => window.__lab.route.acks().length === 0 && window.__lab.route.tick() < 60,
  );
  ctx.check("reset rebuilds from the seed", true);

  // The seed and the variant are the player's to change.
  await page.getByRole("button", { name: "Scenario" }).click();
  await page.getByLabel("Seed", { exact: true }).fill("7");
  await page.waitForFunction(
    () => /seed 7 /.test(document.querySelector("[data-testid=status]")?.textContent ?? ""),
    undefined,
    { timeout: 30000 },
  );
  ctx.check("a new seed restarts the battle on that seed", true);
  await page.getByRole("button", { name: "Scenario" }).click();
  await page.getByRole("radio", { name: "Prepared crossfire" }).click();
  await page.waitForFunction(
    () =>
      /Prepared crossfire/.test(document.querySelector("[data-testid=status]")?.textContent ?? ""),
    undefined,
    { timeout: 30000 },
  );
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  ctx.check("the variant select loads the crossfire battle", true);
}
