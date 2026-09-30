// The ground layer's rules in the browser, as
// each side learns them. Craters from authored bursts and a live barrage, a
// tank slowed (never stopped) by a crater field, tracks, trampling and scorch,
// all drawn from blue's learned cells (the ground patches its publications
// carry); a side switch reopens the stream with red's full snapshot. Then the
// paused village inspector: after the supported attack's opening, each side
// holds its own ground.
// The learned ground drawn as scars on the terrain and
// the grass (crater bowls and rims, scorch, tracks, trampling), only where the
// observed side has learned it, at fixed framings of the lab field
// (SCARS_ONLY=1 runs only those framings and the village inspector).
import { writeFile } from "node:fs/promises";
import { decode, mostChanged, pixel } from "./_png.mjs";
import { lab, obs, advance, snapshot, openBattle, aim, groundCss } from "./_lab.mjs";

const x = (o, id) => o.own.find((u) => u.id === id)?.position[0] ?? NaN;
const cells = (page) => lab(page, () => window.__lab.route.refreshGround());
const stream = (page) => lab(page, () => window.__lab.route.ground());
const marked = (g, channel, test = () => true) =>
  g.cells.filter((c) => c.marks[channel] > 0 && test(c)).length;
const near = (px, py, r) => (c) => Math.hypot(c.x + 0.5 - px, c.y + 0.5 - py) < r;
const colourDistance = (a, b) => a.reduce((sum, v, i) => sum + Math.abs(v - b[i]), 0);
const key = (c) => `${c.x},${c.y}`;
const CHANNELS = ["crater", "scorch", "tracks", "trampled"];

export async function run(ctx) {
  if (process.env.SCARS_ONLY) return scarFramings(ctx).then(() => villageInspector(ctx));
  const page = await openBattle(ctx);
  // Past blue's first fog sweep since the bursts (every 6 ticks).
  await advance(page, 6);

  let g = await cells(page);
  const field = (c) => c.x >= 214 && c.x <= 386 && c.y >= 90 && c.y <= 130;
  ctx.check(
    "blue learns the crater field it watched dug, and its scorch",
    marked(g, "crater", field) > 1000 && marked(g, "scorch", field) > 1000,
    JSON.stringify({ craters: marked(g, "crater", field), scorch: marked(g, "scorch", field) }),
  );

  // The race: both tanks leave together; the one crossing craters falls
  // behind while on them, and both arrive.
  let o = await obs(page);
  let lagged = 0;
  const deltas = [];
  // The field's craters arrived in one delta; the race's tracks trickle in.
  await lab(page, () => window.__lab.route.resetLargestDelta());
  for (let t = 0; t < 2700 && (x(o, 0) < 495 || x(o, 1) < 495); t += 30) {
    await advance(page, 30);
    o = await obs(page);
    deltas.push((await stream(page)).patch);
    if (x(o, 0) > 240 && x(o, 0) < 380) lagged = Math.max(lagged, x(o, 1) - x(o, 0));
  }
  ctx.check(
    "craters slow the tank crossing them, slightly",
    lagged > 2 && lagged < 60,
    JSON.stringify({ lagged }),
  );
  ctx.check(
    "craters are never impassable: both tanks arrive",
    x(o, 0) >= 495 && x(o, 1) >= 495,
    JSON.stringify({ crater: x(o, 0), clean: x(o, 1), tick: o.tick }),
  );
  const known = (await cells(page)).cells.length;
  const { largestDelta } = await stream(page);
  ctx.check(
    "blue's ground keeps arriving as deltas of one stream, each a small part of what it knows",
    deltas.every((p) => p.epoch === 1 && p.side === "blue" && !p.full) &&
      largestDelta > 0 &&
      largestDelta < known / 4,
    JSON.stringify({ largestDelta, known, last: deltas.at(-1) }),
  );

  // Let the barrage and the walking squad work, then read blue's view.
  await advance(page, 900);
  g = await cells(page);
  const open = near(470, 380, 30);
  ctx.check(
    "blue learns the barrage's craters round the squad in the open",
    marked(g, "crater", open) > 0,
    JSON.stringify({ craters: marked(g, "crater", open) }),
  );
  ctx.check(
    "moving tanks leave tracks and the squad tramples its path",
    marked(g, "tracks", (c) => c.y > 160 && c.y < 180) > 200 &&
      marked(g, "trampled", (c) => c.y > 220 && c.y < 250) > 100,
    JSON.stringify({ tracks: marked(g, "tracks"), trampled: marked(g, "trampled") }),
  );
  await writeFile(
    ctx.evidencePath("ground-cells.txt"),
    CHANNELS.map((c) => `${c}: ${marked(g, c)} cells`).join("\n") +
      `\nrace lag on the field: ${lagged.toFixed(1)} m\n`,
  );

  // The flat cell view draws blue's learned cells and toggles off cleanly.
  const centre = await lab(page, () => window.__lab.projectToCss(300, 110, 0));
  const shown = decode(await snapshot(ctx, page, "frame-cells-1280x800.png"));
  await lab(page, () => window.__lab.route.show([]));
  await page.evaluate(() => window.__lab.frame());
  const bare = decode(await snapshot(ctx, page, "frame-cells-off-1280x800.png"));
  await lab(page, (c) => window.__lab.route.show(c), CHANNELS);
  await page.evaluate(() => window.__lab.frame());
  // Crater red against grass green: a hue change more than a brightness one.
  const d = colourDistance(pixel(shown, ...centre), pixel(bare, ...centre));
  ctx.check("the cell view draws the crater field", d > 60, JSON.stringify({ d, centre }));

  // Switch to red: a new epoch opens with red's whole learned ground, then deltas.
  await lab(page, () => window.__lab.route.observeAs("red"));
  await advance(page, 1);
  const switched = await stream(page);
  await advance(page, 6);
  const after = await stream(page);
  ctx.check(
    "a side switch sends red's full snapshot in a new epoch, then deltas",
    switched.patch.full &&
      switched.patch.side === "red" &&
      switched.patch.epoch === 2 &&
      !after.patch.full &&
      after.patch.epoch === 2 &&
      after.side === "red",
    JSON.stringify({ switched, after }),
  );
  await lab(page, () => window.__lab.route.observeAs("blue"));
  await advance(page, 1);

  // One live battle page at a time: each holds a GPU device and a worker.
  await page.close();
  await villageInspector(ctx);
  await scarFramings(ctx);
}

