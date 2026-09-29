// Slice 15: the village battle. Blue plays through the production controls;
// the encounter status, variant, seed, pause/reset and replay export work.
// The camera tour, from the opening framing out to the
// strategic height and in to the ground, through the real wheel.
// The tree-line tour, the forests drawn as trees, and
// the scenery's GPU resources returned on rebuild.
// The grass field at those framings (GRASS_COST=1 also
// measures its GPU cost; run it alone, under the GPU lock).
// Soldiers as posed models, by detail tier and as
// impostor cards, never fogged, picked by the simulation's boxes, and the
// fallen as static corpses.
// Vehicles, buildings and wrecks as their appearances:
// every vehicle a posed model following its published weapon poses, the
// village's houses fitted to their boxes, and a tank firing (recoil).
// Order markers and the Space overlay (a real
// right-drag's facing, a reverse move's marker, Space held at the default and
// ground cameras over the fog), and no enemy plan in the observation.
// Combat effects in the firefight, a burst read at its
// moment and after, from the effects' own frame and the pass inspector's
// world view.
import { readFile } from "node:fs/promises";
import { lab, obs, advance, until, snapshot, presented } from "./_lab.mjs";
import { decode, pixel, writeCrop } from "./_png.mjs";
import { checkOverlayIsolation, paintOnly } from "./_overlays.mjs";
import { cleanupTour, woodsTour } from "./_battleLook.mjs";
import { hasRole, hull as hullOf, isVehicle, unitType, vehicleAppearances } from "./_units.mjs";

const village = JSON.parse(
  await readFile(new URL("../../fixtures/village.json", import.meta.url), "utf8"),
);
const CAMERA = village.presentation.camera;
/** Every mark, paint or overlay, lies on the ground itself (the ground and
 *  its blades read the paint at their own point): checks read the marks at
 *  the surface. */
/** The tour's fixed tick: every framing shows the same battle state. */
const TOUR_TICK = 90;

const text = (page, id) => page.getByTestId(id).innerText();

async function shot(ctx, page, name) {
  await snapshot(ctx, page, `frame-${name}.png`);
}

/** The road is drawn where the simulation has it. Top
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

/** The grass field, read back at a framing. */
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
  // orders are painted in the world, so this frame may hold no overlay at all
  // (decisions.md, "the ground paint's checks read the paint's rise").
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
  // The world only: the HUD's full-width bars cover the frame's
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
  // Soldiers' own tiers (vehicles and props share the layer).
  const finest = (s) => s.bodyTiers.findIndex((n) => n > 0);
  ctx.check(
    "every drawn soldier is a posed model: finer tiers near, impostor cards far, only meshes posed",
    // Vehicles and props are models too, so soldiers are counted
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
  // A death plays on the presentation clock from the frame that first
  // presents its soldier fallen, and that clock stands still at the last
  // presented tick while the battle is paused. Under load few frames draw
  // while the battle steps, so a death can start late: present the fight's
  // tick first, so every death in it has started by then.
  if (fight) await presented(page);
  // Two seconds on, the first deaths are playing out.
  if (fight) await advance(page, 60);
  const shown = await obs(page);
  const fallen = shown.corpses[0];
  if (fallen) {
    await frameOn(page, fallen.position, { distance: 30, pitch: 0.6 });
    await snapshot(ctx, page, "soldiers-fallen-1920x1080.png");
  }
  // Then a second at a time, each presented, until one lies static: within
  // five seconds of the fight, twice a death's length.
  let lying = null;
  for (let t = 60; fight; t += 30) {
    await presented(page);
    lying = await lab(page, () => window.__lab.stats().models);
    if (lying.corpses >= 1 || t >= 150) break;
    await advance(page, 30);
  }
  const after = await obs(page);
  ctx.check(
    "the fallen lie as static corpses, drawn and never posed",
    !!fight &&
      !!lying &&
      lying.corpses >= 1 &&
      lying.corpses <= after.corpses.length &&
      lying.instances > lying.skinned,
    JSON.stringify({ tick: after.tick, fallen: after.corpses.length, lying }),
  );
  await page.close();
}

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

/** Vehicles, buildings and wrecks are appearances placed, fitted and
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
      posed.every((v) => v.articulation && vehicleAppearances.has(v.appearance)) &&
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
  // The fences and sandbags are drawn apart too (bodies a vehicle
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

/** The firefight's combat effects. The battle at a fixed tick, the
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

  // The live battle is heard once the player first clicks, and a
  // pause silences its transients while its loops hold.
  await page.click('[data-testid="battle-panel"] strong');
  await lab(page, () => window.__lab.route.resume());
  // The sound bank is synthesised after the gesture; under load that takes
  // seconds, so wait for the state rather than a fixed time.
  // Each sample is taken in the poll that met its condition, so a frame
  // between the wait and a later read can't change what is judged.
  const sample = (condition, timeout) =>
    page
      .waitForFunction(condition, undefined, { timeout })
      .then((h) => h.jsonValue())
      .catch(() => lab(page, () => window.__lab.route.sound()));
  const live = await sample(() => {
    const s = window.__lab.route.sound();
    return !!s?.running && s.started > 0 && s.loops > 0 && s;
  }, 30000);
  // The pause, then a one-tick advance as a fence: it resolves once that
  // tick's publication is consumed, so every publication sent before the
  // pause has arrived and nothing more will. Under load, the presentation
  // otherwise plays out that backlog after the pause, and its clock moving
  // again restarts transients.
  await lab(page, () => window.__lab.route.pause());
  await lab(page, () => window.__lab.route.advance(1));
  const held = await sample(() => {
    const s = window.__lab.route.sound();
    // Held, silent, and it has heard the last tick published.
    return !!s?.held && s.transients === 0 && s.tick === window.__lab.route.tick() && s;
  }, 30000);
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

/** The aftermath. The first wreck blue learns burns and smokes
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
 *  ramp's white (light), light green (medium) and strong green (heavy). */
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
    else if (g > r + 140 && g > b + 100) green++;
    else if (g > r + 50 && g > b + 50) mint++;
  }
  return { lit, yellow, white, mint, green };
}

/** The orders' yellow in an overlay-on-black frame, summed by its red
 *  channel: how much order ink there is and how strongly it is drawn. */
function yellowInk(png) {
  let ink = 0;
  for (let i = 0; i < png.data.length; i += 4) {
    const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
    if (r > 40 && r > b + 25 && g > 0.7 * r) ink += r;
  }
  return ink;
}

/** The order flash (post-close, 2026-09-28): an order's marks are the Space
 *  view's, shown for its units as the order is given, held for
 *  `orders.flash.hold_s` and faded over `fade_s`; a selection alone shows
 *  none. Holding Space shows the same marks, pixel for pixel. A queued
 *  (Shift) order flashes too. */
