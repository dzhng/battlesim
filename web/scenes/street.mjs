import { playTour } from "./_playTour.mjs";
import { smokeTour } from "./_smokeTour.mjs";
import { rulerTour } from "./_rulerTour.mjs";
import { stageStreetMoves } from "./_streetStaging.mjs";
// Production street controls, deterministic staging, models and effects.
// Optional GPU cost controls run alone under the GPU lock.
import { battleCursor } from "./_cursorOrders.mjs";
import { appJourney, armyJourney } from "./_appJourney.mjs";
import {
  hideHud,
  lab,
  obs,
  advance,
  until,
  snapshot,
  presented,
  openMenu,
  closeMenu,
  openBattle,
  aim,
  groundCss,
  pointerOffCanvas,
} from "./_lab.mjs";
import { anyNear, decode, pixel, writeCrop } from "./_png.mjs";
import { orderPaint, paintHue, paintOnly } from "./_overlays.mjs";
import { cleanupTour, woodsTour } from "./_battleLook.mjs";
import { bareClassMask, classAt } from "./_groundStations.mjs";
import {
  hasRole,
  hull as hullOf,
  isVehicle,
  unitType,
  vehicleAppearances,
  rules,
  streetAttack,
  streetMap,
  visuals,
} from "./_units.mjs";

const CAMERA = visuals.camera;
const roadStrokes = streetMap.surfaces
  .filter((s) => s.shape.kind === "stroke" && s.kind !== "paving") // carriageways
  .map((s) => s.shape);
/** The tour's fixed tick: every framing shows the same battle state. */
const TOUR_TICK = 90;

async function shot(ctx, page, name) {
  await snapshot(ctx, page, `frame-${name}.png`);
}

/** The road is drawn where the simulation has it. Top
 *  down over the first road's longest straight run, ground a metre inside
 *  its edge reads as road and ground a metre and a half outside reads as
 *  verge. */
async function checkRoadEdges(ctx, page) {
  const road = roadStrokes[0];
  const runs = road.points.slice(1).map((b, i) => [road.points[i], b]);
  const length = ([a, b]) => Math.hypot(b[0] - a[0], b[1] - a[1]);
  const [[ax, ay], [bx, by]] = runs.reduce((best, run) =>
    length(run) > length(best) ? run : best,
  );
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
  const mask = await bareClassMask(ctx, page, "road-edges-classes.png");
  const classified = [];
  for (const offset of [0, half - 1, -(half - 1), half + 1.5, -(half + 1.5)]) {
    const point = at(offset);
    const css = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], 0), point);
    classified.push({ point, css, ground: classAt(mask, ...css.map(Math.round)) });
  }
  ctx.check(
    "the road is drawn where the simulation has it: road inside its edge, verge outside",
    classified.slice(0, 3).every((p) => p.ground?.roadSd < 0) &&
      classified.slice(3).every((p) => p.ground?.roadSd > 0),
    JSON.stringify(classified),
  );
  await ctx.writeEvidence("road-edges.json", {
    classified,
    colourDiagnostic: {
      centre,
      inside,
      outside,
      historicalTargetMet:
        inside.every((g) => g < centre + 6) && outside.every((g) => g > centre + 12),
    },
  });
}

/** The middle of `v`: what a paired cost run reports of its batches. */
const median = (v) => [...v].sort((a, b) => a - b)[Math.floor(v.length / 2)];

/** The grass field, read back at a framing. */
const grassClumps = (page) => lab(page, () => window.__lab.grass().clumps());
const grassCounts = (page) => lab(page, () => window.__lab.grass().counts());

/** How far `p` lies outside the road network (negative on a road). A road
 *  ends square: ground past its first or last point is not that run's. */
function offRoad(p) {
  let best = Infinity;
  for (const road of roadStrokes) {
    for (let k = 1; k < road.points.length; k++) {
      const [a, b] = [road.points[k - 1], road.points[k]];
      const ab = [b[0] - a[0], b[1] - a[1]];
      const along = ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / (ab[0] ** 2 + ab[1] ** 2);
      if ((k === 1 && along < 0) || (k === road.points.length - 1 && along > 1)) continue;
      const t = Math.max(0, Math.min(1, along));
      const off = Math.hypot(p[0] - a[0] - ab[0] * t, p[1] - a[1] - ab[1] * t);
      best = Math.min(best, off - road.width_m / 2);
    }
  }
  return best;
}

const insideForest = (point, forest) => {
  const [a, , c] = forest.shape.ring;
  // These are the fixed rectangular street map stimuli, not renderer geometry.
  return point[0] > a[0] && point[0] < c[0] && point[1] > a[1] && point[1] < c[1];
};
const underProp = (p) =>
  [...streetMap.props, ...streetMap.buildings.flatMap((b) => b.geometry.parts)].some(
    ({ center: [cx, cy], yaw, half_extents: [hx, hy] }) => {
      const [dx, dy] = [p[0] - cx, p[1] - cy];
      const [c, s] = [Math.cos(yaw), Math.sin(yaw)];
      return Math.abs(dx * c + dy * s) < hx && Math.abs(-dx * s + dy * c) < hy;
    },
  );

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
      streetMap.forests.some((f) => insideForest(c.root, f)),
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
  const page = await openBattle(ctx, {
    viewport: { width: 1920, height: 1080 },
    tick: TOUR_TICK,
    grass: true,
  });
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
      opening.target[0] === 170 &&
      opening.target[1] === 800,
    JSON.stringify(opening),
  );
  await tourShot("default");
  await checkGrass(ctx, page, "default");
  await checkGrassResidency(ctx, page);
  await wheel(400);
  const far = await camera();
  ctx.check(
    "wheeling out stops at the strategic height, at the curve's end",
    far.distance === CAMERA.zoom_max && highest[0] === CAMERA.zoom_max && far.pitch >= highest[1],
    JSON.stringify(far),
  );
  await tourShot("strategic");
  await checkRoadEdges(ctx, page);

  await lab(page, () => window.__lab.reset());
  await wheel(-400);
  const near = await camera();
  // The wheel carries pitch along the curve, keeping any tilt off it (the
  // opening's own: it looks a little steeper, so the entry reads): one tilt
  // at both of the curve's ends.
  ctx.check(
    "wheeling in stops at ground level, the same tilt off the curve as at the strategic height, the target on the ground",
    near.distance === CAMERA.zoom_min &&
      lowest[0] === CAMERA.zoom_min &&
      Math.abs(near.pitch - lowest[1] - (far.pitch - highest[1])) < 1e-9 &&
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
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick: TOUR_TICK });
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

  // Shared selector tests pin per-cascade identities. Here, check visible
  // detail, population culling and independent caster submissions.
  const drawn = (p) => p.tiers.reduce((a, b) => a + b, 0);
  const finest = (p) => p.tiers.findIndex((n) => n > 0);
  ctx.check(
    "visible trees get finer nearby, backdrop is culled, and forest casters are submitted",
    Object.values(seen).every(
      (s) => s.forest.placed > 400 && drawn(s.forest) > 100 && s.forest.casters > 0,
    ) &&
      drawn(seen.road.forest) > 0.9 * seen.road.forest.placed &&
      drawn(seen.edge.forest) < 0.7 * seen.edge.forest.placed &&
      seen.road.backdrop.placed > 2000 &&
      drawn(seen.road.backdrop) > 0 &&
      drawn(seen.top.backdrop) < drawn(seen.road.backdrop) &&
      finest(seen.ground.forest) < finest(seen.road.forest),
    JSON.stringify(seen),
  );
  // The world only: the HUD's full-width bars cover the frame's
  // top and bottom edges.
  const hud = await hideHud(page);
  const top = decode(await snapshot(ctx, page, "trees-top-check-1920x1080.png"));
  await hud.evaluate((e) => e.remove());
  const [x0, y0] = streetMap.forests[0].shape.ring[0];
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