/** The lab field's scars at a fixed tick, from fixed cameras (1920 × 1080). */
const SCAR_TICK = 1500;
const SCAR_FRAMINGS = {
  // The crater field's east edge, where the carpet meets clean ground.
  field: { target: [392, 110], distance: 55, pitch: 0.85 },
  // The barrage round the red squads.
  barrage: { target: [468, 330], distance: 75, pitch: 0.85 },
  // The race's tracks leaving the field, and the squad's trampled path.
  tracks: { target: [300, 190], distance: 90, pitch: 0.95 },
  // Low over the barrage: crater relief against the sun.
  ground: { target: [466, 368], distance: 28, pitch: 0.32 },
};
const hidePanel = (page) => page.addStyleTag({ content: ".lab-panel { display: none }" });
const cameraAt = (page, { target, distance, pitch }) =>
  aim(page, target, { distance, pitch, yaw: -1.57 });

async function scarFramings(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 } });
  await hidePanel(page);
  for (let t = (await obs(page)).tick; t < SCAR_TICK; t += 300)
    await advance(page, Math.min(300, SCAR_TICK - t));
  await lab(page, () => window.__lab.route.show([]));
  for (const [name, f] of Object.entries(SCAR_FRAMINGS)) {
    await cameraAt(page, f);
    await snapshot(ctx, page, `scars-${name}-1920x1080.png`);
    await lab(page, () => window.__lab.suppressScars(true));
    await snapshot(ctx, page, `scars-${name}-off-1920x1080.png`);
    await lab(page, () => window.__lab.suppressScars(false));
  }
  const stats = await lab(page, () => window.__lab.stats().scars);
  const blue = await cells(page);
  ctx.check(
    "the scar texture holds the side's grid, uploaded whole once and by tiles since",
    stats.cols === 600 &&
      stats.rows === 440 &&
      stats.fullUploads >= 1 &&
      stats.uploads > stats.fullUploads,
    JSON.stringify(stats),
  );

  // The grass answers the scars: sparse in the crater field, laid over where
  // the squad walked, against clean meadow at the same range.
  await cameraAt(page, { target: [300, 170], distance: 110, pitch: 0.9 });
  await page.evaluate(() => window.__lab.frame());
  const clumps = await lab(page, () => window.__lab.grass().clumps());
  const inBox = (x0, x1, y0, y1) => (c) =>
    c.root[0] >= x0 && c.root[0] < x1 && c.root[1] >= y0 && c.root[1] < y1;
  const density = (test, area) => clumps.filter(test).length / area;
  const field = density(inBox(260, 340, 100, 120), 80 * 20);
  const clean = density(inBox(260, 340, 190, 210), 80 * 20);
  const key = (x, y) => `${x},${y}`;
  const trodden = new Set(
    blue.cells.filter((c) => c.marks.trampled >= 6 && !c.marks.crater).map((c) => key(c.x, c.y)),
  );
  const onPath = clumps.filter((c) =>
    trodden.has(key(Math.floor(c.root[0]), Math.floor(c.root[1]))),
  );
  const offPath = clumps.filter(inBox(260, 340, 190, 210));
  const meanLaid = (list) => list.reduce((s, c) => s + c.laid, 0) / Math.max(1, list.length);
  ctx.check(
    "grass thins in the crater field and lies over on the trampled path",
    clean > 0 &&
      field < clean * 0.35 &&
      onPath.length > 20 &&
      meanLaid(onPath) > 0.3 &&
      meanLaid(offPath) < 0.02,
    JSON.stringify({
      field,
      clean,
      trodden: trodden.size,
      trampledMax: Math.max(...blue.cells.map((c) => c.marks.trampled)),
      onPath: onPath.length,
      laidOn: meanLaid(onPath),
      laidOff: meanLaid(offPath),
    }),
  );
  await page.close();
}