async function orderFlashTour(ctx) {
  const { flash } = village.presentation.overlay.orders;
  const tickHz = village.tick_hz;
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  const o = await obs(page);
  const rifle = o.own.find((u) => u.kind === "rifle");
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id]);
  await page.waitForFunction((id) => window.__lab.route.selected()[0] === id, rifle.id);
  const goal = [rifle.position[0] + 30, rifle.position[1]];
  const at = [goal[0] - 12, goal[1]];
  const views = {
    default: { distance: CAMERA.default.distance, pitch: 0.85 },
    far: { distance: 250, pitch: 0.85 },
  };
  const frame = async (view) => {
    await frameAt(page, at, views[view].distance, views[view].pitch, CAMERA.default.yaw);
    // Lines keep their width on screen: rebuilt a frame after a zoom step.
    await lab(page, () => window.__lab.frame());
    await lab(page, () => window.__lab.frame());
  };
  // The order marks are overlay (`yellow-orders`), read over black with the
  // callouts (DOM) hidden; the selection's own markers are paint.
  const overlay = async (name) => {
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
  // Each state: the shot at the default and far cameras, and its order ink
  // at the default one.
  const state = async (name) => {
    await frame("far");
    await snapshot(ctx, page, `order-flash-${name}-far-1920x1080.png`);
    await frame("default");
    await snapshot(ctx, page, `order-flash-${name}-default-1920x1080.png`);
    return overlay(`order-flash-${name}`);
  };
  const selected = yellowInk(await state("selected"));

  // A real right-click on the ground, then the tick that publishes the route.
  const press = await lab(
    page,
    (q) => window.__lab.projectToCss(q[0], q[1], window.__lab.route.surfaceZ(q[0], q[1])),
    goal,
  );
  await page.mouse.click(press[0], press[1], { button: "right" });
  await page.waitForFunction(() => window.__lab.route.acks().length > 0);
  const moving = await until(page, (x) => x.own.find((u) => u.id === rifle.id)?.goal, 10, 1);
  ctx.check("the right-click sent the squad a move", !!moving);
  const issuedPng = await state("issued");
  const issued = yellowInk(issuedPng);
  // Space held at the same tick: every own unit's marks, the squad's the
  // flash's pixel for pixel (its route and destination, where the flash drew
  // them; other units' marks and their glow may add, never change, them).
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders());
  const spacePng = await overlay("order-flash-issued-space");
  await page.keyboard.up("Space");
  await page.waitForFunction(() => !window.__lab.route.showOrders());
  let flashInked = 0,
    differs = 0;
  for (let i = 0; i < issuedPng.data.length; i += 4) {
    const [r, g, b] = [issuedPng.data[i], issuedPng.data[i + 1], issuedPng.data[i + 2]];
    if (!(r > 120 && r > b + 50 && g > 0.7 * r)) continue;
    flashInked++;
    const d = Math.max(
      Math.abs(r - spacePng.data[i]),
      Math.abs(g - spacePng.data[i + 1]),
      Math.abs(b - spacePng.data[i + 2]),
    );
    if (d > 24) differs++;
  }
  ctx.check(
    "an order just given shows its unit's marks, as Space draws them",
    issued > selected + 50000 && flashInked > 500 && differs <= flashInked * 0.02,
    JSON.stringify({ selected, issued, flashInked, differs }),
  );

  // Through the hold, still in full; half way through the fade, dimmer;
  // two seconds on, gone, as with the selection alone.
  const since = (s) => Math.round(s * tickHz);
  await advance(page, since(flash.hold_s) - 2);
  const held = yellowInk(await overlay("order-flash-held"));
  await advance(page, since(flash.fade_s / 2) + 2);
  const fading = yellowInk(await overlay("order-flash-fading"));
  await advance(page, since(2) - since(flash.hold_s + flash.fade_s / 2));
  const after = yellowInk(await state("2s"));
  ctx.check(
    "the flash holds, fades, and is gone two seconds on",
    held > 0.8 * issued &&
      fading < 0.8 * held &&
      fading > selected + 0.15 * (held - selected) &&
      after < selected + 0.02 * (issued - selected),
    JSON.stringify({ selected, issued, held, fading, after }),
  );

  // Space held two seconds on: the marks again, at the default and far
  // cameras (evidence), and plenty of ink.
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders());
  const space = yellowInk(await state("space"));
  await page.keyboard.up("Space");
  await page.waitForFunction(() => !window.__lab.route.showOrders());
  ctx.check(
    "holding Space shows the order marks again",
    space > issued * 0.8,
    JSON.stringify({ issued, space }),
  );

  // A queued waypoint (Shift+right-click) flashes the squad's marks too.
  await lab(page, () => window.__lab.frame());
  const quiet = yellowInk(await overlay("order-flash-quiet"));
  const next = await lab(
    page,
    (q) => window.__lab.projectToCss(q[0], q[1], window.__lab.route.surfaceZ(q[0], q[1])),
    [goal[0], goal[1] + 15],
  );
  const acks = (await lab(page, () => window.__lab.route.acks())).length;
  await page.keyboard.down("Shift");
  await page.mouse.click(next[0], next[1], { button: "right" });
  await page.keyboard.up("Shift");
  await page.waitForFunction((n) => window.__lab.route.acks().length > n, acks);
  await advance(page, 2);
  const queued = yellowInk(await overlay("order-flash-queued"));
  ctx.check(
    "a queued order flashes its unit's marks too",
    quiet < selected + 0.02 * (issued - selected) && queued > 0.8 * issued,
    JSON.stringify({ selected, issued, quiet, queued }),
  );
  await page.close();
}

