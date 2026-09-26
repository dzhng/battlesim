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
// Battle-look slice 25: combat effects in the firefight, a burst read at its
// moment and after, from the effects' own frame and the pass inspector's
// world view.
import { readFile } from "node:fs/promises";
import { lab, obs, advance, until, snapshot } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";
import { checkOverlayIsolation } from "./_overlays.mjs";

const village = JSON.parse(
  await readFile(new URL("../../fixtures/village.json", import.meta.url), "utf8"),
);
const CAMERA = village.presentation.camera;
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
  // Rings, zone and orders are overlays: exactly their own colours over the
  // finished, fogged and graded world.
  const isolation = await checkOverlayIsolation(ctx, page, "overlay-default");
  ctx.check(
    "overlays keep their own colours over the finished frame",
    isolation.isolated && isolation.opaque > 0,
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
  edge: { target: [790, 790], distance: 120, pitch: 0.42, yaw: -Math.PI / 2 },
  // The same edge from the ground, the camera's closest zoom.
  ground: { target: [790, 800], distance: 25, pitch: 0.22, yaw: -Math.PI / 2 },
  // Straight down on the west forest's south-west corner.
  top: { target: [712, 832], distance: 90, pitch: 1.5, yaw: -Math.PI / 2 },
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
      (s) => s.forest.placed > 500 && drawn(s.forest) === s.forest.placed,
    ) &&
      seen.road.backdrop.placed > 2000 &&
      drawn(seen.road.backdrop) > 0 &&
      drawn(seen.top.backdrop) < drawn(seen.road.backdrop) &&
      seen.ground.forest.tiers[0] > 0,
    JSON.stringify(seen),
  );
  const top = decode(await snapshot(ctx, page, "trees-top-check-1920x1080.png"));
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
  const outside = (await luminance(x0 - 14, y0 + 22)) + (await luminance(x0 + 22, y0 - 14));
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

const isVehicle = (u) => u.kind === "tank" || u.kind === "supply";
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
      posed.every((v) => v.articulation && ["tank", "supply_truck"].includes(v.appearance)) &&
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
  const structures = await lab(page, () => window.__lab.route.structures());
  ctx.check(
    "the village's houses stand as their appearances, each fitted to its box",
    structures.length === village.map.props.length &&
      structures.every(
        (s, i) =>
          s.state === "intact" &&
          s.position[0] === village.map.props[i].center[0] &&
          s.position[1] === village.map.props[i].center[1] &&
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
  const o = await until(page, (f) => f.blasts.length > 0, 30 * 60, 1);
  if (!o) {
    ctx.check("a burst in the firefight is drawn as a fireball", false, "no blast by tick 2400");
    await page.close();
    return;
  }
  const burst = o.blasts[0].point;
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
      units: o.own.map((u) => u.id),
      gesture: 1,
      goal: [1000, 800],
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

  const o = await until(page, (f) => f.knownProps.some((p) => p.kind === "wreck"), 30 * 150, 15);
  if (!o) {
    ctx.check("a known wreck burns and smokes", false, "no wreck by tick 4600");
    await page.close();
    return;
  }
  const wreck = o.knownProps.find((p) => p.kind === "wreck");
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

/** The tours, by name: `VILLAGE_TOURS=effects,smoke` runs only those. */
const TOURS = {
  camera: tour,
  trees: treeTour,
  soldiers: soldierTour,
  vehicles: vehicleTour,
  effects: effectTour,
  smoke: smokeTour,
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

  // Each selected tank and its destination ring carry the panel's name.
  const tags = await lab(page, () => ({
    names: [...document.querySelectorAll(".ro-unit.ro-selected .ro-name")].map((n) => ({
      unit: Number(n.parentElement.dataset.unit),
      text: n.textContent,
    })),
    goals: [...document.querySelectorAll(".ro-goal")].map((n) => {
      const r = n.getBoundingClientRect();
      return {
        unit: Number(n.dataset.goal),
        text: n.textContent,
        shown: n.style.display !== "none",
        at: [r.x + r.width / 2, r.bottom],
      };
    }),
    panel: [...document.querySelectorAll("[data-testid=selection-panel] [data-unit] strong")].map(
      (n) => n.textContent,
    ),
  }));
  const now = await obs(page);
  const goalPx = await Promise.all(
    tanks.map((id) => {
      const g = now.own.find((u) => u.id === id).goal;
      return g ? lab(page, (p) => window.__lab.projectToCss(p[0], p[1], 0), g) : null;
    }),
  );
  ctx.check(
    "each selected tank and its destination ring show the panel's name",
    tanks.every((id, k) => {
      const name = tags.names.find((n) => n.unit === id)?.text;
      const goal = tags.goals.find((g) => g.unit === id);
      // A tank still under way has a tag just above its destination ring.
      const atRing =
        !goalPx[k] ||
        (goal?.text === name &&
          goal.shown &&
          Math.abs(goal.at[0] - goalPx[k][0]) < 40 &&
          goal.at[1] < goalPx[k][1] &&
          goalPx[k][1] - goal.at[1] < 40);
      return !!name && tags.panel.includes(name) && atRing;
    }),
    JSON.stringify({ tags, goalPx }),
  );

  // Zoomed out, where the two tanks' clusters and destination names would
  // pile up, none sits under the panel and none overprints another.
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
      panel: box(document.querySelector("[data-occludes-readouts]")),
      readouts: shown(".ro-unit"),
      goals: shown(".ro-goal"),
    };
  });
  const overlap = (a, b) => a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1;
  ctx.check(
    "no readout or name sits under the panel or overprints another",
    placed.readouts.length > 0 &&
      placed.goals.length === tanks.length &&
      [...placed.readouts, ...placed.goals].every(
        (b, k, all) => !overlap(b, placed.panel) && all.every((c, j) => j === k || !overlap(b, c)),
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
  await page.getByLabel("Seed").fill("7");
  await page.waitForFunction(
    () => /seed 7 /.test(document.querySelector("[data-testid=status]")?.textContent ?? ""),
    undefined,
    { timeout: 30000 },
  );
  ctx.check("a new seed restarts the battle on that seed", true);
  await page.getByLabel("Variant").selectOption("prepared_crossfire");
  await page.waitForFunction(
    () =>
      /Prepared crossfire/.test(document.querySelector("[data-testid=status]")?.textContent ?? ""),
    undefined,
    { timeout: 30000 },
  );
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  ctx.check("the variant select loads the crossfire battle", true);
}