/** The paused village: each side's learned ground is its own. */
async function villageInspector(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page, `${ctx.url}?village`);
  await page.waitForFunction(() => window.__lab.route?.warm(), undefined, { timeout: 240000 });
  const blue = await cells(page);
  const blueStream = await stream(page);
  await lab(page, () => window.__lab.route.observeAs("red"));
  await advance(page, 1);
  const red = await cells(page);
  const redStream = await stream(page);
  const redKeys = new Set(red.cells.map(key));
  const blueKeys = new Set(blue.cells.map(key));
  const onlyBlue = blue.cells.filter((c) => !redKeys.has(key(c))).length;
  const onlyRed = red.cells.filter((c) => !blueKeys.has(key(c))).length;
  ctx.check(
    "the paused village: each side holds ground the other never saw",
    blueStream.side === "blue" &&
      redStream.patch.full &&
      redStream.side === "red" &&
      marked(blue, "crater") + marked(blue, "tracks") > 0 &&
      onlyBlue > 0 &&
      onlyRed > 0,
    JSON.stringify({
      blue: blue.cells.length,
      red: red.cells.length,
      onlyBlue,
      onlyRed,
      blueCraters: marked(blue, "crater"),
      redCraters: marked(red, "crater"),
      tick: (await obs(page)).tick,
    }),
  );
  await sidesDrawTheirOwnScars(ctx, page, blue, red);
  await lab(page, () => window.__lab.route.observeAs("blue"));
  await advance(page, 1);
  await snapshot(ctx, page, "frame-village-blue-1920x1080.png");
  await villageScars(ctx, page, blue);
  await page.close();
}

/** The village after the opening bombardment, as blue learned it: where its
 *  craters and scorch are thickest, at the default and ground framings, with
 *  scars and without (the same paused tick). */
