// Battle-look slices 07 and 08: the ground layer's rules in the browser, as
// each side learns them. Craters from authored bursts and a live barrage, a
// tank slowed (never stopped) by a crater field, tracks, trampling and scorch,
// all drawn from blue's learned cells (the ground patches its publications
// carry); a side switch reopens the stream with red's full snapshot. Then the
// paused village inspector: after the supported attack's opening, each side
// holds its own ground.
import { writeFile } from "node:fs/promises";
import { decode, pixel } from "./_png.mjs";
import { lab, obs, advance, snapshot } from "./_lab.mjs";

const x = (o, id) => o.own.find((u) => u.id === id)?.position[0] ?? NaN;
const cells = (page) => lab(page, () => window.__lab.route.refreshGround());
const stream = (page) => lab(page, () => window.__lab.route.ground());
const marked = (g, channel, test = () => true) =>
  g.cells.filter((c) => c.marks[channel] > 0 && test(c)).length;
const near = (px, py, r) => (c) => Math.hypot(c.x + 0.5 - px, c.y + 0.5 - py) < r;
const colourDistance = (a, b) => a.reduce((sum, v, i) => sum + Math.abs(v - b[i]), 0);
const key = (c) => `${c.x},${c.y}`;

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 20000 });
  await lab(page, () => window.__lab.route.pause());
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
    ["crater", "scorch", "tracks", "trampled"]
      .map((c) => `${c}: ${marked(g, c)} cells`)
      .join("\n") + `\nrace lag on the field: ${lagged.toFixed(1)} m\n`,
  );

  // The flat cell view draws blue's learned cells and toggles off cleanly.
  const centre = await lab(page, () => window.__lab.projectToCss(300, 110, 0));
  const shown = decode(await snapshot(ctx, page, "frame-cells-1280x800.png"));
  await lab(page, () => window.__lab.route.show([]));
  await page.evaluate(() => window.__lab.frame());
  const bare = decode(await snapshot(ctx, page, "frame-cells-off-1280x800.png"));
  await lab(page, () => window.__lab.route.show(["crater", "scorch", "tracks", "trampled"]));
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

  await villageInspector(ctx);
}

/** The paused village: each side's learned ground is its own. */
async function villageInspector(ctx) {
  const page = await ctx.newPage();
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
  await lab(page, () => window.__lab.route.observeAs("blue"));
  await advance(page, 1);
  await snapshot(ctx, page, "frame-village-blue-1280x800.png");
}
