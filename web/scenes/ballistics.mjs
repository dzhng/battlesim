// Slice 07: physical flight reproduced in the browser through the WASM flight
// store. Outcomes come from the store's events; frames show trajectories and
// impact locations from an overview, side views, a timing sequence and crops.
import { writeFile } from "node:fs/promises";
import { decode, pixel, writeCrop } from "./_png.mjs";
import { checkOverlayIsolation } from "./_overlays.mjs";

const END_TICK = 300;

const CAMERAS = {
  overview: null,
  // Side-on to the crest shots (flying north at x ≈ 96), from the east.
  crest: { target: [96, 205, 32], distance: 250, pitch: 0.12, yaw: 0 },
  // Side-on to the gravity arcs (flying east along y ≈ 20) from beyond the
  // map's south edge, so no other trace stands in front of them.
  arcs: { target: [110, 20, 2], distance: 140, pitch: 0.08, yaw: -Math.PI / 2 },
  // The fast bullet at the sliding board, from the shooter's side.
  board: { target: [150, 134.6, 0.9], distance: 9, pitch: 0.35, yaw: Math.PI + 0.7 },
  // The soldier and the tank crossing the grenade's line.
  crossing: { target: [340, 140, 2], distance: 80, pitch: 0.3, yaw: -1.15 },
  // The oblique-AP preset's standing tank and the bounce paths leaving it.
  ricochet: { target: [238, 262, 1.5], distance: 45, pitch: 0.55, yaw: -2.2 },
};
// Trace tube half widths sized to each camera's distance.
const LINE_HALF = {
  overview: 0.3,
  crest: 0.25,
  arcs: 0.1,
  board: 0.02,
  crossing: 0.1,
  ricochet: 0.06,
};

const flat = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);

async function show(page, name) {
  await page.evaluate(
    ([camera, half]) => {
      const lab = window.__lab;
      if (camera) lab.setCamera({ ...lab.camera(), ...camera });
      else lab.reset();
      lab.route.setLineHalfWidth(half);
    },
    [CAMERAS[name], LINE_HALF[name]],
  );
  await page.evaluate(() => window.__lab.frame());
  await page.evaluate(() => window.__lab.frame());
}

const runTo = (page, tick) =>
  page.evaluate(async (tick) => {
    window.__lab.route.runTo(tick);
    await window.__lab.frame();
    await window.__lab.frame();
    return window.__lab.route.state();
  }, tick);

const project = (page, p) => page.evaluate((p) => window.__lab.projectToCss(...p), p);

/** Whether any pixel within `r` of (x, y) passes `test`. */
function anyNear(png, x, y, r, test) {
  for (let dy = -r; dy <= r; dy++) {
    for (let dx = -r; dx <= r; dx++) if (test(pixel(png, x + dx, y + dy))) return true;
  }
  return false;
}