async function villageScars(ctx, page, blue) {
  const burnt = blue.cells.filter((c) => c.marks.crater + c.marks.scorch > 0);
  const around = (c) => burnt.filter((o) => Math.hypot(o.x - c.x, o.y - c.y) < 15).length;
  const densest = burnt.reduce((a, c) => (!a || around(c) > around(a) ? c : a), null);
  if (!densest) return;
  await hidePanel(page);
  await lab(page, () => window.__lab.route.show([]));
  const target = [densest.x + 0.5, densest.y + 0.5];
  for (const [name, framing] of Object.entries({
    default: { target, distance: 65, pitch: 0.85 },
    ground: { target: [target[0], target[1] - 14], distance: 25, pitch: 0.22 },
  })) {
    await cameraAt(page, framing);
    await snapshot(ctx, page, `village-scars-${name}-1920x1080.png`);
    await lab(page, () => window.__lab.suppressScars(true));
    await snapshot(ctx, page, `village-scars-${name}-off-1920x1080.png`);
    await lab(page, () => window.__lab.suppressScars(false));
  }
}

/** Only the observed side's learned ground is drawn: where one side has marks
 *  the other never saw, the ground differs between the two sides' views; where
 *  neither has a mark, it does not. Fog, grass and the lab's flat cell view
 *  are off, so scars are all that can differ. Called observing red. */
async function sidesDrawTheirOwnScars(ctx, page, blue, red) {
  const redKeys = new Set(red.cells.map(key));
  const strength = (m) => 2 * (m.crater + m.scorch) + m.tracks + m.trampled;
  const blueKeys = new Set(blue.cells.map(key));
  const props = (await obs(page)).knownProps;
  const exclusive = [
    ...blue.cells.filter((c) => !redKeys.has(key(c))),
    ...red.cells.filter((c) => !blueKeys.has(key(c))),
  ]
    .filter(
      (c) =>
        c.marks.crater > 0 &&
        props.every(
          (p) =>
            Math.hypot(c.x + 0.5 - p.center[0], c.y + 0.5 - p.center[1]) >
            Math.hypot(p.half[0], p.half[1]) + 4,
        ),
    )
    .sort((a, b) => strength(b.marks) - strength(a.marks))[0];
  const marked = new Set([...blue.cells, ...red.cells].map(key));
  const offset = exclusive
    ? [
        [12, 0],
        [-12, 0],
        [0, 12],
        [0, -12],
      ].find(([dx, dy]) => !marked.has(key({ x: exclusive.x + dx, y: exclusive.y + dy })))
    : null;
  ctx.check(
    "one side holds a crater the other never saw, beside ground neither has marked",
    !!exclusive && !!offset,
    JSON.stringify({ exclusive }),
  );
  if (!exclusive || !offset) return;
  const at = [exclusive.x + 0.5, exclusive.y + 0.5];
  const blank = [at[0] + offset[0], at[1] + offset[1]];
  await lab(page, () => window.__lab.suppressGrass(true));
  await lab(page, () => window.__lab.suppressFog(true));
  await lab(page, () => window.__lab.route.show([]));
  await cameraAt(page, { target: at, distance: 30, pitch: 1.3 });
  const [pc, pb] = [await groundCss(page, at), await groundCss(page, blank)];
  const redShot = decode(await snapshot(ctx, page, "scars-sides-red-1920x1080.png"));
  await lab(page, () => window.__lab.route.observeAs("blue"));
  await advance(page, 1);
  const blueShot = decode(await snapshot(ctx, page, "scars-sides-blue-1920x1080.png"));
  // Within a few pixels of each point: the crater's bowl and rim against grass
  // is a colour change, and the bare ground is the same in both views.
  const mark = mostChanged(blueShot, redShot, pc, 4);
  const bare = mostChanged(blueShot, redShot, pb, 4);
  ctx.check(
    "scars draw only the observed side's learned ground",
    mark > 24 && bare < 6,
    JSON.stringify({ marks: exclusive.marks, mark, bare, at, blank }),
  );
  await lab(page, (c) => window.__lab.route.show(c), CHANNELS);
  await lab(page, () => window.__lab.suppressFog(false));
  await lab(page, () => window.__lab.suppressGrass(false));
  await lab(page, () => window.__lab.route.observeAs("red"));
  await advance(page, 1);
}