/** Total War markers (D2), the Space overlay (D2+), right-drag
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
  // Every mark lies on the surface.
  const toMark = toCss;

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
  // Cover icons are found by where and what they are, not by colour alone
  // (light cover shares the orders' yellow): a filled pip in the middle of
  // each soldier's marker (his "cover now") and of each destination spot's
  // ("cover there"), in its tier's colour, where a soldier without cover
  // shows the marker's empty middle.
  const tierOf = ([r, g, b]) =>
    r + g + b < 90
      ? null
      : g > r + 140 && g > b + 100
        ? "heavy"
        : g > r + 50 && g > b + 50
          ? "medium"
          : r > b + 50 && g > b + 30
            ? "light"
            : "other";
  const middle = (p) => {
    const sum = [0, 0, 0];
    for (let dy = -1; dy <= 1; dy++)
      for (let dx = -1; dx <= 1; dx++) {
        const i = ((Math.round(p[1]) + dy) * inked.width + Math.round(p[0]) + dx) * 4;
        for (let k = 0; k < 3; k++) sum[k] += inked.data[i + k] / 9;
      }
    return sum;
  };
  const pips = { checked: 0, right: 0, wrong: [] };
  for (const u of o.own.filter((v) => v.members.length > 0))
    for (const [k, m] of u.memberOrders.entries()) {
      const cases = [[u.members[k], m.coverNow], ...(u.goal ? [[m.spot, m.coverThere]] : [])];
      for (const [q, tier] of cases) {
        if (!q) continue;
        const p = await toMark([q[0], q[1]]);
        if (!p || p[0] < 8 || p[1] < 8 || p[0] > 1912 || p[1] > 1072) continue;
        pips.checked++;
        const seen = tierOf(middle(p));
        if (seen === (tier ?? null)) pips.right++;
        else if (pips.wrong.length < 6) pips.wrong.push({ unit: u.id, tier, seen });
      }
    }
  ctx.check(
    "cover icons sit in the middle of each soldier's marker, in their tier's colour",
    pips.checked >= 8 && pips.right >= pips.checked * 0.8,
    JSON.stringify({ tiers: [...tiers], pips }),
  );
  // Centred, to the pixel: a green (medium or heavy) pip's centroid lies
  // within a pixel of its marker circle's centre, for a soldier where he
  // stands and for a destination spot.
  const centred = { now: [], there: [] };
  const pixelAt = (x, y) => {
    const i = (y * inked.width + x) * 4;
    return [inked.data[i], inked.data[i + 1], inked.data[i + 2]];
  };
  const isRing = ([r, g, b]) => r > 60 && g > 0.7 * r && b < 0.6 * r && r > g * 0.9;
  const isPip = (c) => ["medium", "heavy"].includes(tierOf(c));
  for (const u of o.own.filter((v) => v.members.length > 0))
    for (const [k, m] of u.memberOrders.entries())
      for (const [which, q, tier] of [
        ["now", u.members[k], m.coverNow],
        ...(u.goal ? [["there", m.spot, m.coverThere]] : []),
      ]) {
        if (!q || !["medium", "heavy"].includes(tier) || centred[which].length >= 4) continue;
        const p = await toMark([q[0], q[1]]);
        if (!p || p[0] < 20 || p[1] < 20 || p[0] > 1900 || p[1] > 1060) continue;
        // The marker's radius on screen, so only its own ring (an annulus
        // round the soldier) and its own pip (inside it) count, not a route
        // or a ring crossing near it.
        const edge = await toMark([q[0] + 0.45, q[1]]);
        const R = Math.max(3, Math.hypot(edge[0] - p[0], edge[1] - p[1]));
        const sum = { pip: [0, 0, 0], ring: [0, 0, 0] };
        const reach = Math.ceil(R * 1.6);
        for (let dy = -reach; dy <= reach; dy++)
          for (let dx = -reach; dx <= reach; dx++) {
            const [x, y] = [Math.round(p[0]) + dx, Math.round(p[1]) + dy];
            const d = Math.hypot(x - p[0], y - p[1]);
            const c = pixelAt(x, y);
            const into =
              d < R * 0.8 && isPip(c)
                ? sum.pip
                : d > R * 0.6 && d < R * 1.5 && isRing(c)
                  ? sum.ring
                  : null;
            if (into) ((into[0] += x), (into[1] += y), into[2]++);
          }
        // The marker's circle is drawn round the soldier's (or the spot's)
        // own point, `p` on screen; a ring the hull or another mark cuts or
        // crosses would move a measured centre, so the circle's centre is
        // that point, and a marker must show enough of its ring to count.
        // A pip another mark covers part of (a route, an arrowhead drawn over
        // it) is skipped: only a whole pip has a centroid worth reading.
        const whole = Math.PI * (R * (0.3 / 0.45)) ** 2;
        if (sum.pip[2] < whole * 0.7 || sum.ring[2] < 8) continue;
        const pip = [sum.pip[0] / sum.pip[2], sum.pip[1] / sum.pip[2]];
        centred[which].push({
          dpx: +Math.hypot(pip[0] - p[0], pip[1] - p[1]).toFixed(2),
          at: p.map((v) => +v.toFixed(1)),
          pip: pip.map((v) => +v.toFixed(1)),
        });
      }
  // An order is the unit's (user: "it should always be as a unit"): no line
  // runs from a soldier. Half way from each soldier to his spot (3 m or more
  // off), the overlay holds none of the orders' yellow, but for other marks
  // crossing there by chance (at most a tenth).
  const legs = { checked: 0, inked: 0 };
  for (const u of o.own.filter((v) => v.members.length > 0))
    for (const [k, m] of u.memberOrders.entries()) {
      const q = u.members[k];
      // A moving squad's soldiers walk to spots along its own route; a
      // holding squad's walk to their posts, where lines once ran.
      if (u.goal || !q || Math.hypot(m.spot[0] - q[0], m.spot[1] - q[1]) < 3) continue;
      const p = await toMark([(q[0] + m.spot[0]) / 2, (q[1] + m.spot[1]) / 2]);
      if (!p || p[0] < 8 || p[1] < 8 || p[0] > 1912 || p[1] > 1072) continue;
      legs.checked++;
      if (isRing(pixelAt(Math.round(p[0]), Math.round(p[1])))) legs.inked++;
    }
  ctx.check(
    "with Space, no line runs from a soldier to his spot: each order is one line for its unit",
    legs.inked <= Math.floor(legs.checked / 10),
    JSON.stringify(legs),
  );
  const all = [...centred.now, ...centred.there];
  ctx.check(
    "a cover icon's centre is its marker's centre, within a pixel",
    all.length > 0 && all.every((c) => c.dpx <= 1),
    JSON.stringify(centred),
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
  // A route lies over the grass, solid: along the middle of each
  // first leg in view, the ribbon's centre line is ink at every pixel (the
  // blades once poked through it as dark speckle). Squads only: a vehicle's
  // route stops short of its ring.
  // The HUD's bars (a top bar and a bottom command bar).
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
  // wide, so a sample's ink is the sum of the five pixels across
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
    // edge of its area ring (both drawn at the fixture's scale):
    // sample only the line between them.
    const scale = village.presentation.overlay.orders.area_draw_scale;
    const here = Math.max(
      ...u.members.map((m) => Math.hypot(m[0] - u.position[0], m[1] - u.position[1])),
    );
    // Past the circle round the soldiers and its arrowhead, where the route
    // leaves along the facing.
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
    // Painted in the lit world, a route takes the soldiers'
    // shadows, which dim it by up to about a sixth; a blade over it hides it
    // (decisions.md, "the ground paint's checks read the paint's rise").
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
  await checkSoldiersNotOverdrawn(ctx, page);
  await frameAt(page, at, cameras.default.distance, cameras.default.pitch, CAMERA.default.yaw);
  await page.keyboard.up("Space");
  await page.waitForFunction(() => !window.__lab.route.showOrders());
  ctx.check("releasing Space hides the overlay again", true);
  // The selection's callouts and markers without Space (evidence): no
  // order marks, since no order was just given.
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id, tank.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 2);
  await frameAt(page, at, cameras.default.distance, cameras.default.pitch, CAMERA.default.yaw);
  await lab(page, () => window.__lab.frame());
  await snapshot(ctx, page, "orders-selected-default-1920x1080.png");
  // The selection's markers against its orders: Space held, so they show.
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders());
  await checkSelectionYellow(ctx, page, rifle.id, tank.id);
  await checkRimJoin(ctx, page, rifle.id);
  await page.keyboard.up("Space");
  await page.waitForFunction(() => !window.__lab.route.showOrders());
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

/** An overlay order mark never draws over a body (post-close review, 6).
 *  With Space held every soldier stands on his marker ring, in the overlay
 *  under `yellow-orders`. From a low camera the ring's far arc lies behind
 *  his legs on screen, so the overlay's clearance over the ground must not
 *  reach his shins. His body is the ground mask's non-ground pixels in a
 *  band over his feet; above his ankles (`ANKLE_M`), none of them may carry
 *  the orders' yellow. */