async function soldierTour(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick: TOUR_TICK });
  const o = await obs(page);
  const squads = o.own.filter((u) => u.members.length > 0);
  const soldiers = squads.reduce((n, u) => n + u.members.length, 0);
  // The squad nearest the opening framing's target.
  const [tx, ty] = [170, 800];
  const squad = squads.reduce((a, b) =>
    Math.hypot(a.position[0] - tx, a.position[1] - ty) <=
    Math.hypot(b.position[0] - tx, b.position[1] - ty)
      ? a
      : b,
  );
  const seen = {};
  for (const [name, framing] of Object.entries(SQUAD_FRAMINGS)) {
    await aim(page, squad.position, { ...framing, yaw: -Math.PI / 2 });
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
      finest(seen.ground) >= 0 &&
      finest(seen.default) > finest(seen.ground) &&
      seen.far.cards === soldiers,
    JSON.stringify({ soldiers, seen }),
  );

  // Units are drawn by identification: never fogged. In the fog mask a
  // soldier is white wherever the camera sees him, his back to every eye included.
  await aim(page, squad.position, { ...SQUAD_FRAMINGS.ground, yaw: -Math.PI / 2 });
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
  // The frame's pick records are f32, so his half width is the radius's f32.
  await aim(page, squad.position, { ...SQUAD_FRAMINGS.default, yaw: -Math.PI / 2 });
  await page.evaluate(() => window.__lab.frame());
  const m = squad.members[0];
  const torso = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1), m);
  const picked = await lab(page, (p) => window.__lab.pickAt(p[0], p[1]), torso);
  const box = await lab(page, (k) => window.__lab.instances()[k], picked);
  ctx.check(
    "clicking a soldier picks his box",
    picked >= 0 &&
      box.half[0] === Math.fround(rules.physics.soldier_radius_m) &&
      Math.hypot(box.x - m[0], box.y - m[1]) < 1.5,
    JSON.stringify({ picked, box, member: m }),
  );

  // Infantry beside armour during a real advance. The deterministic death
  // lifecycle is exercised by the consequences scene.
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
    const tanks = o.own.filter((u) => u.kind === "test_tank");
    const near = (u) =>
      Math.min(
        ...tanks.map((t) =>
          Math.hypot(t.position[0] - u.position[0], t.position[1] - u.position[1]),
        ),
      );
    const squad = o.own.filter((u) => u.members.length > 0).sort((a, b) => near(a) - near(b))[0];
    if (!squad) return;
    await aim(page, squad.position, { ...SQUAD_FRAMINGS.default, yaw: -Math.PI / 2 });
    await snapshot(ctx, page, `soldiers-${name}-1920x1080.png`);
    await lab(page, () => window.__lab.setFrameView("world"));
    await snapshot(ctx, page, `soldiers-${name}-world-1920x1080.png`);
    await lab(page, () => window.__lab.setFrameView("final"));
  };
  // Eight seconds into the advance.
  await advance(page, 240);
  await besideTank("advance");
  await page.close();
}

const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));

/** Vehicles and wrecks are appearances and buildings their templates' art, placed and
 *  articulated from what the side knows. */
async function vehicleTour(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick: TOUR_TICK });
  await lab(page, () => window.__lab.frame());
  let o = await obs(page);
  let posed = await lab(page, () => window.__lab.route.vehicles());
  const stats = await lab(page, () => window.__lab.stats());
  const proxies = stats.models.proxyAppearances;
  const own = o.own.filter(isVehicle);
  const enemy = o.identified.filter(isVehicle);
  ctx.check(
    "every own and identified vehicle is drawn as its appearance, and nothing as a proxy",
    posed.length === own.length + enemy.length &&
      posed.every((v) => v.articulation && vehicleAppearances.has(v.appearance)) &&
      proxies === 0,
    JSON.stringify({
      posed: posed.length,
      own: own.length,
      enemy: enemy.length,
      proxies,
    }),
  );
  // Each own tank's turret is on its cannon's published bearing, relative to the hull.
  const turrets = own
    .filter((u) => u.kind === "test_tank")
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
  // The houses are buildings like any town's: each its template's rows in the
  // art library, on its own box, and no fitted appearance stands for one.
  // The drawn box is published in float32, so the map's extents are compared
  // as float32 too.
  const houses = streetMap.buildings.flatMap((b) => b.geometry.parts);
  const buildings = await lab(page, () => window.__lab.route.buildings());
  const fitted = (await lab(page, () => window.__lab.route.structures())).filter((s) =>
    houses.some((h) => s.position[0] === h.center[0] && s.position[1] === h.center[1]),
  );
  const drawn = await lab(page, () => window.__lab.stats().buildings);
  ctx.check(
    "the street's houses stand as their templates' art, each on its box",
    buildings.length === houses.length &&
      buildings.every(
        (b, i) =>
          !b.fallen &&
          b.frame[0] === houses[i].center[0] &&
          b.frame[1] === houses[i].center[1] &&
          b.parts.length === 1 &&
          b.parts[0].half.length === 3 &&
          b.parts[0].half.every((h, k) => h === Math.fround(houses[i].half_extents[k])),
      ) &&
      fitted.length === 0 &&
      drawn.buildings === houses.length &&
      drawn.fallen === 0,
    JSON.stringify({ buildings, fitted, drawn }),
  );
  const tank = own.find((u) => u.kind === "test_tank");
  // The nearest tank from its right front, as WARNO frames its nearest tank.
  await aim(page, tank.position, { distance: 20, pitch: 0.42, yaw: tank.yaw - Math.PI * 0.3 });
  await snapshot(ctx, page, "vehicles-tank-1920x1080.png");
  await aim(page, tank.position, { distance: 65, pitch: 0.85, yaw: -1.57 });
  await snapshot(ctx, page, "vehicles-default-1920x1080.png");
  // The street's farms from the western approach.
  await aim(page, [1000, 812], { distance: 150, pitch: 0.55, yaw: 0 });
  await snapshot(ctx, page, "vehicles-street-1920x1080.png");

  // Drive and fire: the tanks attack-move on the street; run on until an own
  // tank's cannon fires, then look at it on the tick of the shot.
  await lab(
    page,
    (units) =>
      window.__lab.route.command({ kind: "attack_move", units, gesture: 1, goal: [900, 800] }),
    own.filter((u) => u.kind === "test_tank").map((u) => u.id),
  );
  const shots = (f) =>
    Object.fromEntries(
      f.own.filter((u) => u.kind === "test_tank").map((u) => [u.id, u.weaponPoses[0]?.shots ?? 0]),
    );
  const start = shots(o);
  o = await until(
    page,
    (f) =>
      f.own.some(
        (u) => u.kind === "test_tank" && (u.weaponPoses[0]?.shots ?? 0) > (start[u.id] ?? 0),
      ),
    30 * 240,
    1,
  );
  const shooter = o?.own.find(
    (u) => u.kind === "test_tank" && (u.weaponPoses[0]?.shots ?? 0) > (start[u.id] ?? 0),
  );
  if (shooter) {
    await aim(page, shooter.position, {
      distance: 26,
      pitch: 0.35,
      yaw: shooter.weaponPoses[0].bearing + Math.PI * 0.6,
    });
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

/** A real ordered ground burst, framed at its moment and as it grows,
 *  drawn from the side's publications independently of assault balance. */
async function effectTour(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick: 30 });
  const tank = (await obs(page)).own.find((u) => u.kind === "test_tank");
  const ack = await lab(
    page,
    (id) =>
      window.__lab.route.command({
        kind: "attack",
        units: [id],
        target: { kind: "ground", point: [300, 820, 0] },
      }),
    tank.id,
  );
  ctx.check("the tank's ordered ground burst is admitted", ack.error === null, JSON.stringify(ack));
  const o = await until(page, (f) => f.blasts.length > 0, 600, 1);
  if (!o) {
    ctx.check("a published burst is drawn as a fireball", false, "no blast within 600 ticks");
    await page.close();
    return;
  }
  const burst = o.blasts[0].point;
  await aim(page, burst, { distance: 70, pitch: 0.55, yaw: -1.2 });
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
    "a published burst is drawn as a fireball where it was published",
    fire > 40 && stats.frame.instances > 0 && stats.effects.dropped === 0,
    JSON.stringify({ tick: o.tick, burst, firePixels: fire, ...stats }),
  );

  // The live battle is heard once the player first clicks, and a
  // pause silences its transients while its loops hold.
  await page.click('[data-testid="battle-panel"] header');
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

