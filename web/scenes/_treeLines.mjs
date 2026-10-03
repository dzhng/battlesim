// A tree line (a strip of forest between fields) as it is drawn, checked on
// the generated map's longest one at the ground rig's station across it. The
// simulation lets no far sight through a strip, so the picture must not show
// the field beyond between bare boles: with its shrubs the line hides most
// of the framed forest ground from the play camera. A strip has the plots'
// verge under it, with the verge's grass, rather than a forest floor.
import { writeFile } from "node:fs/promises";
import { decode, pixel } from "./_png.mjs";
import { lab } from "./_lab.mjs";
import { chroma, linear, meanColour } from "./_colour.mjs";
import {
  classPixels,
  classAt,
  openStations,
  pairedCost,
  shoot,
  stationReport,
} from "./_groundStations.mjs";

const MAP = "generated";
const STATION = "tree-line-65";
/** Minimum coverage of forest ground in this tree-line framing. The class
 *  mask identifies forest ground, not an individual forest. */
const HIDDEN = 0.6;
/** A pixel of ground the grass has changed differs by this much in a channel. */
const GRASS_STEP = 12;
/** The share of the strip's ground a blade of the verge's grass changes, at
 *  least: it is the ground's own colour, so most of it changes little. A
 *  wood's floor has none. */
const GRASSED = 0.05;

const understorey = (page, on) => lab(page, (off) => window.__lab.suppressUnderstorey(off), !on);

export async function treeLines(ctx) {
  const page = await openStations(ctx, MAP);
  const capture = async (name, options) => {
    const png = await shoot(page, MAP, STATION, options);
    await writeFile(ctx.evidencePath(`tree-line-${name}.png`), png);
    return decode(png);
  };
  const mask = await capture("mask-bare", { view: "ground-classes", grass: false, trees: false });
  const forest = [...classPixels(mask, (c) => c.forest === "inside")];
  const state = () =>
    lab(page, () => ({ tick: window.__lab.route.tick(), camera: window.__lab.camera() }));
  const controls = [];
  const seen = async (shrubs, name) => {
    await understorey(page, shrubs);
    const under = await capture(name, { view: "ground-classes", grass: false });
    controls.push(await state());
    return forest.map(([x, y]) => classAt(under, x, y)?.forest === "inside");
  };
  const crowns = await seen(false, "mask-crowns");
  const line = await seen(true, "mask-line");
  const hidden = {
    crowns: 1 - crowns.filter(Boolean).length / forest.length,
    line: 1 - line.filter(Boolean).length / forest.length,
  };
  const additionalPixels = crowns.filter((visible, i) => visible && !line[i]).length;
  const stats = await lab(page, () => window.__lab.stats().scenery.understorey);
  ctx.check(
    `a tree line hides at least ${HIDDEN} of the framed forest ground; drawn shrubs hide additional ground beyond its crowns`,
    forest.length > 20000 &&
      stats.tiers.some((n) => n > 0) &&
      hidden.line >= HIDDEN &&
      additionalPixels > 0 &&
      JSON.stringify(controls[0]) === JSON.stringify(controls[1]),
    JSON.stringify({ forestPixels: forest.length, hidden, additionalPixels, drawn: stats.tiers }),
  );
  await capture("final");

  // The ground under it: the verge's colour (green of red, where a wood's
  // floor is brown), with grass on it.
  const bare = await capture("verge-bare", { grass: false, trees: false });
  const grassed = await capture("verge-grass", { trees: false });
  const [a] = chroma(meanColour(bare, forest).rgb.map(linear));
  const grown =
    forest.filter(([x, y]) => {
      const [p, q] = [pixel(bare, x, y), pixel(grassed, x, y)];
      return p.some((v, k) => Math.abs(v - q[k]) > GRASS_STEP);
    }).length / forest.length;
  ctx.check(
    "the ground under a tree line is the plots' verge with its grass, not a wood's bare floor",
    a < 0 && grown >= GRASSED,
    JSON.stringify({ a: +a.toFixed(2), grassed: +grown.toFixed(3) }),
  );
  await ctx.writeEvidence("tree-line.json", {
    forestPixels: forest.length,
    hidden,
    additionalPixels,
    controls,
    identity: stationReport(page).identity,
    historicalShrubTarget: {
      minimum: 0.1,
      observed: hidden.line - hidden.crowns,
      met: hidden.line - hidden.crowns >= 0.1,
    },
    a,
    grown,
    stats,
  });
  await page.close();
}

/** The pairs a station's cost is measured over. */
const COST_PAIRS = 4;

/** TREE_LINE_COST=1: what the shrubs under the tree lines cost a frame from
 *  the tactical camera and the play camera (`pairedCost`), and how many
 *  stand on the map. */
export async function treeLineCost(ctx) {
  const page = await openStations(ctx, MAP);
  const result = { stations: {} };
  for (const station of ["tree-line-250", STATION]) {
    await shoot(page, MAP, station);
    const cost = await pairedCost(page, "suppressUnderstorey", COST_PAIRS);
    await understorey(page, true);
    await lab(page, () => window.__lab.frame());
    result.stations[station] = {
      bareMs: +cost.plainMs.toFixed(3),
      shrubsMs: +cost.costMs.toFixed(3),
      differences: cost.differences.map((v) => +v.toFixed(3)),
      drawn: await lab(page, () => window.__lab.stats().scenery.understorey),
    };
  }
  result.adapter = await page.evaluate(() => window.__lab.adapter);
  await ctx.writeEvidence("tree-line-cost.json", result);
  ctx.check(
    "the shrubs under the tree lines cost under half a millisecond of GPU a frame",
    Object.values(result.stations).every((s) => s.shrubsMs < 0.5),
    JSON.stringify(result.stations),
  );
  await page.close();
}
