// How a generated town and the plain round it read together (C31), on the
// ground rig's generated map: what the terrain's own class mask says the
// ground between the buildings is, what the plain beyond is still, and how
// the country road through the town is drawn either side of the place it
// leaves it.
import {
  BIOME,
  classAt,
  classPixels,
  drawnRoads,
  groundUnder,
  oneIn,
  openStations,
  shoot,
  stationFrame,
  stationReport,
} from "./_groundStations.mjs";
import { luminance, warmth } from "./_colour.mjs";
import { lab } from "./_lab.mjs";
import { pixel } from "./_png.mjs";

const YARD = BIOME.field_rules.settlement_kind;
const street = BIOME.roads[BIOME.roads.default.town.kind];

/** Open ground: this far from any paving, clear of its shoulder or walk. */
const OFF_ROAD_M = 5;
/** A road's core: this far inside its edge. */
const CORE_M = -1;
/** A road's surface is judged this far from where its kind changes: past
 *  the drifts of its join. */
const JOIN_CLEAR_M = 12;
/** A group is judged on at least this many pixels. */
const GROUP_PIXELS = 150;

/** What share of `mask`'s open ground each plot kind covers, by name. */
function plotShares(mask) {
  const counts = {};
  let ground = 0;
  for (let y = 0; y < mask.height; y += 2)
    for (let x = 0; x < mask.width; x += 2) {
      const c = classAt(mask, x, y);
      if (!c || c.forest !== "none" || c.riverSd < 3 || c.roadSd < OFF_ROAD_M) continue;
      const name = BIOME.plots[c.plotKind]?.name ?? "?";
      counts[name] = (counts[name] ?? 0) + 1;
      ground++;
    }
  const share = (keep) =>
    Object.entries(counts).reduce((sum, [name, n]) => sum + (keep(name) ? n : 0), 0) /
    Math.max(1, ground);
  const drilled = (name) => (BIOME.plots.find((p) => p.name === name)?.furrow_m ?? 0) > 0;
  return {
    ground,
    yard: +share((name) => name === YARD).toFixed(4),
    drilled: +share(drilled).toFixed(4),
    crops: Object.keys(counts).filter(drilled).sort(),
  };
}

/** The pixels of `mask` whose class passes `keep`, one in `every`. */
const sample = (mask, every, keep) => oneIn(every, classPixels(mask, keep));

/** A group of pixels read in the sun: the median of its brighter half, and
 *  how far that colour leans from blue toward red. */
function lit(shot, pixels) {
  const sorted = pixels
    .map((p) => pixel(shot, ...p))
    .sort((a, b) => luminance(a) - luminance(b))
    .slice(Math.floor(pixels.length / 2));
  return {
    count: pixels.length,
    luminance: +luminance(sorted[Math.floor(sorted.length / 2)] ?? [0, 0, 0]).toFixed(4),
    warmth: +warmth(sorted[Math.floor(sorted.length / 2)] ?? [1, 1, 1]).toFixed(3),
  };
}

export async function townGround(ctx) {
  const page = await openStations(ctx, "generated");
  const masks = {};
  for (const station of ["town-250", "town-edge-250", "country-250"])
    masks[station] = plotShares((await stationFrame(page, "generated", station)).mask);
  ctx.check(
    "between a town's buildings the ground is its yards and commons: no crop is drilled there",
    masks["town-250"].ground > 50000 &&
      masks["town-250"].yard > 0.8 &&
      masks["town-250"].drilled < 0.005,
    JSON.stringify(masks["town-250"]),
  );
  ctx.check(
    "the plain beyond the town keeps its drilled fields, and no yard lies out in it",
    masks["country-250"].ground > 50000 &&
      masks["country-250"].drilled > 0.25 &&
      masks["country-250"].crops.length >= 2 &&
      masks["country-250"].yard === 0,
    JSON.stringify(masks["country-250"]),
  );
  ctx.check(
    "the town's edge is in one frame: its last yards on one side, drilled fields on the other",
    // How much of the frame is drilled depends on which fields lie past the
    // meadow that surrounds a town: some, of a crop or more.
    masks["town-edge-250"].yard > 0.15 &&
      masks["town-edge-250"].drilled > 0.03 &&
      masks["town-edge-250"].crops.length >= 1,
    JSON.stringify(masks["town-edge-250"]),
  );

  // The country road where it leaves the town: its surface either side of
  // the join, each pixel put on the stretch the renderer's own strokes say
  // it is, and the grass beside each.
  // Compare the two materials under the same light: buildings cast shadow
  // over the gravel stretch, which would otherwise measure shade as albedo.
  let join;
  await lab(page, () => window.__lab.suppressBuildings(true));
  try {
    join = await stationFrame(page, "generated", "road-join-65");
  } finally {
    await lab(page, () => window.__lab.suppressBuildings(false));
  }
  const { edges } = stationReport(page);
  const clear = ({ xy }) => Math.hypot(xy[0] - edges.join[0], xy[1] - edges.join[1]) > JOIN_CLEAR_M;
  const cores = sample(join.mask, 7, (c) => c.roadSd < CORE_M);
  const world = await groundUnder(page, cores);
  const drawn = await drawnRoads(
    page,
    world.map(({ xy }) => xy),
  );
  const surface = (kind) =>
    lit(
      join.shot,
      cores.filter((_, i) => drawn[i]?.kind === kind && clear(world[i])),
    );
  const [asphalt, gravel] = [surface("road"), surface("country_road")];
  ctx.check(
    "the country road is a street as far as the town's last yards and gravel beyond: darker and greyer in town",
    asphalt.count >= GROUP_PIXELS &&
      gravel.count >= GROUP_PIXELS &&
      asphalt.luminance <= 0.85 * gravel.luminance &&
      asphalt.warmth < gravel.warmth,
    JSON.stringify({ asphalt, gravel }),
  );
  // Grass: the street has a walk, bare; the country road a shoulder, where
  // the field's grass thins but grows.
  await shoot(page, "generated", "road-join-65");
  const roots = await lab(page, async () => {
    await window.__lab.frame();
    return (await window.__lab.grass().clumps()).map((c) => [c.root[0], c.root[1]]);
  });
  const beside = (await drawnRoads(page, roots)).filter(
    (road, i) => road && clear({ xy: roots[i] }),
  );
  const stats = await lab(page, () => window.__lab.stats().grass);
  await page.close();
  /** Clumps from `from` to `to` metres outside a road drawn as `kind`. */
  const band = (kind, from, to) =>
    beside.filter((road) => road.kind === kind && -road.inside >= from && -road.inside < to).length;
  const walk = street.walk.width_m;
  const grown = {
    onRoad: beside.filter((road) => road.inside > 0.3).length,
    walk: band("road", 0.3, walk - 0.3),
    pastWalk: band("road", walk + 1, walk + 6),
    shoulder: band("country_road", 0.5, walk),
  };
  ctx.check(
    "grass grows only off the paving: none on either road nor on the street's walk, and the country road's shoulder keeps some",
    roots.length > 1000 &&
      grown.onRoad === 0 &&
      grown.walk === 0 &&
      grown.pastWalk > 20 &&
      grown.shoulder > 20,
    JSON.stringify({ clumps: roots.length, grown }),
  );
  await ctx.writeEvidence("town-ground.json", { masks, asphalt, gravel, grown, grass: stats });
}