async function checkSoldiersNotOverdrawn(ctx, page) {
  const ANKLE_M = 0.12;
  const o = await obs(page);
  const squads = o.own.filter((u) => u.members.length >= 3);
  const squad = squads.find((u) => !u.goal) ?? squads[0];
  if (!squad) {
    ctx.check("a soldier on his marker is never overdrawn by it", false, "no squad");
    return;
  }
  const lead = squad.members[0];
  await frameAt(page, [lead[0], lead[1]], CAMERA.zoom_min, 0.6, CAMERA.default.yaw);
  await lab(page, () => window.__lab.frame());
  await lab(page, () => window.__lab.frame());
  await page.evaluate(() => {
    document.querySelector("[data-testid=readouts]").style.visibility = "hidden";
  });
  await snapshot(ctx, page, "overdraw-final.png");
  const view = async (v, name) => {
    await lab(page, (x) => window.__lab.setFrameView(x), v);
    return decode(await snapshot(ctx, page, name));
  };
  const ground = await view("ground-mask", "overdraw-ground-mask.png");
  const over = await view("overlays-on-black", "overdraw-overlay.png");
  await lab(page, () => window.__lab.setFrameView("final"));
  await page.evaluate(() => {
    document.querySelector("[data-testid=readouts]").style.visibility = "";
  });
  const at = (png, x, y) => {
    const i = (y * png.width + x) * 4;
    return [png.data[i], png.data[i + 1], png.data[i + 2]];
  };
  const yellow = ([r, g, b]) => r > 40 && g > 0.6 * r && b < 0.6 * r;
  const soldiers = [];
  for (const m of squad.members) {
    const z = await lab(page, (q) => window.__lab.route.surfaceZ(q[0], q[1]), m);
    const css = (h) =>
      lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), [m[0], m[1], z + h]);
    const [feet, ankle, head] = [await css(0), await css(ANKLE_M), await css(1.9)];
    if (!feet || !head || head[1] < 2 || feet[1] > 1078 || feet[0] < 40 || feet[0] > 1880) continue;
    const half = Math.max(6, Math.round((feet[1] - head[1]) * 0.3));
    let body = 0;
    let inked = 0;
    for (let y = Math.max(0, Math.round(head[1])); y < Math.round(ankle[1]); y++)
      for (let x = Math.round(feet[0]) - half; x <= Math.round(feet[0]) + half; x++) {
        const [r, g, b] = at(ground, x, y);
        if (r + g + b > 30) continue;
        body++;
        if (yellow(at(over, x, y))) inked++;
      }
    soldiers.push({ feet: feet.map(Math.round), body, inked });
  }
  const body = soldiers.reduce((n, s) => n + s.body, 0);
  const inked = soldiers.reduce((n, s) => n + s.inked, 0);
  ctx.check(
    "a soldier on his marker is never overdrawn by it: no order ink on his body above the ankles",
    soldiers.length >= 2 && body > 400 && inked <= Math.max(4, body * 0.002),
    JSON.stringify({ squad: squad.id, body, inked, soldiers }),
  );
}

/** The selection has its own colour: a selected squad's circle where it
 *  stands and a selected vehicle's marker are the scheme's `selected` role
 *  (amber paint under `yellow-orders`), while the squad's destination area
 *  ring stays the order colour. */