/** The paint's ink, by hue: the orders' yellow, and the cover ramp's white
 *  (light), light green (medium) and strong green (heavy). */
function overlayInk(png) {
  const ink = { lit: 0, yellow: 0, amber: 0, medium: 0, heavy: 0, other: 0 };
  for (let i = 0; i < png.data.length; i += 4) {
    const hue = paintHue([png.data[i], png.data[i + 1], png.data[i + 2]]);
    if (!hue) continue;
    ink.lit++;
    ink[hue]++;
  }
  return ink;
}

/** The orders' yellow in the paint, summed by its red channel: how much
 *  order ink there is and how strongly it is drawn. */
function yellowInk(png) {
  let ink = 0;
  for (let i = 0; i < png.data.length; i += 4) {
    const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
    if (r > 20 && r > b + 12 && g > 0.7 * r) ink += r;
  }
  return ink;
}

/** An order's flash uses the Space view's marks, shown as the order is given,
 *  held for
 *  `orders.flash.hold_s` and faded over `fade_s`; a selection alone shows
 *  none. Holding Space shows the same marks, pixel for pixel. A queued
 *  (Shift) order flashes too. */
async function orderFlashTour(ctx) {
  const { flash } = visuals.overlay.orders;
  const tickHz = rules.tick_hz;
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 } });
  const o = await obs(page);
  const rifle = o.own.find((u) => u.kind === "test_rifle");
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id]);
  await page.waitForFunction((id) => window.__lab.route.selected()[0] === id, rifle.id);
  const goal = [rifle.position[0] + 30, rifle.position[1]];
  const at = [goal[0] - 12, goal[1]];
  const views = {
    default: { distance: CAMERA.default.distance, pitch: 0.85 },
    far: { distance: 250, pitch: 0.85 },
  };
  const frame = async (view) => {
    await aim(page, at, {
      distance: views[view].distance,
      pitch: views[view].pitch,
      yaw: CAMERA.default.yaw,
    });
    // Lines keep their width on screen: rebuilt a frame after a zoom step.
    await lab(page, () => window.__lab.frame());
    await lab(page, () => window.__lab.frame());
  };
  // The order marks, read in the paint.
  const overlay = (name) => orderPaint(ctx, page, name);
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
  const press = await groundCss(page, goal);
  await page.mouse.click(press[0], press[1], { button: "right" });
  await page.waitForFunction(() => window.__lab.route.acks().length > 0);
  // Off the canvas: with Space held the range ruler (paint too) stays away.
  await pointerOffCanvas(page);
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
    if (!(r > 60 && r > b + 25 && g > 0.7 * r)) continue;
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
  const next = await groundCss(page, [goal[0], goal[1] + 15]);
  const acks = await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);
  await page.keyboard.down("Shift");
  await page.mouse.click(next[0], next[1], { button: "right" });
  await page.keyboard.up("Shift");
  await page.waitForFunction((n) => (window.__lab.route.acks()[0]?.seq ?? 0) > n, acks);
  await advance(page, 2);
  const queued = yellowInk(await overlay("order-flash-queued"));
  ctx.check(
    "a queued order flashes its unit's marks too",
    quiet < selected + 0.02 * (issued - selected) && queued > 0.8 * issued,
    JSON.stringify({ selected, issued, quiet, queued }),
  );
  await page.close();
}

/** Held group destinations use the same individual markers as confirmation. */
async function groupPreviewTour(ctx, rotate = true) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 } });
  const before = await obs(page);
  const tank = before.own.find((u) => u.kind === "test_tank");
  const rifle = before.own.find((u) => u.kind === "test_rifle");
  const ids = [tank.id, rifle.id];
  await lab(page, (ids) => window.__lab.route.select(ids), ids);
  await page.waitForFunction(
    (ids) => JSON.stringify(window.__lab.route.selected()) === JSON.stringify(ids),
    ids,
  );
  const goal = [tank.position[0] + 55, tank.position[1] + 15];
  await aim(page, goal, { distance: 220, pitch: 0.85, yaw: CAMERA.default.yaw });
  await lab(page, () => window.__lab.frame());
  const press = await groundCss(page, goal);
  const n = await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);
  const to = await groundCss(page, [goal[0], goal[1] + 20]);
  await page.mouse.move(...press);
  await page.mouse.down({ button: "right" });
  if (rotate) await page.mouse.move(...to, { steps: 6 });
  await page.waitForFunction((rotate) => {
    const p = window.__lab.route.movePreview();
    const marks = p;
    return marks.length && (!rotate || marks.every((m) => Math.abs(m.facing - Math.PI / 2) < 0.05));
  }, rotate);
  const label = rotate ? "group-preview" : "group-preview-plain";
  ctx.check(
    "pressing and orienting a move never issues an order",
    (await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0)) === n,
  );
  await shot(ctx, page, `${label}-held`);
  const raw = await lab(page, () => window.__lab.route.movePreview());
  const marks = raw;
  ctx.check(
    "held group move draws one distinct destination per selected unit",
    marks.length === ids.length &&
      ids.every((id) => marks.some((m) => m.unit === id)) &&
      Math.hypot(marks[0].c[0] - marks[1].c[0], marks[0].c[1] - marks[1].c[1]) > 5,
    JSON.stringify(marks),
  );
  if (marks.length !== ids.length) {
    await page.mouse.up({ button: "right" });
    await page.close();
    return;
  }
  await page.mouse.up({ button: "right" });
  await page.waitForFunction((n) => (window.__lab.route.acks()[0]?.seq ?? 0) > n, n);
  await page.waitForFunction(() => window.__lab.route.movePreview().length > 0);
  ctx.check(
    "released markers remain until the committed publication takes over",
    (await lab(page, () => window.__lab.route.movePreview())).length === ids.length,
  );
  await shot(ctx, page, `${label}-released`);
  await advance(page, 2);
  const committed = (await obs(page)).own.filter((u) => ids.includes(u.id));
  ctx.check(
    "held markers match each committed destination and facing",
    marks.length === ids.length &&
      marks.every((m) => {
        const u = committed.find((u) => u.id === m.unit);
        const c = u?.area?.anchor ?? u?.goal;
        return (
          c &&
          Math.hypot(m.c[0] - c[0], m.c[1] - c[1]) < 0.01 &&
          Math.abs(wrap(m.facing - u.finalFacing)) < 0.01
        );
      }),
    JSON.stringify({
      marks,
      committed: committed.map((u) => ({
        id: u.id,
        goal: u.goal,
        area: u.area,
        facing: u.finalFacing,
      })),
    }),
  );
  await page.waitForFunction(() => window.__lab.route.movePreview().length === 0);
  await shot(ctx, page, `${label}-committed`);
  await page.close();
}

