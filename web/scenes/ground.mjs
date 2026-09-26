// Battle-look slice 07: the ground layer's rules in the browser. Craters from
// authored bursts and a live barrage, a tank slowed (never stopped) by a
// crater field, tracks, trampling and scorch, and the flat cell debug view.
import { writeFile } from "node:fs/promises";
import { decode, pixel } from "./_png.mjs";
import { lab, obs, advance, snapshot } from "./_lab.mjs";

const x = (o, id) => o.own.find((u) => u.id === id)?.position[0] ?? NaN;
const cells = (page) => lab(page, () => window.__lab.route.refreshGround());
const marked = (g, channel, test = () => true) =>
  g.cells.filter((c) => c.marks[channel] > 0 && test(c)).length;
const near = (px, py, r) => (c) => Math.hypot(c.x + 0.5 - px, c.y + 0.5 - py) < r;
const colourDistance = (a, b) => a.reduce((sum, v, i) => sum + Math.abs(v - b[i]), 0);

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 20000 });
  await lab(page, () => window.__lab.route.pause());

  let g = await cells(page);
  const field = (c) => c.x >= 214 && c.x <= 386 && c.y >= 90 && c.y <= 130;
  ctx.check(
    "authored bursts dig the crater field and scorch it",
    marked(g, "crater", field) > 1000 && marked(g, "scorch", field) > 1000,
    JSON.stringify({ craters: marked(g, "crater", field), scorch: marked(g, "scorch", field) }),
  );

  // The race: both tanks leave together; the one crossing craters falls
  // behind while on them, and both arrive.
  let o = await obs(page);
  let lagged = 0;
  for (let t = 0; t < 2700 && (x(o, 0) < 495 || x(o, 1) < 495); t += 30) {
    await advance(page, 30);
    o = await obs(page);
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

  // Let the barrage and the walking squad work, then read the layer.
  await advance(page, 900);
  g = await cells(page);
  const open = near(470, 380, 30);
  ctx.check(
    "the barrage digs craters round the squad in the open",
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

  // The flat cell view draws the layer and toggles off cleanly.
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
}