async function checkSelectionYellow(ctx, page, squadId, vehicleId) {
  const o = await obs(page);
  const squad = o.own.find((u) => u.id === squadId);
  const vehicle = o.own.find((u) => u.id === vehicleId);
  const { area_draw_scale: scale, vehicle_marker_margin_m: margin } =
    village.presentation.overlay.orders;
  const vehicleR = hullOf(vehicle.kind).half_extents_m[0] + margin;
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
  // Samples round a circle: how many are inked, and how many of those in
  // the selection's colour; on the overlay (the selection) or the paint.
  const circle = async (c, radius, onOverlay = true) => {
    const out = { inked: 0, yellow: 0 };
    const look = onOverlay ? nearOverlay : near;
    for (let k = 0; k < 32; k++) {
      const a = (k / 32) * 2 * Math.PI;
      const q = [c[0] + Math.cos(a) * radius, c[1] + Math.sin(a) * radius];
      const z = await lab(page, (w) => window.__lab.route.surfaceZ(w[0], w[1]), q);
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
    [...tipAt, 0],
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
          [...q, 0],
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
        [...q, 0],
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

/** Paint and overlay marks lie at one height: a squad's
 *  route joins its circle exactly where the circle ends, along the route's
 *  first leg on screen. Selected, the circle is amber paint and the route
 *  yellow overlay: the circle's last pixel and the route's first meet within
 *  a pixel. Unselected, both are overlay: the line from the circle into the
 *  route has no gap. Space is held throughout (the caller's), so the orders
 *  show. */
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
  // Unselected: circle and route both overlay, one run of yellow.
  await lab(page, () => window.__lab.route.select([]));
  const plain = await overlayShot("rim-join-unselected-overlay.png");
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

/** Painted ground marks sit under the effects: on a moving
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
    hullOf(vehicle.kind).half_extents_m[0] +
    village.presentation.overlay.orders.vehicle_marker_margin_m;
  const at = [];
  for (let k = 0; k < 64; k++) {
    const a = (k / 64) * 2 * Math.PI;
    const q = [vehicle.position[0] + Math.cos(a) * r, vehicle.position[1] + Math.sin(a) * r];
    const z = await lab(page, (w) => window.__lab.route.surfaceZ(w[0], w[1]), q);
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
  const [hx, hy, hz] = hullOf(vehicle.kind).half_extents_m;
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

/** GLOW_SHEET=1: the ground marks' glow options, a selected
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

/** The village's field works (teeth, sandbags, fences)
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

/** A flash's core projects this close to the drawn
 *  muzzle, in CSS px at the default camera. */
const MUZZLE_PX = 3;
/** The flash's own light, added over the frame without effects, within
 *  `FLASH_BOX_PX` of the drawn muzzle: at least this bright (0..255). */
const FLASH_BOX_PX = 6;
const FLASH_LIGHT_MIN = 60;
/** A hull's round starts this close to its
 *  flash, in metres. The drawn muzzle leads or trails the simulation's by
 *  the gun's pitch and recoil and a tick's drive; the phantom muzzle this
 *  replaced was metres off for an HMG turned from the cannon. */
const TRACER_M = 1;

/** Every flash sits on the muzzle as the model draws
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
/** Select similar (double-click a unit for its
 *  type, again or with Ctrl for its role), a mixed selection's command bar
 *  (the union of its capabilities, each order to the units that can), and
 *  the unit card's role symbol and silhouette. */
async function selectionTour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1600, height: 900 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  const o = await obs(page);
  const ids = (kind) => o.own.filter((u) => u.kind === kind).map((u) => u.id);
  const [rifles, tanks, trucks] = [ids("rifle"), ids("tank"), ids("supply")];
  const selected = () => lab(page, () => [...window.__lab.route.selected()].sort((a, b) => a - b));
  const same = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);
  /** Double-click a unit where it is drawn: a soldier's torso, a hull's middle. */
  const doubleClick = async (unit, modifiers = []) => {
    const at = unit.members.length ? unit.members[0] : unit.position;
    await frameAt(page, at, 60, 0.85, CAMERA.default.yaw);
    await lab(page, () => window.__lab.frame());
    const px = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1), at);
    for (const key of modifiers) await page.keyboard.down(key);
    await page.mouse.dblclick(px[0], px[1]);
    for (const key of modifiers) await page.keyboard.up(key);
    await page.evaluate(() => new Promise((r) => requestAnimationFrame(r)));
  };
  const rifle = o.own.find((u) => u.id === rifles[0]);
  await doubleClick(rifle);
  const byType = await selected();
  ctx.check(
    "double-clicking a rifle squad selects every own rifle squad",
    rifles.length > 1 &&
      same(
        byType,
        [...rifles].sort((a, b) => a - b),
      ),
    JSON.stringify({ byType, rifles }),
  );
  await doubleClick(rifle);
  const byRole = await selected();
  const infantry = o.own
    .filter((u) => hasRole(u.kind, unitType("rifle").roles[0]))
    .map((u) => u.id)
    .sort((a, b) => a - b);
  ctx.check(
    "a second double-click widens to the rifle's role (infantry), not recon or AT",
    same(byRole, infantry),
    JSON.stringify({ byRole, infantry }),
  );
  await lab(page, () => window.__lab.route.select([]));
  const tank = o.own.find((u) => u.id === tanks[0]);
  await doubleClick(tank, ["Control"]);
  const tanksByRole = await selected();
  ctx.check(
    "Ctrl + double-click on a tank selects by its role at once: both tanks",
    tanks.length > 1 &&
      same(
        tanksByRole,
        [...tanks].sort((a, b) => a - b),
      ),
    JSON.stringify({ tanksByRole, tanks }),
  );

  // The card: the tank's role symbol beside its silhouette.
  await lab(page, (id) => window.__lab.route.select([id]), tanks[0]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  const card = page.getByTestId("selection-panel");
  const icons = await card.evaluate((c) =>
    [...c.querySelectorAll(".ro-unit-icons svg")].map((s) => {
      const r = s.getBoundingClientRect();
      return { w: r.width, h: r.height, paths: s.querySelectorAll("path").length };
    }),
  );
  ctx.check(
    "the unit card shows the role symbol and the model's silhouette, each at least 20 px tall",
    icons.length === 2 && icons.every((i) => i.h >= 20),
    JSON.stringify(icons),
  );
  await page.locator("footer.hud-bottom").screenshot({
    path: ctx.evidencePath("selection-card-tank.png"),
  });
  await lab(page, (id) => window.__lab.route.select([id]), rifles[0]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  await page.locator("footer.hud-bottom").screenshot({
    path: ctx.evidencePath("selection-card-rifle.png"),
  });

  // A tank and a supply truck: Deploy is lit and reaches the truck only.
  await lab(page, (sel) => window.__lab.route.select(sel), [tanks[0], trucks[0]]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 2);
  const deploy = page.getByRole("button", { name: /^Deploy / });
  const bar = await deploy.evaluate((b) => ({ disabled: b.disabled, reach: b.dataset.reach }));
  ctx.check(
    "with a tank and a supply truck selected, Deploy is lit and reaches 1 of 2",
    !bar.disabled && bar.reach === "1/2",
    JSON.stringify(bar),
  );
  await page.locator("footer.hud-bottom").screenshot({
    path: ctx.evidencePath("selection-group-mixed.png"),
  });
  const before = (await lab(page, () => window.__lab.route.acks())).length;
  await deploy.click();
  await page.waitForFunction((n) => window.__lab.route.acks().length > n, before);
  const [ack] = await lab(page, () => window.__lab.route.acks());
  await advance(page, 3);
  const after = await obs(page);
  const truck = after.own.find((u) => u.id === trucks[0]);
  const tankAfter = after.own.find((u) => u.id === tanks[0]);
  ctx.check(
    "Deploy orders only the truck: it sets up, the tank carries on",
    ack.ack.error === null &&
      ack.label === `deploy supply #${trucks[0]}` &&
      truck.deployment?.target === "deployed" &&
      !tankAfter.deployment,
    JSON.stringify({ ack, truck: truck.deployment, tank: tankAfter.deployment }),
  );
  await snapshot(ctx, page, "selection-mixed-1600x900.png");
  await page.close();
}

/** The range ruler (Space held with a selection) and right-clicking a
 *  contact's area to attack it. */
async function rulerTour(ctx) {
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
  const ruler = () => lab(page, () => window.__lab.route.ruler());
  /** The ruler's text as shown: the distance, and each tick's label. */
  const shownText = () =>
    lab(page, () => {
      const layer = document.querySelector("[data-testid=range-ruler]");
      const visible = (e) => e && getComputedStyle(e).display !== "none";
      return {
        shown: visible(layer),
        distance: layer?.querySelector(".rr-distance")?.textContent ?? null,
        weapons: [...(layer?.querySelectorAll(".rr-weapon") ?? [])].map((w) => ({
          text: w.textContent,
          inRange: w.dataset.inRange === "true",
        })),
        ticks: [...(layer?.querySelectorAll(".rr-tick") ?? [])]
          .filter(visible)
          .map((t) => t.textContent),
      };
    });
  /** The muzzle-to-aim distance from `unit` to the ground at `p`, as the
   *  simulation's range check measures it (computed here from the fixture). */
  const expected = async (unit, p) => {
    const [first] = unitType(unit.kind).mounts;
    const muzzle = first?.muzzle_m
      ? first.pivot_m[2] + first.muzzle_m[2]
      : village.physics.infantry_muzzle_m;
    const dz = (await surface(p)) + village.physics.infantry_aim_m - (unit.position[2] + muzzle);
    return Math.hypot(p[0] - unit.position[0], p[1] - unit.position[1], dz);
  };
  const frames = async () => {
    await lab(page, () => window.__lab.frame());
    await lab(page, () => window.__lab.frame());
  };

  // Default camera: the rifle squad and the tank selected, the cursor on the
  // ground 40 m past the squad, away from the tank.
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id, tank.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 2);
  const away = Math.atan2(
    rifle.position[1] - tank.position[1],
    rifle.position[0] - tank.position[0],
  );
  const near = [rifle.position[0] + Math.cos(away) * 40, rifle.position[1] + Math.sin(away) * 40];
  const mid = [(rifle.position[0] + near[0]) / 2, (rifle.position[1] + near[1]) / 2];
  await frameAt(page, mid, CAMERA.default.distance, 0.85, CAMERA.default.yaw);
  await frames();
  let css = await toCss(near);
  await page.mouse.move(css[0], css[1]);
  await frames();
  ctx.check("without Space, no ruler", (await ruler()) === null && !(await shownText()).shown);
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders());
  await frames();
  let r = await ruler();
  let want = await expected(rifle, near);
  let text = await shownText();
  ctx.check(
    "with Space held, the ruler runs from the selected unit nearest the cursor, measuring muzzle to aim point",
    r?.unit === rifle.id && Math.abs(r.distance_m - want) < 0.5,
    JSON.stringify({ unit: r?.unit, distance: r?.distance_m, want }),
  );
  ctx.check(
    "the ruler shows its distance in metres, and every weapon of the squad reaches 40 m",
    text.shown &&
      text.distance === `${Math.round(r.distance_m)} m` &&
      text.weapons.length === r.marks.length &&
      text.weapons.every((w) => w.inRange) &&
      text.ticks.length === 0,
    JSON.stringify(text),
  );
  await snapshot(ctx, page, "ruler-default-1920x1080.png");
  // The ruler is paint: the frame with it less the frame with the pointer
  // off the canvas (no ruler) is lit along the line.
  const withRuler = decode(await snapshot(ctx, page, "ruler-default-on.png"));
  await page.mouse.move(960, 1075);
  await frames();
  const offCanvas = await ruler();
  const without = decode(await snapshot(ctx, page, "ruler-default-off.png"));
  const quarter = await toCss([
    rifle.position[0] + Math.cos(away) * 30,
    rifle.position[1] + Math.sin(away) * 30,
  ]);
  let lit = 0;
  for (let dy = -4; dy <= 4; dy++)
    for (let dx = -4; dx <= 4; dx++) {
      const a = pixel(withRuler, quarter[0] + dx, quarter[1] + dy);
      const b = pixel(without, quarter[0] + dx, quarter[1] + dy);
      if (Math.abs(a[0] - b[0]) + Math.abs(a[1] - b[1]) + Math.abs(a[2] - b[2]) > 40) lit++;
    }
  ctx.check(
    "the ruler is painted along its line, and goes with the pointer off the ground",
    offCanvas === null && lit >= 4,
    JSON.stringify({ lit, offCanvas }),
  );

  // The tank alone, the cursor 40 m off it: the line leaves its marker's
  // circle at the border, as it leaves the squad's (evidence only).
  await lab(page, (ids) => window.__lab.route.select(ids), [tank.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  const tankNear = [tank.position[0] - Math.cos(away) * 40, tank.position[1] - Math.sin(away) * 40];
  await frameAt(
    page,
    [(tank.position[0] + tankNear[0]) / 2, (tank.position[1] + tankNear[1]) / 2],
    CAMERA.default.distance,
    0.85,
    CAMERA.default.yaw,
  );
  await frames();
  css = await toCss(tankNear);
  await page.mouse.move(css[0], css[1]);
  await frames();
  r = await ruler();
  ctx.check("with the tank alone selected, the ruler runs from the tank", r?.unit === tank.id);
  await snapshot(ctx, page, "ruler-tank-near-1920x1080.png");

  // Far camera: the squad alone, the cursor 750 m off, past both its reaches.
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  // Toward the map's middle, so the far points stay on the map.
  const [mx, my] = village.map.size.map((v) => v / 2);
  const inward = (u) => Math.atan2(my - u.position[1], mx - u.position[0]);
  const far = [
    rifle.position[0] + Math.cos(inward(rifle)) * 750,
    rifle.position[1] + Math.sin(inward(rifle)) * 750,
  ];
  const farMid = [(rifle.position[0] + far[0]) / 2, (rifle.position[1] + far[1]) / 2];
  await frameAt(page, farMid, 1100, 0.85, CAMERA.default.yaw);
  await frames();
  css = await toCss(far);
  await page.mouse.move(css[0], css[1]);
  await frames();
  r = await ruler();
  want = await expected(rifle, far);
  text = await shownText();
  ctx.check(
    "past every reach, each weapon reads out of range and ticks the line where its reach ends",
    r?.unit === rifle.id &&
      Math.abs(r.distance_m - want) < 2 &&
      r.marks.length > 0 &&
      r.marks.every((m) => !m.inRange && m.along_m > 0 && m.along_m < 750) &&
      text.weapons.every((w) => !w.inRange) &&
      text.ticks.length === r.marks.length,
    JSON.stringify({ r, want, text }),
  );
  await snapshot(ctx, page, "ruler-far-1920x1080.png");
  // Both again, the cursor 1000 m out from the tank: the tank is nearer. Its
  // HMG falls short, its gun reaches.
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id, tank.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 2);
  const tankFar = [
    tank.position[0] + Math.cos(inward(tank)) * 1000,
    tank.position[1] + Math.sin(inward(tank)) * 1000,
  ];
  await frameAt(
    page,
    [(tank.position[0] + tankFar[0]) / 2, (tank.position[1] + tankFar[1]) / 2],
    1400,
    0.85,
    CAMERA.default.yaw,
  );
  await frames();
  css = await toCss(tankFar);
  if (css) await page.mouse.move(css[0], css[1]);
  await frames();
  r = await ruler();
  text = await shownText();
  ctx.check(
    "from the tank at 1000 m, the gun reaches and the HMG's reach ticks the line",
    r?.unit === tank.id &&
      r.marks.length === 2 &&
      !r.marks[0].inRange &&
      r.marks[1].inRange &&
      text.ticks.length === 1,
    JSON.stringify({ r, text }),
  );
  await snapshot(ctx, page, "ruler-tank-far-1920x1080.png");
  await page.keyboard.up("Space");
  await page.waitForFunction(() => !window.__lab.route.showOrders());
  await frames();
  ctx.check(
    "releasing Space removes the ruler",
    (await ruler()) === null && !(await shownText()).shown,
  );

  // A contact's area: right-click it with an armed unit selected.
  o = await until(page, (x) => x.contacts.length > 0, 30 * 180, 30);
  const contact = o?.contacts[0];
  if (contact) {
    const armed = o.own.find((u) => u.kind === "tank") ?? o.own.find((u) => u.kind === "rifle");
    await lab(page, (ids) => window.__lab.route.select(ids), [armed.id]);
    await page.waitForFunction(() => window.__lab.route.selected().length === 1);
    await frameAt(page, contact.center, 120, 0.85, CAMERA.default.yaw);
    await frames();
    const before = (await lab(page, () => window.__lab.route.acks())).length;
    css = await toCss(contact.center);
    await page.mouse.click(css[0], css[1], { button: "right" });
    await page.waitForFunction((n) => window.__lab.route.acks().length > n, before);
    const [ack] = await lab(page, () => window.__lab.route.acks());
    ctx.check(
      "right-clicking a contact's area attacks it",
      ack.ack.error === null && ack.label.startsWith(`attack area ${contact.id} `),
      JSON.stringify({ ack, contact }),
    );
    await snapshot(ctx, page, "contact-attack-1920x1080.png");
  }
  ctx.check("a contact appears to right-click", !!contact, `${o?.tick}`);
  await page.close();
}