/** Total War markers (D2), the Space overlay (D2+), right-drag
 *  facing (Q9) and a reverse move's marker (Q31), own units only. */
async function orderTour(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 } });
  let o = await obs(page);
  const rifle = o.own.find((u) => u.kind === "test_rifle");
  const tank = o.own.find((u) => u.kind === "test_tank");

  // A real right-drag: press at the goal, release north-east of it.
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id]);
  await page.waitForFunction((id) => window.__lab.route.selected()[0] === id, rifle.id);
  const goal = [rifle.position[0] + 30, rifle.position[1]];
  const at = [goal[0] - 12, goal[1]];
  await aim(page, at, { distance: CAMERA.default.distance, pitch: 0.85, yaw: CAMERA.default.yaw });
  await lab(page, () => window.__lab.frame());
  const press = await groundCss(page, goal);
  const release = await groundCss(page, [goal[0] + 10, goal[1] + 10]);
  await page.mouse.move(press[0], press[1]);
  await page.mouse.down({ button: "right" });
  await page.waitForFunction(() => window.__lab.route.movePreview().length > 0);
  await page.mouse.move(release[0], release[1], { steps: 6 });
  await page.waitForFunction(
    () => Math.abs(window.__lab.route.movePreview()[0]?.facing - Math.PI / 4) < 0.25,
  );
  const [preview] = await lab(page, () => window.__lab.route.movePreview());
  ctx.check(
    "the held facing marker keeps the press destination while its arrow turns",
    Math.hypot(preview.c[0] - goal[0], preview.c[1] - goal[1]) < 1,
    JSON.stringify(preview),
  );
  await shot(ctx, page, "held-facing");
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
  // The order marks, read in the paint.
  const ordersOnly = (name) => orderPaint(ctx, page, name);
  const without = overlayInk(await ordersOnly("orders-default-nospace"));
  // Off the canvas: with Space held the range ruler (paint too) stays away.
  await pointerOffCanvas(page);
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders());
  await lab(page, () => window.__lab.frame());
  for (const [name, view] of Object.entries(cameras)) {
    await aim(page, at, { distance: view.distance, pitch: view.pitch, yaw: CAMERA.default.yaw });
    // Lines keep their width on screen: the overlay is rebuilt for the new
    // zoom a frame after the camera moves.
    await lab(page, () => window.__lab.frame());
    await lab(page, () => window.__lab.frame());
    await snapshot(ctx, page, `orders-space-${name}-1920x1080.png`);
  }
  // Into the fog: the other squad sent to ground blue cannot see.
  const other = o.own.find((u) => u.kind === "test_rifle" && u.id !== rifle.id);
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
    await aim(page, fogged, {
      distance: cameras.default.distance,
      pitch: cameras.default.pitch,
      yaw: CAMERA.default.yaw,
    });
    await snapshot(ctx, page, "orders-space-fog-1920x1080.png");
  }
  ctx.check("a squad's markers are framed in the fog", !!fogged, JSON.stringify(fogged));
  await aim(page, at, {
    distance: cameras.default.distance,
    pitch: cameras.default.pitch,
    yaw: CAMERA.default.yaw,
  });
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
  // Light cover's pip is the orders' yellow.
  const tierOf = (c) => {
    const hue = paintHue(c);
    return hue === "yellow" ? "light" : hue;
  };
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
        const p = await groundCss(page, [q[0], q[1]]);
        if (!p || p[0] < 8 || p[1] < 8 || p[0] > 1912 || p[1] > 1072) continue;
        pips.checked++;
        // A soldier without cover shows no cover green there (a ring or
        // route of another unit may cross his marker by chance: paint hides
        // under no hull).
        const seen = tierOf(middle(p));
        const ok = tier ? seen === tier : !["medium", "heavy"].includes(seen);
        if (ok) pips.right++;
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
  const isRing = ([r, g, b]) => r > 30 && g > 0.7 * r && b < 0.6 * r && r > g * 0.9;
  const isPip = (c) => ["medium", "heavy"].includes(tierOf(c));
  for (const u of o.own.filter((v) => v.members.length > 0))
    for (const [k, m] of u.memberOrders.entries())
      for (const [which, q, tier] of [
        ["now", u.members[k], m.coverNow],
        ...(u.goal ? [["there", m.spot, m.coverThere]] : []),
      ]) {
        if (!q || !["medium", "heavy"].includes(tier) || centred[which].length >= 4) continue;
        const p = await groundCss(page, [q[0], q[1]]);
        if (!p || p[0] < 20 || p[1] < 20 || p[0] > 1900 || p[1] > 1060) continue;
        // The marker's radius on screen, so only its own ring (an annulus
        // round the soldier) and its own pip (inside it) count, not a route
        // or a ring crossing near it.
        const edge = await groundCss(page, [q[0] + 0.45, q[1]]);
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
            if (!into) continue;
            into[0] += x;
            into[1] += y;
            into[2]++;
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
    const p = await groundCss(page, [(u.position[0] + x) / 2, (u.position[1] + y) / 2]);
    if (p && p[0] > 4 && p[1] > 4 && p[0] < 1916 && p[1] < 1076) inView.push(p);
  }
  const inkedAt = (p) => anyNear(inked, p, 3, ([r, g, b]) => r + g + b >= 30);
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
    const a = await groundCss(page, [u.position[0], u.position[1]]);
    const b = await groundCss(page, [x, y]);
    if (!a || !b) continue;
    const n = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
    const across = [-(b[1] - a[1]) / n, (b[0] - a[0]) / n];
    // The route runs from the edge of the circle the squad stands in to the
    // edge of its area ring (both drawn at the fixture's scale):
    // sample only the line between them.
    const scale = visuals.overlay.orders.area_draw_scale;
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
      const p = await groundCss(page, q);
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
  await checkSoldiersNotOverdrawn(ctx, page);
  await aim(page, at, {
    distance: cameras.default.distance,
    pitch: cameras.default.pitch,
    yaw: CAMERA.default.yaw,
  });
  await page.keyboard.up("Space");
  await page.waitForFunction(() => !window.__lab.route.showOrders());
  ctx.check("releasing Space hides the overlay again", true);
  // The selection's callouts and markers without Space (evidence): no
  // order marks, since no order was just given.
  await lab(page, (ids) => window.__lab.route.select(ids), [rifle.id, tank.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 2);
  await aim(page, at, {
    distance: cameras.default.distance,
    pitch: cameras.default.pitch,
    yaw: CAMERA.default.yaw,
  });
  await lab(page, () => window.__lab.frame());
  await snapshot(ctx, page, "orders-selected-default-1920x1080.png");
  // Separate the destination ring only for route clipping; the preceding
  // cover checks keep their covered destination.
  await lab(
    page,
    (c) =>
      window.__lab.route.command({
        kind: "move",
        units: [c.id],
        gesture: 3503,
        goal: c.goal,
        route: "shortest",
        facing: c.facing,
      }),
    {
      id: rifle.id,
      goal: [goal[0], goal[1] + (tank.position[1] >= rifle.position[1] ? -10 : 10)],
      facing: wanted,
    },
  );
  await advance(page, 3);
  // The selection's markers against its orders: Space held, so they show
  // (the pointer off the canvas, so the range ruler doesn't).
  await pointerOffCanvas(page);
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders());
  await checkSelectionYellow(ctx, page, rifle.id, tank.id);
  await checkRimJoin(ctx, page, rifle.id);
  await page.keyboard.up("Space");
  await page.waitForFunction(() => !window.__lab.route.showOrders());
  await checkPaintedLight(ctx, page, tank.id);
  // The pause menu, open (evidence for the chrome).
  await openMenu(page);
  await snapshot(ctx, page, "orders-pause-menu-1920x1080.png");
  await closeMenu(page);
  if (process.env.GLOW_SHEET === "1") await glowSheet(ctx, page, rifle.id, tank.id);
  if (process.env.MARKS_SHEET)
    await marksSheet(ctx, page, [rifle.id, tank.id, other?.id], fogged, process.env.MARKS_SHEET);
  await page.close();
}