async function capture(ctx, page, name) {
  const frame = await page.screenshot();
  await writeFile(ctx.evidencePath(name), frame);
  return frame;
}

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.evaluate(() => window.__lab.route.reset(false));
  ctx.check(
    "village gravity needs one chord per 30 Hz tick within 2 cm",
    (await page.evaluate(() => window.__lab.route.subsegments())) === 1,
  );

  // Timing sequence: overview and the two between-tick collisions.
  await show(page, "overview");
  await runTo(page, 4);
  await capture(ctx, page, "seq-overview-t004.png");
  await show(page, "board");
  await capture(ctx, page, "seq-board-t004.png");
  const board = await runTo(page, 5).then(() => capture(ctx, page, "seq-board-t005.png"));
  const boardHit = (await page.evaluate(() => window.__lab.route.state())).events.find(
    (e) => e.struck === "body:1",
  );
  const boardPx = await project(page, boardHit.point);
  await show(page, "overview");
  await runTo(page, 20);
  await capture(ctx, page, "seq-overview-t020.png");
  await show(page, "crossing");
  await runTo(page, 40);
  await capture(ctx, page, "seq-crossing-t040.png");
  await runTo(page, 46);
  const crossing = await capture(ctx, page, "seq-crossing-t046.png");
  await show(page, "overview");
  await runTo(page, 120);
  await capture(ctx, page, "seq-overview-t120.png");
  // Flight lines and impact marks are overlays: laid over the finished frame.
  const isolation = await checkOverlayIsolation(ctx, page, "overlay-t120");
  ctx.check(
    "flight marks keep their own colours over the finished frame",
    isolation.isolated && isolation.opaque > 0,
    JSON.stringify(isolation),
  );
  const s = await runTo(page, END_TICK);
  await writeFile(ctx.evidencePath("state.json"), JSON.stringify(s, null, 2));

  const shot = (label) => s.shots.find((x) => x.label === label);
  const ends = (e) => e.kind === "impact" || e.kind === "expired";
  const end = (label) => s.events.find((e) => ends(e) && e.projectile === shot(label).projectile);

  // Gravity arcs land on their aim points: 2 cm vertical chord error, grazing.
  const arcs = [60, 120, 180].map((range, i) => ({
    range,
    aim: [20 + range, 14 + 6 * i, 0],
    end: end(`grenade arc ${range} m`),
  }));
  ctx.check(
    "grenade arcs land on their aim points",
    arcs.every((a) => a.end?.struck === "terrain" && flat(a.end.point, a.aim) < 0.05),
    JSON.stringify(arcs.map((a) => [a.range, a.end?.point])),
  );
  const hmg = end("hmg at sliding board");
  ctx.check(
    "a 900 m/s round leads the 0.3 m sliding board and hits its face mid-chord",
    hmg?.struck === "body:1" && hmg.tick === 5 && Math.abs(hmg.point[0] - 149.85) < 1e-6,
    JSON.stringify(hmg),
  );
  const walker = end("grenade at walker");
  const dodger = end("grenade at dodger");
  ctx.check(
    "steady motion is hit; reversing after launch dodges the unguided round",
    walker?.struck === "body:2" && dodger?.struck === "terrain",
    JSON.stringify({ walker, dodger }),
  );
  const direct = shot("direct grenade over crest");
  const mortar = shot("indirect lab mortar over crest");
  const landed = end("indirect lab mortar over crest");
  ctx.check(
    "direct fire over the crest has no solution; the indirect high arc clears it",
    !direct.fired &&
      direct.reason === "blocked" &&
      direct.blocked_at[1] > 130 &&
      direct.blocked_at[1] < 290 &&
      direct.blocked_at[2] > 1 &&
      mortar.fired &&
      mortar.arc === "high" &&
      landed?.struck === "terrain" &&
      flat(landed.point, [92, 292]) < 0.05,
    JSON.stringify({ blocked_at: direct.blocked_at, landed: landed?.point }),
  );
  const across = end("grenade across crossing bodies");
  const passed = s.events.filter(
    (e) => e.kind === "near_miss" && e.unit === 4 && e.projectile === across.projectile,
  );
  ctx.check(
    "the tank crossing late takes the round; the soldier crossing early is a near miss",
    across?.struck === "body:5" && passed.length > 0,
    `${across?.struck}, ${passed.length} near-miss ticks, closest ${Math.min(...passed.map((e) => e.distance)).toFixed(2)} m`,
  );
  const fired = s.shots.filter((x) => x.fired);
  ctx.check(
    "every fired round ended in exactly one event; a ricochet is not an ending",
    fired.every(
      (x) => s.events.filter((e) => ends(e) && e.projectile === x.projectile).length === 1,
    ),
  );
  // The oblique-AP preset: failed penetrations on the standing tank glance off
  // it and fly on as the same round, at most twice.
  const preset = s.shots.filter((x) => /^(oblique AP|hmg at tank side)/.test(x.label));
  const glances = s.events.filter(
    (e) => e.kind === "ricochet" && preset.some((x) => x.projectile === e.projectile),
  );
  const flewOn = glances.filter((g) => {
    const last = s.events.find((e) => ends(e) && e.projectile === g.projectile);
    return last && (last.tick > g.tick || last.time > g.time);
  });
  const ap = preset.filter((x) => x.label.startsWith("oblique AP")).map((x) => x.projectile);
  const perRound = preset.map((x) => glances.filter((g) => g.projectile === x.projectile).length);
  ctx.check(
    "oblique-AP preset: rounds glance off the tank and fly on, never more than twice",
    glances.some((g) => g.struck === "body:6" && ap.includes(g.projectile)) &&
      flewOn.length === glances.length &&
      Math.max(...perRound) <= 2,
    `${glances.length} ricochets across ${preset.length} rounds: ${JSON.stringify(perRound)}`,
  );
  const keys = s.events
    .filter((e) => e.kind === "near_miss")
    .map((e) => `${e.tick}/${e.projectile}/${e.unit}`);
  ctx.check(
    "at most one near miss per round per unit per tick",
    new Set(keys).size === keys.length,
  );

  // Rendered: impact marks sit at the reported impact points.
  // Impact-mark red on lit or shaded faces; not the orange trace or violet body.
  const red = ([r, g, b]) => r > 140 && g < 0.3 * r && b < 0.3 * r;
  ctx.check(
    "the board impact mark is drawn at the reported point",
    boardHit.projectile === hmg.projectile &&
      anyNear(decode(board), boardPx[0], boardPx[1], 12, red),
  );
  await writeCrop(
    decode(board),
    ctx.evidencePath("crop-board-t005-3x.png"),
    boardPx[0],
    boardPx[1],
    120,
    80,
    3,
  );
  await show(page, "crossing");
  const crossingEnd = await capture(ctx, page, "frame-crossing.png");
  const tankPx = await project(page, across.point);
  ctx.check(
    "the tank impact mark is drawn at the reported point",
    anyNear(decode(crossingEnd), tankPx[0], tankPx[1], 12, red),
  );
  await writeCrop(
    decode(crossing),
    ctx.evidencePath("crop-crossing-t046-3x.png"),
    tankPx[0] - 60,
    tankPx[1],
    150,
    90,
    3,
  );

  await show(page, "ricochet");
  const bounce = decode(await capture(ctx, page, "frame-ricochet.png"));
  const bouncePx = await project(page, glances[0].point);
  const lime = ([r, g, b]) => g > 180 && r < 0.75 * g && b < 0.6 * g;
  ctx.check(
    "a ricochet mark is drawn where the round glanced off",
    anyNear(bounce, bouncePx[0], bouncePx[1], 12, lime),
  );
  await writeCrop(
    bounce,
    ctx.evidencePath("crop-ricochet-2x.png"),
    bouncePx[0],
    bouncePx[1],
    240,
    150,
    2,
  );

  await show(page, "crest");
  const crest = decode(await capture(ctx, page, "frame-crest-side.png"));
  const blockPx = await project(page, direct.blocked_at);
  ctx.check(
    "the blocked arc's obstruction mark sits on the crest",
    anyNear(crest, blockPx[0], blockPx[1], 3, ([r, g, b]) => r + g + b < 90),
  );
  await writeCrop(
    crest,
    ctx.evidencePath("crop-crest-block-3x.png"),
    blockPx[0],
    blockPx[1],
    120,
    70,
    3,
  );
  const landPx = await project(page, landed.point);
  await writeCrop(
    crest,
    ctx.evidencePath("crop-crest-landing-3x.png"),
    landPx[0],
    landPx[1],
    90,
    60,
    3,
  );

  await show(page, "arcs");
  const side = decode(await capture(ctx, page, "frame-arcs-side.png"));
  const farPx = await project(page, arcs[2].aim);
  await writeCrop(
    side,
    ctx.evidencePath("crop-arcs-landing-3x.png"),
    farPx[0] - 60,
    farPx[1] - 20,
    140,
    60,
    3,
  );
  const midPx = await project(page, [110, 20, 3]);
  await writeCrop(side, ctx.evidencePath("crop-arcs-strip-2x.png"), midPx[0], midPx[1], 320, 60, 2);
  // The 180 m arc's apex (≈ 6 m up at mid-range) is visibly off the ground.
  const apexPx = await project(page, [110, 26, 7.4]);
  const groundPx = await project(page, [110, 26, 0]);
  ctx.check("the side view resolves the 180 m arc's height", groundPx[1] - apexPx[1] > 20);

  await show(page, "overview");
  await capture(ctx, page, "frame-overview-1280x800.png");

  // Combat effects on the oblique-AP preset, on the
  // tick the first round glances: sparks off the plate, in the world itself
  // (the pass inspector's world view, under no overlay).
  await page.evaluate(() => window.__lab.route.reset(false));
  await runTo(page, glances[0].tick);
  await show(page, "ricochet");
  await page.evaluate(() => window.__lab.setFrameView("world"));
  const sparks = decode(await capture(ctx, page, "frame-effects-ricochet-world.png"));
  await page.evaluate(() => window.__lab.setFrameView("final"));
  await capture(ctx, page, "frame-effects-ricochet.png");
  const effects = await page.evaluate(() => window.__lab.stats().effects);
  const glancePx = await project(page, glances[0].point);
  const hot = ([r, g, b]) => r > 235 && g > 200 && b > 140;
  ctx.check(
    "ricochet sparks are drawn where the round glanced off",
    effects.instances > 0 && anyNear(sparks, glancePx[0], glancePx[1], 16, hot),
    JSON.stringify({ tick: glances[0].tick, effects }),
  );
  await writeCrop(
    sparks,
    ctx.evidencePath("crop-effects-ricochet-3x.png"),
    glancePx[0],
    glancePx[1],
    160,
    100,
    3,
  );

  // Spread: seeded, so the same salvo reproduces exactly and differs from aim.
  const spreadRun = async () => {
    await page.evaluate(() => window.__lab.route.reset(true));
    return (await runTo(page, END_TICK)).events;
  };
  const a = await spreadRun();
  const b = await spreadRun();
  ctx.check(
    "a salvo with weapon spread replays identically from its seed",
    JSON.stringify(a) === JSON.stringify(b) && JSON.stringify(a) !== JSON.stringify(s.events),
    `${a.length} events`,
  );

  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [1280, 800],
    dpr: 1,
    seed: 20260925,
    endTick: END_TICK,
    cameras: CAMERAS,
    lineHalfWidth: LINE_HALF,
  });
}
