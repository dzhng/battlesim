// A tree line (a strip of forest between fields) as it is drawn, checked on
// the generated map's longest one at the ground rig's station across it. The
// simulation lets no far sight through a strip, so the picture must not show
// the field beyond between bare boles: with its shrubs the line hides most
// of its own ground from the play camera. And a strip has no forest floor:
// the ground under it is the plots' verge, with the verge's grass.
import { decode, pixel } from "./_png.mjs";
import { lab } from "./_lab.mjs";
import { chroma, linear, meanColour } from "./_colour.mjs";
import { classPixels, classAt, openStations, pairedCost, shoot } from "./_groundStations.mjs";

const MAP = "generated";
const STATION = "tree-line-65";
/** The share of the strip's own ground the line hides from the play camera
 *  with its shrubs, at least; and how much of that the shrubs add to what
 *  the crowns alone hide. */
const HIDDEN = 0.6;
const SHRUBS_ADD = 0.1;
/** A pixel of ground the grass has changed differs by this much in a channel. */
const GRASS_STEP = 12;
/** The share of the strip's ground a blade of the verge's grass changes, at
 *  least: it is the ground's own colour, so most of it changes little. A
 *  wood's floor has none. */
const GRASSED = 0.05;

const understorey = (page, on) => lab(page, (off) => window.__lab.suppressUnderstorey(off), !on);

export async function treeLines(ctx) {
  const page = await openStations(ctx, MAP);
  // The strip's own ground under these pixels, from the terrain's mask.
  const mask = decode(
    await shoot(page, MAP, STATION, { view: "ground-classes", grass: false, trees: false }),
  );
  const strip = [...classPixels(mask, (c) => c.forest === "inside")];
  /** The share of the strip's ground still seen with the trees drawn. */
  const seen = async (shrubs) => {
    await understorey(page, shrubs);
    const under = decode(await shoot(page, MAP, STATION, { view: "ground-classes", grass: false }));
    return (
      strip.filter(([x, y]) => classAt(under, x, y)?.forest === "inside").length / strip.length
    );
  };
  const hidden = { crowns: 1 - (await seen(false)), line: 1 - (await seen(true)) };
  const stats = await lab(page, () => window.__lab.stats().scenery.understorey);
  ctx.check(
    `a tree line hides its own ground from the play camera: ${HIDDEN} of it or more, its shrubs ${SHRUBS_ADD} of it or more beyond the crowns`,
    strip.length > 20000 &&
      stats.placed > 0 &&
      hidden.line >= HIDDEN &&
      hidden.line - hidden.crowns >= SHRUBS_ADD,
    JSON.stringify({ strip: strip.length, hidden, placed: stats.placed }),
  );

  // The ground under it: the verge's colour (green of red, where a wood's
  // floor is brown), with grass on it.
  const bare = decode(await shoot(page, MAP, STATION, { grass: false, trees: false }));
  const grassed = decode(await shoot(page, MAP, STATION, { trees: false }));
  const [a] = chroma(meanColour(bare, strip).rgb.map(linear));
  const grown =
    strip.filter(([x, y]) => {
      const [p, q] = [pixel(bare, x, y), pixel(grassed, x, y)];
      return p.some((v, k) => Math.abs(v - q[k]) > GRASS_STEP);
    }).length / strip.length;
  ctx.check(
    "the ground under a tree line is the plots' verge with its grass, not a wood's bare floor",
    a < 0 && grown >= GRASSED,
    JSON.stringify({ a: +a.toFixed(2), grassed: +grown.toFixed(3) }),
  );
  await ctx.writeEvidence("tree-line.json", { strip: strip.length, hidden, a, grown, stats });
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