/** An order mark never draws over a body (post-close review, 6). With
 *  Space held every soldier stands on his marker ring. From a low camera the
 *  ring's far arc lies behind his legs on screen, and the paint must stay on
 *  the ground. His body is the ground mask's non-ground pixels in a band over
 *  his feet; above his ankles (`ANKLE_M`), none of them may carry the
 *  orders' yellow. */
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
  await aim(page, [lead[0], lead[1]], {
    distance: CAMERA.zoom_min,
    pitch: 0.6,
    yaw: CAMERA.default.yaw,
  });
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
  await lab(page, () => window.__lab.setFrameView("final"));
  const over = await paintOnly(ctx, page, "overdraw");
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
 *  stands and a selected vehicle's marker are the selection's amber, while
 *  the squad's destination area ring stays the orders' yellow. */
async function checkSelectionYellow(ctx, page, squadId, vehicleId) {
  const o = await obs(page);
  const squad = o.own.find((u) => u.id === squadId);
  const vehicle = o.own.find((u) => u.id === vehicleId);
  const { area_draw_scale: scale, vehicle_marker_margin_m: margin } = visuals.overlay.orders;
  const vehicleR = hullOf(vehicle.kind).half_extents_m[0] + margin;
  // The paint (the orders and the selection), each mark in its own colour.
  const paint = await orderPaint(ctx, page, "orders-selected");
  const near = (css, test) => anyNear(paint, css, 2, (c) => test(...c));
  // The orders' yellow (green near red), and the selection's amber (red
  // well over green).
  const yellow = (r, g, b) => paintHue([r, g, b]) === "yellow";
  const warm = (r, g, b) => paintHue([r, g, b]) === "amber";
  const inked = (r, g, b) => r + g + b > 45;
  // Samples round a circle: how many are inked, and how many of those in
  // the selection's colour.
  const circle = async (c, radius) => {
    const out = { inked: 0, yellow: 0 };
    const look = near;
    for (let k = 0; k < 32; k++) {
      const a = (k / 32) * 2 * Math.PI;
      const q = [c[0] + Math.cos(a) * radius, c[1] + Math.sin(a) * radius];
      const z = await lab(page, (w) => window.__lab.route.surfaceZ(w[0], w[1]), q);
      const p = await lab(page, (w) => window.__lab.projectToCss(w[0], w[1], w[2]), [...q, z]);
      if (!p || !look(p, inked)) continue;
      out.inked++;
      if (look(p, warm)) out.yellow++;
    }
    return out;
  };
  const here = Math.max(
    ...squad.members.map((m) => Math.hypot(m[0] - squad.position[0], m[1] - squad.position[1])),
  );
  const standing = await circle(squad.position, (here + 0.45 + 0.4) * scale);
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
  // orders' yellow in the paint, sampled on a grid well inside each circle
  // (the selected soldiers' markers are the selection's amber).
  const order = yellow;
  // A route that ran inside would lie along its first leg from the centre
  // (another unit's mark may cross a circle by chance: paint hides under no
  // hull): sample that line from the centre to near the rim.
  const routeInside = async (u, r) => {
    if (!u.route.length) return 0;
    const a = Math.atan2(u.route[0][1] - u.position[1], u.route[0][0] - u.position[0]);
    let hits = 0;
    for (let t = 0.1; t <= 0.8; t += 0.1) {
      const q = [u.position[0] + Math.cos(a) * r * t, u.position[1] + Math.sin(a) * r * t];
      const p = await lab(
        page,
        (w) =>
          window.__lab.projectToCss(w[0], w[1], window.__lab.route.surfaceZ(w[0], w[1]) + w[2]),
        [...q, 0],
      );
      if (p && near(p, order)) hits++;
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
      if (p && near(p, order)) hits++;
    }
    return hits;
  };
  const inside = {
    squad: await routeInside(squad, radius),
    vehicle: await routeInside(vehicle, vehicleR),
    squadArrow: await routeOnArrow(squad, radius),
    vehicleArrow: await routeOnArrow(vehicle, vehicleR),
  };
  ctx.check(
    "no route is drawn inside a unit's own circle marker or over its arrowhead, squad or vehicle",
    Object.values(inside).every((n) => n === 0),
    JSON.stringify(inside),
  );
  const area =
    squad.goal && squad.area ? await circle(squad.area.anchor, squad.area.radius * scale) : null;
  const marker = await circle(vehicle.position, vehicleR);
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

/** A squad's route joins its circle where the circle ends, along the
 *  route's first leg on screen, selected (the circle amber, the route
 *  yellow) or not (both yellow). Read in the paint view, each pixel is split
 *  into its amber and its yellow by solving its red and green against the
 *  two roles' painted colours, so neither is judged by a hue threshold on an
 *  antialiased edge. Each edge is where its ink crosses half its own peak,
 *  interpolated between samples: the circle's outer edge and the route's
 *  start. Space is held throughout (the caller's), so the orders show. */
async function checkRimJoin(ctx, page, squadId) {
  const squad = (await obs(page)).own.find((u) => u.id === squadId);
  if (!squad?.route.length) {
    ctx.check("a squad's route joins its circle at the rim", false, "no route");
    return;
  }
  const [from, to] = [await groundCss(page, squad.position), await groundCss(page, squad.route[0])];
  const d = Math.hypot(to[0] - from[0], to[1] - from[1]);
  const STEP = 0.25;
  const ts = Array.from({ length: Math.floor(d / STEP) }, (_, k) => k * STEP);
  // Bilinear, so the profile is continuous along the ray.
  const at = (png, t) => {
    const x = from[0] + ((to[0] - from[0]) * t) / d,
      y = from[1] + ((to[1] - from[1]) * t) / d;
    const [x0, y0] = [Math.floor(x), Math.floor(y)];
    const [fx, fy] = [x - x0, y - y0];
    const c = [0, 0, 0];
    for (const [dx, dy, w] of [
      [0, 0, (1 - fx) * (1 - fy)],
      [1, 0, fx * (1 - fy)],
      [0, 1, (1 - fx) * fy],
      [1, 1, fx * fy],
    ]) {
      const i = ((y0 + dy) * png.width + x0 + dx) * 4;
      for (let k = 0; k < 3; k++) c[k] += png.data[i + k] * w;
    }
    return c;
  };
  const { color, selected: amberRgb, glow } = visuals.overlay.orders;
  const Y = color.map((v) => v * glow.order);
  const A = amberRgb.map((v) => v * glow.selected);
  const det = A[0] * Y[1] - A[1] * Y[0];
  const split = ([r, g]) => ({
    amber: (r * Y[1] - g * Y[0]) / det,
    yellow: (A[0] * g - A[1] * r) / det,
  });
  // Where `f` last falls through `share` of its peak before `until`, and
  // first rises through half of `plateau` after `after`, interpolated
  // between samples. The circle ends in its arrowhead's point, whose ink
  // fades out over its last pixels: it ends where its ink goes out (a tenth
  // of its peak). The route starts square: at its half-peak.
  const fallsAt = (f, until, share) => {
    const level = share * Math.max(...ts.filter((t) => t < until).map(f));
    let edge = null;
    for (let k = 1; k < ts.length && ts[k] < until; k++)
      if (f(ts[k - 1]) >= level && f(ts[k]) < level)
        edge = ts[k - 1] + (STEP * (f(ts[k - 1]) - level)) / (f(ts[k - 1]) - f(ts[k]));
    return edge;
  };
  const risesAt = (f, after, plateau) => {
    for (let k = 1; k < ts.length; k++)
      if (ts[k] > after && f(ts[k - 1]) < plateau / 2 && f(ts[k]) >= plateau / 2)
        return ts[k - 1] + (STEP * (plateau / 2 - f(ts[k - 1]))) / (f(ts[k]) - f(ts[k - 1]));
    return null;
  };

  // Selected: the circle's amber goes out, the route's yellow starts.
  await lab(page, (id) => window.__lab.route.select([id]), squadId);
  const paint = await orderPaint(ctx, page, "rim-join-selected");
  const amber = (t) => Math.max(0, split(at(paint, t)).amber);
  const yellow = (t) => Math.max(0, split(at(paint, t)).yellow);
  const plateau = Math.max(...ts.map(yellow));
  const firstYellow = ts.find((t) => yellow(t) >= plateau / 2) ?? d;
  const circleEnd = fallsAt(amber, firstYellow + 4, 0.1);
  const routeStart = circleEnd === null ? null : risesAt(yellow, circleEnd - 4, plateau);
  const gap = circleEnd !== null && routeStart !== null ? routeStart - circleEnd : null;

  // Unselected, circle and route one yellow: between where the circle ended
  // and where the route started (selected, the same frame), the yellow falls
  // under half the route's only where the route meets the arrowhead's point
  // (its last two pixels of fading ink), never as a gap.
  await lab(page, () => window.__lab.route.select([]));
  const plain = await orderPaint(ctx, page, "rim-join-unselected");
  const plainYellow = (t) => Math.max(0, split(at(plain, t)).yellow);
  const plainPlateau = Math.max(...ts.map(plainYellow));
  let dark = 0;
  if (gap !== null)
    for (const t of ts)
      if (t > circleEnd - 2 && t < routeStart + 2 && plainYellow(t) < plainPlateau / 2)
        dark += STEP;
  await lab(page, (id) => window.__lab.route.select([id]), squadId);
  ctx.check(
    "a squad's route joins its circle at the rim, selected (amber circle, yellow route) or not",
    gap !== null && Math.abs(gap) <= 1.5 && dark <= 2.5,
    JSON.stringify({ circleEnd, routeStart, gap, unselectedDark: dark }),
  );
}

/** Painted ground marks sit under the effects: on a moving
 *  tank's marker ring at the ground camera, its dust, drawn over the ring,
 *  hides part of it (some of the ring shows less ink than with the effects
 *  held off). The readouts scene checks that they take cast shadows. */
async function checkPaintedLight(ctx, page, vehicleId) {
  // The tank's marker as paint: the selection's amber.
  const selectedBefore = await lab(page, () => window.__lab.route.selected());
  await lab(page, (id) => window.__lab.route.select([id]), vehicleId);
  const vehicle = (await obs(page)).own.find((u) => u.id === vehicleId);
  await aim(page, vehicle.position, {
    distance: CAMERA.zoom_min,
    pitch: CAMERA.pitch_curve[0][1],
    yaw: CAMERA.default.yaw,
  });
  await lab(page, () => window.__lab.frame());
  const r = hullOf(vehicle.kind).half_extents_m[0] + visuals.overlay.orders.vehicle_marker_margin_m;
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
    await aim(page, at, { distance, pitch, yaw: CAMERA.default.yaw });
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
  await aim(page, mid, { distance: CAMERA.default.distance, pitch: 0.85, yaw: CAMERA.default.yaw });
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

/** The street's field works (teeth, sandbags, fences)
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
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 } });
  const { distance, yaw } = visuals.camera.default;
  for (const [name, at] of Object.entries(WORKS)) {
    await aim(page, at, { distance, pitch: 0.85, yaw });
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
  const works = streetMap.props.filter((p) => look[p.kind]);
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
 *  driving and standing, its cupola HMG while driving, the jeep's HMG and a
 *  rifleman's rifle. The cannon and jeep share a road approach; a fresh southern
 *  approach supplies the moving tank HMG. One tank holds throughout; a jeep
 *  approaches the street on a separately admitted move. The rifle fights in
 *  a third battle: the street's houses see the open west and the tank shells
 *  what they see, so a squad crossing it dies short of rifle range. It rounds
 *  the west wood's south holding fire, as the woods tour's walker does, and
 *  goes into the trees at the red AT team on the wood's north edge. At each
 *  kind's first shot the default camera frames the shooter, and the flash's
 *  core and the drawn muzzle socket (posed from the model instance, apart
 *  from the flashes' own muzzles) must project within `MUZZLE_PX`, with the
 *  flash's light on screen there. */
async function muzzleTour(ctx) {
  let page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick: 30 });
  const approach = async (southern) => {
    const o = await obs(page);
    const driver = o.own.find((u) => u.kind === "test_tank");
    const jeep = o.own.find((u) => u.kind === "test_jeep");
    await stageStreetMoves(
      ctx,
      page,
      [
        {
          kind: "move",
          units: [driver.id],
          gesture: 1,
          goal: southern ? [720, 910] : [driver.position[0] + 400, driver.position[1]],
          route: "shortest",
        },
        { kind: "attack_move", units: [jeep.id], gesture: 2, goal: [690, 910] },
      ],
      southern ? "muzzle southern approach" : "muzzle road approach",
    );
  };
  const flank = async () => {
    const rifle = (await obs(page)).own.find((u) => u.kind === "test_rifle");
    const [[x0], , [, y1]] = streetMap.forests[0].shape.ring;
    const corner = [x0 - 60, y1 + 40];
    // A fire policy, acknowledged before the next staging counts its own.
    const policy = async (p) => {
      const before = await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);
      await lab(
        page,
        (c) => window.__lab.route.command({ kind: "set_engagement", units: [c.id], policy: c.p }),
        { id: rifle.id, p },
      );
      await page.waitForFunction((n) => (window.__lab.route.acks()[0]?.seq ?? 0) > n, before);
    };
    await policy("return_fire_only");
    await stageStreetMoves(
      ctx,
      page,
      [
        {
          kind: "move",
          units: [rifle.id],
          gesture: 3,
          goal: [x0 - 300, y1 + 40],
          route: "fastest",
        },
        {
          order: { kind: "move", units: [rifle.id], gesture: 4, goal: corner, route: "fastest" },
          queued: true,
        },
      ],
      "muzzle rifle round the wood",
    );
    const atCorner = await until(
      page,
      (o) => {
        const u = o.own.find((q) => q.id === rifle.id);
        return (
          u &&
          !u.goal &&
          u.queue.length === 0 &&
          Math.hypot(u.position[0] - corner[0], u.position[1] - corner[1]) < 1
        );
      },
      15000,
      30,
    );
    ctx.check(
      "the rifle squad reaches the wood's south-west corner",
      !!atCorner,
      JSON.stringify(atCorner?.own.find((q) => q.id === rifle.id)?.position ?? null),
    );
    if (!atCorner) throw new Error("muzzle rifle never reached the wood");
    const at = streetAttack.units.find((u) => u.side === "red" && u.kind === "test_at");
    await policy("fire_at_will");
    await stageStreetMoves(
      ctx,
      page,
      [{ kind: "attack_move", units: [rifle.id], gesture: 5, goal: at.position }],
      "muzzle rifle into the wood",
    );
  };
  const passes = [
    { name: "road", subjects: ["tank cannon, driving", "tank cannon, standing", "jeep HMG"] },
    { name: "southern", subjects: ["tank HMG, driving"] },
    { name: "wood", subjects: ["rifle"] },
  ];
  const wanted = new Set(passes.flatMap((p) => p.subjects));
  const found = {};
  for (const [k, pass] of passes.entries()) {
    // Separate controlled fights: the moving tank HMG in physical reach on
    // the flank, and the rifle's fight in the wood.
    if (k > 0) {
      await page.close();
      page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick: 30 });
    }
    if (pass.name === "wood") {
      await flank();
      // The wood's canopy roofs the rifle's fight from the capture camera:
      // that battle is drawn with the trees lifted off, as the ground
      // stations do, from before the fight so no capture rebuilds the scene.
      await lab(page, () => window.__lab.suppressTrees(true));
    } else await approach(pass.name === "southern");
    const pending = new Set(pass.subjects);
    let prev = await obs(page);
    for (let t = 0; t < 30 * 180 && pending.size > 0; t++) {
      await advance(page, 1);
      let o = await obs(page);
      let shot = null;
      for (const u of o.own) {
        const was = prev.own.find((q) => q.id === u.id);
        if (!was || shot) continue;
        const driving = Math.hypot(
          u.position[0] - was.position[0],
          u.position[1] - was.position[1],
        );
        for (const w of u.weaponPoses) {
          const before = was.weaponPoses.find((x) => x.mount === w.mount)?.shots ?? w.shots;
          if (w.shots <= before) continue;
          const name =
            u.kind === "test_tank"
              ? `tank ${w.mount === 0 ? "cannon" : "HMG"}, ${driving > 0.01 ? "driving" : "standing"}`
              : u.kind === "test_jeep"
                ? "jeep HMG"
                : u.kind === "test_rifle" && w.mount === 0
                  ? "rifle"
                  : null;
          if (pending.has(name)) shot = { name, unit: u, mount: w.mount };
        }
      }
      prev = o;
      if (!shot) continue;
      const { name, unit } = shot;
      // The shooter's drawn muzzle: a hull's mount's node, or the rifleman
      // whose new round starts next tick (the simulation flies a round from
      // the tick after it fires).
      let near = unit.position;
      let shooterAt = unit.position;
      let socket = shot.mount === 0 && unit.kind === "test_tank" ? "muzzle" : "hmg_muzzle";
      if (name === "rifle") {
        await advance(page, 1);
        o = prev = await obs(page);
        const squad = o.own.find((u) => u.id === unit.id);
        const round = o.projectiles.find(
          (s) =>
            s.kind === "rifle" &&
            squad.memberIds.includes(s.shooterMember) &&
            Math.hypot(
              squad.members[squad.memberIds.indexOf(s.shooterMember)][0] - s.path[0][0],
              squad.members[squad.memberIds.indexOf(s.shooterMember)][1] - s.path[0][1],
            ) < 2,
        );
        if (!round) continue;
        near = round.path[0];
        shooterAt = squad.members[squad.memberIds.indexOf(round.shooterMember)];
        socket = "muzzle";
      }
      wanted.delete(name);
      pending.delete(name);
      await aim(page, near, {
        distance: CAMERA.default.distance,
        pitch: 0.85,
        yaw: CAMERA.default.yaw,
      });
      const slug = name.replace(/[^a-z]+/gi, "-").toLowerCase();
      await presented(page);
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
      // Select the firing body first: a neighboring soldier's barrel can lie
      // closer to the published launch point than this soldier's own muzzle.
      const muzzle = candidates.length
        ? candidates.reduce((a, b) =>
            dist(b.origin, shooterAt) < dist(a.origin, shooterAt) ? b : a,
          ).at
        : null;
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
  }
  ctx.check(
    "every kind of shot fired in the staged street fights: tank cannon driving and standing, tank HMG driving, jeep HMG, rifle",
    wanted.size === 0,
    JSON.stringify({ missing: [...wanted], found: Object.keys(found) }),
  );
  await page.close();
}