/** The HUD's enemy red, as CSS computes it. */
const ENEMY_RGB = `rgb(${village.presentation.hud.enemy.map((v) => Math.round(v * 255)).join(", ")})`;

/** One panel as drawn: shown, its name, every state row (state, word, ring
 *  progress) and every weapon tag. `attr` is `unit`, `enemy` or `contact`. */
const panelOf = (page, attr, id) =>
  lab(
    page,
    ([a, k]) => {
      const n = document.querySelector(`[data-testid=readouts] .ro-unit[data-${a}="${k}"]`);
      if (!n) return null;
      const r = n.getBoundingClientRect();
      return {
        shown: n.style.display !== "none",
        box: { x: r.left, y: r.top, w: r.width, h: r.height },
        name: n.querySelector(".ro-name")?.textContent.trim() ?? null,
        states: [...n.querySelectorAll(".ro-state")].map((e) => ({
          state: e.dataset.state,
          word: e.querySelector(".ro-state-word").textContent,
          progress: e.dataset.progress,
        })),
        weapons: [...n.querySelectorAll(".ro-weapon")].map((e) => e.textContent.trim()),
        text: n.textContent,
        colour: getComputedStyle(n.querySelector(".ro-name") ?? n).color,
      };
    },
    [attr, id],
  );