/** Select similar (double-click a unit for its
 *  type, again or with Ctrl for its role), a mixed selection's command bar
 *  (the union of its capabilities, each order to the units that can), and
 *  the army card's role symbol and silhouette. */
async function selectionTour(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1600, height: 900 } });
  const o = await obs(page);
  const ids = (kind) => o.own.filter((u) => u.kind === kind).map((u) => u.id);
  const [rifles, tanks, trucks] = [ids("test_rifle"), ids("test_tank"), ids("test_supply")];
  const selected = () => lab(page, () => [...window.__lab.route.selected()].sort((a, b) => a - b));
  const same = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);
  /** Double-click a unit where it is drawn: a soldier's torso, a hull's middle. */
  const doubleClick = async (unit, modifiers = []) => {
    const at = unit.members.length ? unit.members[0] : unit.position;
    await aim(page, at, { distance: 60, pitch: 0.85, yaw: CAMERA.default.yaw });
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
    .filter((u) => hasRole(u.kind, unitType("test_rifle").roles[0]))
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

  // The card: the tank's portrait (its role symbol and silhouette) beside
  // its info panel, the callouts' own.
  await lab(page, (id) => window.__lab.route.select([id]), tanks[0]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  const card = page.locator(`.hud-army-card[data-unit="${tanks[0]}"]`);
  await card.hover();
  const detail = page.getByRole("tooltip");
  const icons = await card.evaluate((c) =>
    [...c.querySelectorAll("svg")].map((s) => {
      const r = s.getBoundingClientRect();
      return { w: r.width, h: r.height, paths: s.querySelectorAll("path").length };
    }),
  );
  const cardPanel = await detail.evaluate((c) => ({
    name: c.querySelector(".ro-body .ro-name-word")?.textContent,
    weapons: c.querySelectorAll(".ro-body .ro-weapon").length,
    strength: c.querySelector(".ro-name > .ro-pips")?.dataset.lit,
  }));
  ctx.check(
    "the unit card shows the role symbol and the model's silhouette, each at least 20 px tall, with the tank's hover info panel with its strength",
    icons.length === 2 &&
      icons.every((i) => i.h >= 20) &&
      cardPanel.name === `Tank #${tanks[0]}` &&
      cardPanel.weapons === 2 &&
      cardPanel.strength === "5",
    JSON.stringify({ icons, cardPanel }),
  );
  await page.locator("footer.hud-bottom").screenshot({
    path: ctx.evidencePath("army-deck-tank.png"),
  });
  await lab(page, (id) => window.__lab.route.select([id]), rifles[0]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  await page.locator("footer.hud-bottom").screenshot({
    path: ctx.evidencePath("army-deck-rifle.png"),
  });

  // A tank and a supply truck: the one Deploy/Pack toggle reaches the truck
  // only, and says which it will do.
  await lab(page, (sel) => window.__lab.route.select(sel), [tanks[0], trucks[0]]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 2);
  const toggle = page.getByRole("button", { name: /^(Deploy|Pack) / });
  const bar = await toggle.evaluate((b) => ({
    name: b.ariaLabel.split(" ")[0],
    reach: b.dataset.reach,
  }));
  const setUp = (await obs(page)).own.find((u) => u.id === trucks[0]).deployment?.target;
  const verb = setUp === "deployed" ? "Pack" : "Deploy";
  ctx.check(
    "with a tank and a supply truck selected, one Deploy/Pack toggle names what it will do without a fraction",
    bar.name === verb && bar.reach === undefined,
    JSON.stringify({ bar, setUp }),
  );
  await page.locator("footer.hud-bottom").screenshot({
    path: ctx.evidencePath("selection-group-mixed.png"),
  });
  const before = await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);
  await toggle.click();
  await page.waitForFunction((n) => (window.__lab.route.acks()[0]?.seq ?? 0) > n, before);
  const [ack] = await lab(page, () => window.__lab.route.acks());
  await advance(page, 3);
  const after = await obs(page);
  const truck = after.own.find((u) => u.id === trucks[0]);
  const tankAfter = after.own.find((u) => u.id === tanks[0]);
  ctx.check(
    "the toggle orders only the truck: it sets up (or packs), the tank carries on",
    ack.ack.error === null &&
      ack.label === `${verb.toLowerCase()} test_supply #${trucks[0]}` &&
      truck.deployment?.target === (verb === "Deploy" ? "deployed" : "packed") &&
      !tankAfter.deployment,
    JSON.stringify({ ack, truck: truck.deployment, tank: tankAfter.deployment }),
  );
  await snapshot(ctx, page, "selection-mixed-1600x900.png");
  await page.close();
}

/** The info panels' layout: at a far, busy camera the closest units' panels
 *  get full detail and stay clear of each other, holding that layout steady,
 *  and hovering an unselected unit opens its full panel. */
async function panelLayoutTour(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 } });
  const own = (await obs(page)).own;
  await lab(
    page,
    (ids) => window.__lab.route.select(ids),
    own.map((u) => u.id),
  );
  const cards = () =>
    lab(page, () =>
      [...document.querySelectorAll(".ro-layer .ro-unit")]
        .filter((n) => n.style.display !== "none")
        .map((n) => {
          const id = n.dataset.unit ?? n.dataset.enemy ?? n.dataset.contact;
          const r = n.getBoundingClientRect();
          return {
            id,
            owner: n.dataset.owner,
            key: `${n.dataset.owner}-${id}`,
            mode: n.dataset.zoom,
            x: r.x,
            y: r.y,
            w: r.width,
            h: r.height,
          };
        }),
    );
  for (const [name, distance, pitch, yaw] of [
    ["crowded", 900, 0.85, CAMERA.default.yaw],
    ["horizontal", 450, 0.25, Math.PI],
  ]) {
    await page.mouse.move(1885, 25);
    await aim(page, [230, 800], { distance, pitch, yaw });
    await lab(page, () => window.__lab.frame());
    const small = await cards();
    const o = await obs(page);
    const eye = await lab(page, () => window.__lab.rayAt(960, 540).origin);
    const point = (c) =>
      c.owner === "own"
        ? o.own.find((u) => String(u.id) === c.id).position
        : c.owner === "enemy"
          ? o.identified.find((u) => String(u.id) === c.id).position
          : [...o.contacts.find((u) => String(u.id) === c.id).center, 0];
    const d = (c) => point(c).reduce((sum, x, i) => sum + (x - eye[i]) ** 2, 0);
    const quota = Math.ceil(small.length * 0.3);
    const nearest = [...small]
      .sort((a, b) => d(a) - d(b))
      .slice(0, quota)
      .map((c) => c.key);
    await shot(ctx, page, `panels-${name}-small`);
    await page.keyboard.down("Space");
    await page.waitForFunction(
      () => document.querySelector(".ro-layer").dataset.zoom === "default",
    );
    await lab(page, () => window.__lab.frame());
    const full = await cards();
    const moved = full.filter((c, i) => Math.hypot(c.x - small[i].x, c.y - small[i].y) > 0.5);
    const overlap = full.some((a, i) =>
      full
        .slice(i + 1)
        .some((b) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h),
    );
    ctx.check(
      `${name}: the closest 30% receive full detail`,
      small.length > 0 && nearest.every((k) => full.find((c) => c.key === k)?.mode === "default"),
      JSON.stringify({ nearest, full }),
    );
    ctx.check(
      `${name}: full cards stay clear with at most one shift per nearby card`,
      !overlap && moved.length <= quota,
      JSON.stringify({ moved: moved.map((c) => c.key), quota }),
    );
    await shot(ctx, page, `panels-${name}-full`);
    await lab(page, () => window.__lab.frame());
    ctx.check(
      `${name}: the held layout remains stable`,
      JSON.stringify(await cards()) === JSON.stringify(full),
    );
    await page.keyboard.up("Space");
    await page.waitForFunction(
      () => document.querySelector(".ro-layer").dataset.zoom === "compressed",
    );
  }
  const tank = own.find((u) => u.kind === "test_tank");
  await lab(page, () => window.__lab.route.select([]));
  await aim(page, tank.position, { distance: 900, pitch: 0.85, yaw: CAMERA.default.yaw });
  await lab(page, () => window.__lab.frame());
  const selector = `.ro-layer .ro-unit[data-unit="${tank.id}"]`;
  const p = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1), tank.position);
  await page.mouse.move(p[0], p[1]);
  await page.waitForFunction((s) => {
    const n = document.querySelector(s);
    return n?.dataset.zoom === "default" && n.style.display !== "none";
  }, selector);
  ctx.check("hovering an unselected unit reveals its full card even at far zoom", true);
  await shot(ctx, page, "panels-hover-unit");
  await page.mouse.move(1885, 25);
  await page.waitForFunction((s) => {
    const n = document.querySelector(s);
    return (
      n?.style.display !== "none" &&
      n?.dataset.zoom === "compressed" &&
      !n.parentElement.dataset.hovered
    );
  }, selector);
  ctx.check("leaving unit hover restores its visible compact card", true);
  await aim(page, tank.position, { distance: 450, pitch: 0.85, yaw: CAMERA.default.yaw });
  await lab(page, () => window.__lab.frame());
  await page.locator(selector).locator(".ro-name").hover();
  await page.waitForFunction(
    (s) => document.querySelector(s)?.dataset.zoom === "default",
    selector,
  );
  const r = await page.locator(selector).boundingBox();
  await page.mouse.move(r.x + 10, r.y + r.height - 5);
  await lab(page, () => window.__lab.frame());
  ctx.check(
    "hovering the expanded rows keeps the card open without Space",
    (await page.locator(selector).getAttribute("data-zoom")) === "default",
  );
  await shot(ctx, page, "panels-hover-card");
  await page.mouse.click(r.x + 10, r.y + r.height - 5);
  await lab(page, () => window.__lab.frame());
  ctx.check(
    "clicking a hovered card keeps its detail open while the cursor rests there",
    (await page.locator(selector).getAttribute("data-zoom")) === "default",
  );
  await page.mouse.move(1885, 25);
  await page.waitForFunction(
    (s) => document.querySelector(s)?.dataset.zoom === "compressed",
    selector,
  );
  ctx.check(
    "hover never issues a unit command",
    (await lab(page, () => window.__lab.route.acks())).length === 0,
  );
  await page.close();
}

/** The tours, by name: `STREET_TOURS=effects,smoke` runs only those. */
const TOURS = {
  appflow: appJourney,
  army: armyJourney,
  cursor: battleCursor,
  "panel-layout": panelLayoutTour,
  ruler: rulerTour,
  selection: selectionTour,
  orders: orderTour,
  "group-preview": groupPreviewTour,
  "group-preview-plain": (ctx) => groupPreviewTour(ctx, false),
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
  play: playTour,
};

export async function run(ctx) {
  const only = process.env.STREET_TOURS?.split(",");
  if (only) {
    for (const name of only) await TOURS[name](ctx);
    return;
  }
  for (const visit of Object.values(TOURS)) await visit(ctx);
}