/** A frame named `name`, and a 2× crop round the panel it shows. */
async function panelShot(ctx, page, attr, id, name) {
  const png = decode(await snapshot(ctx, page, `${name}-1920x1080.png`));
  const p = await panelOf(page, attr, id);
  if (p?.shown)
    await writeCrop(
      png,
      ctx.evidencePath(`${name}-crop-2x.png`),
      p.box.x + p.box.w / 2 - 20,
      p.box.y + p.box.h / 2 + 20,
      p.box.w / 2 + 70,
      p.box.h / 2 + 60,
      2,
    );
}

/** Ink of the ground marks (overlay over black, and the paint) within
 *  `radius` metres of `at` on screen, the panels hidden. */
async function marksNear(ctx, page, at, radius, name) {
  await page.evaluate(() => {
    document.querySelector("[data-testid=readouts]").style.visibility = "hidden";
  });
  await lab(page, () => window.__lab.setFrameView("overlays-on-black"));
  const over = decode(await snapshot(ctx, page, `${name}-overlay.png`));
  await lab(page, () => window.__lab.setFrameView("final"));
  const paint = await paintOnly(ctx, page, name);
  await page.evaluate(() => {
    document.querySelector("[data-testid=readouts]").style.visibility = "";
  });
  const c = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), at);
  const rim = await lab(
    page,
    ([p, r]) =>
      [0, 1, 2, 3].map((k) =>
        window.__lab.projectToCss(
          p[0] + r * Math.cos((k * Math.PI) / 2),
          p[1] + r * Math.sin((k * Math.PI) / 2),
          p[2],
        ),
      ),
    [at, radius],
  );
  const px = Math.max(...rim.map((q) => Math.hypot(q[0] - c[0], q[1] - c[1])));
  let overlay = 0,
    painted = 0;
  for (let y = Math.max(0, c[1] - px); y < Math.min(over.height, c[1] + px); y++)
    for (let x = Math.max(0, c[0] - px); x < Math.min(over.width, c[0] + px); x++) {
      if (Math.hypot(x - c[0], y - c[1]) > px) continue;
      const [r, g, b] = pixel(over, x, y);
      if (r + g + b >= 60) overlay++;
      const [pr, pg, pb] = pixel(paint, x, y);
      if (pr + pg + pb >= 60) painted++;
    }
  return { overlay, painted, px: Math.round(px) };
}

/** Every unit's info panel holds its states, as an icon and a short word;
 *  the ground keeps only selection and movement. Enemy panels are red and
 *  say only what blue can know. Shots of each panel at the default camera
 *  and one far, busy frame. */
async function panelTour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  const look = async (at, distance = CAMERA.default.distance) => {
    await frameAt(page, at, distance, 0.85, CAMERA.default.yaw);
    // The panels and fixed-width lines follow the new view a frame later.
    await lab(page, () => window.__lab.frame());
    await lab(page, () => window.__lab.frame());
  };
  let o = await obs(page);
  await lab(page, () => window.__lab.frame());
  const owned = await lab(page, () =>
    [...document.querySelectorAll("[data-testid=readouts] .ro-unit[data-owner=own]")].map((n) => [
      Number(n.dataset.unit),
      n.querySelector(".ro-name")?.textContent.trim() ?? null,
    ]),
  );
  const titles = new Map(owned);
  ctx.check(
    "every own unit has an info panel, armed or not, titled with its type's name",
    o.own.length > 0 &&
      o.own.every((u) => titles.get(u.id) === unitType(u.kind).name.toUpperCase()),
    JSON.stringify({ own: o.own.map((u) => [u.id, u.kind]), owned }),
  );

  // The supply truck: its stock always; deploying, a filling ring; then
  // DEPLOYED. Nothing of it on the ground.
  const truck = o.own.find((u) => u.kind === "supply");
  await look(truck.position);
  let p = await panelOf(page, "unit", truck.id);
  ctx.check(
    "the truck's panel shows its stock",
    !!p?.shown && p.states.some((s) => s.word === `SUPPLY ${truck.stock}`),
    JSON.stringify(p),
  );
  await panelShot(ctx, page, "unit", truck.id, "panel-truck-start");
  await lab(
    page,
    (id) => window.__lab.route.command({ kind: "set_deployment", units: [id], deployed: true }),
    truck.id,
  );
  await advance(page, 30 * 6);
  o = await obs(page);
  const deploying = o.own.find((u) => u.id === truck.id);
  await look(deploying.position);
  p = await panelOf(page, "unit", truck.id);
  const row = p?.states.find((s) => s.state === "deploying");
  ctx.check(
    "a deploying unit's panel carries DEPLOYING, its ring the published progress",
    !!row &&
      row.word === "DEPLOYING" &&
      Math.abs(Number(row.progress) - deploying.deployment.progress) < 1e-6 &&
      deploying.deployment.progress > 0 &&
      deploying.deployment.progress < 1,
    JSON.stringify({ row, deployment: deploying.deployment }),
  );
  await panelShot(ctx, page, "unit", truck.id, "panel-truck-deploying");
  const bare = await marksNear(ctx, page, deploying.position, 8, "truck-deploying-marks");
  // The control: selected, its marker is there to be seen.
  await lab(page, (id) => window.__lab.route.select([id]), truck.id);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  const marked = await marksNear(ctx, page, deploying.position, 8, "truck-selected-marks");
  await lab(page, () => window.__lab.route.select([]));
  ctx.check(
    "no deployment ring on the ground: an unselected deploying truck leaves no mark, where its selection marker shows",
    bare.overlay + bare.painted <= 8 && marked.overlay + marked.painted > 200,
    JSON.stringify({ bare, marked }),
  );
  await advance(page, 30 * 10);
  await look(deploying.position);
  p = await panelOf(page, "unit", truck.id);
  ctx.check(
    "a fully deployed unit's panel says DEPLOYED",
    p?.states.some((s) => s.state === "deployed" && s.word === "DEPLOYED"),
    JSON.stringify(p?.states),
  );
  await panelShot(ctx, page, "unit", truck.id, "panel-truck-deployed");

  // Space held: every truck's reach, beside the orders.
  await lab(page, () => window.__lab.frame());
  await look(deploying.position, 240);
  const reachOff = await marksNear(ctx, page, deploying.position, 95, "reach-nospace");
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders());
  await look(deploying.position, 240);
  const reachOn = await marksNear(ctx, page, deploying.position, 95, "reach-space");
  await snapshot(ctx, page, "space-truck-reach-1920x1080.png");
  await page.keyboard.up("Space");
  ctx.check(
    "holding Space shows every supply truck's reach",
    reachOn.painted - reachOff.painted > 500,
    JSON.stringify({ reachOff, reachOn }),
  );

  // A tank's and a squad's panels.
  const tank = o.own.find((u) => u.kind === "tank");
  await look(tank.position);
  await panelShot(ctx, page, "unit", tank.id, "panel-tank");
  const squad = o.own.find((u) => u.kind === "rifle");
  await look(squad.position);
  await panelShot(ctx, page, "unit", squad.id, "panel-squad");

  // Into the fight: blue attack-moves on the village.
  await lab(page, () => {
    const o = window.__lab.route.observation();
    window.__lab.route.command({
      kind: "attack_move",
      units: o.own.filter((u) => u.kind !== "supply").map((u) => u.id),
      gesture: 1,
      goal: [1000, 800],
    });
  });
  // Each subject is checked and shot on the first published tick it appears
  // (the tour's start tick varies with load, so the fight's does too).
  const enemyPanelCheck = async (o) => {
    const enemy = o.identified[0];
    if (!enemy) return false;
    await look(enemy.position);
    const p = await panelOf(page, "enemy", enemy.id);
    ctx.check(
      "an identified enemy's panel is red and names its type and weapon types, never a count",
      !!p?.shown &&
        p.name === unitType(enemy.kind).name.toUpperCase() &&
        p.weapons.length === unitType(enemy.kind).mounts.length &&
        !/\d/.test(p.text) &&
        p.colour === ENEMY_RGB,
      JSON.stringify(p),
    );
    await panelShot(ctx, page, "enemy", enemy.id, "panel-enemy");
    return true;
  };
  // A last sighting and a firing report: what was known, and how long ago.
  const contactPanelCheck = (source, word, file) => async (o) => {
    const c = o.contacts.find((x) => x.source === source);
    if (!c) return false;
    await look([c.center[0], c.center[1], 0]);
    const p = await panelOf(page, "contact", c.id);
    const ago = Number(p?.states.at(-1)?.word.match(word)?.[1]);
    const expected = Math.floor((o.tick - c.evidenceTick) / village.tick_hz);
    const named =
      source === "last_seen"
        ? !!c.kind && p?.name === unitType(c.kind).name.toUpperCase()
        : p?.name === "UNKNOWN" && p.weapons.length === new Set(p.weapons).size;
    ctx.check(
      `a ${source} contact's panel is red, names what was known, and says how long ago`,
      !!p?.shown && named && ago === expected && p.colour === ENEMY_RGB,
      JSON.stringify({ p, expected, contact: c }),
    );
    await panelShot(ctx, page, "contact", c.id, file);
    return true;
  };
  // A suppressed own squad's row.
  const suppressedPanelCheck = async (o) => {
    const pinned = o.own
      .filter((u) => u.suppression >= 0.2)
      .sort((a, b) => b.suppression - a.suppression)[0];
    if (!pinned) return false;
    await look(pinned.position);
    const p = await panelOf(page, "unit", pinned.id);
    const row = p?.states.find((s) => s.state === "suppressed" || s.state === "pinned");
    const expected =
      pinned.suppression >= village.suppression.collapse_level ? "pinned" : "suppressed";
    ctx.check(
      "a suppressed squad's panel says SUPPRESSED or, past the collapse level, PINNED, with no level meter",
      row?.state === expected && row.progress === "",
      JSON.stringify({ row, suppression: pinned.suppression }),
    );
    await panelShot(ctx, page, "unit", pinned.id, "panel-suppressed");
    return true;
  };
  const pending = new Map([
    ["an identified enemy", enemyPanelCheck],
    [
      "a last sighting",
      contactPanelCheck("last_seen", /^LAST SEEN (\d+) s AGO$/, "panel-last-seen"),
    ],
    ["a firing report", contactPanelCheck("firing", /^HEARD (\d+) s AGO$/, "panel-heard")],
    ["a suppressed squad", suppressedPanelCheck],
  ]);
  for (let t = 0; t < 30 * 240 && pending.size; t += 15) {
    await advance(page, 15);
    o = await obs(page);
    for (const [name, check] of [...pending]) if (await check(o)) pending.delete(name);
  }
  ctx.check(
    "the fight brings an enemy, a last sighting, a report and a suppressed squad",
    pending.size === 0,
    JSON.stringify([...pending.keys()]),
  );

  // Far and busy: only the selection's panels stay.
  const pick = o.own
    .filter((u) => u.kind !== "supply")
    .slice(0, 2)
    .map((u) => u.id);
  await lab(page, (ids) => window.__lab.route.select(ids), pick);
  await page.waitForFunction((n) => window.__lab.route.selected().length === n, pick.length);
  await look(o.own.find((u) => u.id === pick[0]).position, 1150);
  const far = await lab(page, () =>
    [...document.querySelectorAll("[data-testid=readouts] .ro-unit")]
      .filter((n) => n.style.display !== "none")
      .map((n) => [
        n.dataset.owner,
        Number(n.dataset.unit ?? -1),
        n.querySelector(".ro-name")?.textContent.trim() ?? null,
        // The compact form: no row's words drawn.
        [...n.querySelectorAll(".ro-word")].every((w) => getComputedStyle(w).display === "none"),
      ]),
  );
  ctx.check(
    "zoomed far out, only the selected units' panels show, each its name over its compact rows",
    far.length > 0 &&
      far.every(
        ([owner, id, name, compact]) => owner === "own" && pick.includes(id) && !!name && compact,
      ),
    JSON.stringify({ far, pick }),
  );
  // Stacked panels keep the fixture's gap: a panel above another never
  // touches it.
  const stacked = await lab(page, () =>
    [...document.querySelectorAll("[data-testid=readouts] .ro-unit")]
      .filter((n) => n.style.display !== "none")
      .map((n) => {
        const r = n.getBoundingClientRect();
        return { x0: r.left, x1: r.right, y0: r.top, y1: r.bottom };
      }),
  );
  const gaps = stacked.flatMap((a, i) =>
    stacked
      .slice(i + 1)
      .filter((b) => a.x0 < b.x1 && b.x0 < a.x1)
      .map((b) => Math.max(b.y0 - a.y1, a.y0 - b.y1)),
  );
  const wantGap = village.presentation.hud.panel_gap_px;
  ctx.check(
    `stacked panels keep at least the fixture's ${wantGap} px between them`,
    gaps.length > 0 && gaps.every((g) => g >= wantGap - 0.5),
    JSON.stringify({ gaps, stacked }),
  );
  await snapshot(ctx, page, "panels-far-1920x1080.png");
  await page.close();
}

const TOURS = {
  panels: panelTour,
  ruler: rulerTour,
  selection: selectionTour,
  orders: orderTour,
  orderFlash: orderFlashTour,
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
  // destination carries no text (its marker and route say
  // whose it is).
  const tags = await lab(page, () => ({
    names: [...document.querySelectorAll(".ro-unit.ro-selected .ro-name")].map((n) => ({
      unit: Number(n.closest(".ro-unit").dataset.unit),
      text: n.textContent.trim(),
    })),
    // The name's cell also holds the role symbol, so compare its text
    // trimmed; both are drawn in capitals, so compare them so.
    panel: [...document.querySelectorAll("[data-testid=selection-panel] [data-unit] strong")].map(
      (n) => n.textContent.trim().toUpperCase(),
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
