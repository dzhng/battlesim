// How the roads read (C66 to C68), measured on the ground rig's stations: each
// station's frame beside the terrain's own class mask, so a band is the
// pixels the material says it is, never a hand-drawn crop.
import {
  BIOME,
  STATION_MAPS,
  classPixels,
  fixture,
  groundUnder,
  isOpen,
  openStations,
  pairedCost,
  shoot,
  stationFrame,
  villageExport,
} from "./_groundStations.mjs";
import { chroma, linear, meanColour, warmth } from "./_colour.mjs";
import { aim, lab } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";

/** The biome's road row for a map's `n`th surface, and that surface's half
 *  width. */
function roadOf(map, n) {
  const roads = BIOME.roads;
  const surface = fixture(`maps/${map}/map.json`).surfaces[n];
  return { row: roads[surface.kind] ?? roads.default, half: surface.shape.width_m / 2 };
}

/** A road's core: this far inside its edge, clear of the feather. */
const CORE_M = -0.5;
/** The grass beside a road: this far outside its edge, past anything worn. */
const GRASS_M = [5, 9];
/** A band is judged on at least this many pixels. */
const BAND_PIXELS = 2000;
/** The 1 m bands from a country road's edge that its shoulder covers. */
const SHOULDER_BANDS = 4;
/** The road-to-field transition is walked in bands this wide, from inside
 *  the core out to the grass. */
const STEP_M = 0.5;
const WALK_M = [-1, GRASS_M[0]];
/** A band may be this much darker than the darker of the core and the field
 *  and not count as a trough: open field differs from band to band by which
 *  plots and furrows lie in each, and the bare half metre beside a track
 *  through a flowering crop is the field's own soil, a tenth darker than its
 *  flowers. */
const FIELD_LEVEL = 0.12;

/** The pixels of `mask` that are open ground (no wood, no bank) and pass
 *  `keep(class)`. */
const groundPixels = (mask, keep) => classPixels(mask, (c) => isOpen(c) && keep(c));

/** Two grounds are apart in hue when their mean colours differ by this much
 *  in the a*b* plane (the river bank's bar), and a road may then be this
 *  share of its neighbour's luminance: a brown track through rapeseed is not
 *  a shadow on it. */
const HUE_APART = 6;
const DARKER_APART = 0.85;

/** L-G3 at one station: the road's core against the grass beside it. */
function coreAgainstGrass({ shot, mask }) {
  const core = meanColour(
    shot,
    groundPixels(mask, (c) => c.roadSd < CORE_M),
  );
  const grass = meanColour(
    shot,
    groundPixels(mask, (c) => c.roadSd >= GRASS_M[0] && c.roadSd <= GRASS_M[1]),
  );
  const [a, b] = [core.rgb, grass.rgb].map((rgb) => chroma(rgb.map(linear)));
  return {
    core: core.luminance,
    grass: grass.luminance,
    apart: +Math.hypot(a[0] - b[0], a[1] - b[1]).toFixed(1),
    pixels: [core.count, grass.count],
  };
}

/** The walk from a road's core out to the grass in `shot`: each band's mean
 *  luminance, the grass band's, and whether no band dips under both the core
 *  and the grass. */
function walk(shot, mask) {
  const bands = [];
  for (let from = WALK_M[0]; from < WALK_M[1]; from += STEP_M)
    bands.push(
      meanColour(
        shot,
        groundPixels(mask, (c) => c.roadSd >= from && c.roadSd < from + STEP_M),
      ),
    );
  const grass = meanColour(
    shot,
    groundPixels(mask, (c) => c.roadSd >= GRASS_M[0] && c.roadSd <= GRASS_M[1]),
  ).luminance;
  const light = bands.map((b) => b.luminance);
  return {
    light: light.map((v) => +v.toFixed(4)),
    grass: +grass.toFixed(4),
    noTrough: light.every((v) => v >= Math.min(light[0], grass) * (1 - FIELD_LEVEL)),
    counted: bands.every((b) => b.count >= BAND_PIXELS / 4),
  };
}

/** A band of a road's core `m` metres from its centreline (half width
 *  `half`), `within` metres either way: the mask's distance is from the
 *  road's edge. */
const lane = (mask, half, m, within) => [
  ...groundPixels(mask, (c) => Math.abs(-c.roadSd - (half - m)) <= within),
];

/** How much darker a road's ruts are than its surface beside them (midway
 *  from a rut's outer side to the road's edge), as a share of that surface,
 *  in a bare-ground frame. */
function rutContrast({ bare, mask }, { row, half }) {
  const outer = Math.max(...row.ruts.offsets_m) + row.ruts.width_m / 2;
  const ruts = row.ruts.offsets_m.flatMap((m) => lane(mask, half, m, 0.07));
  const between = lane(mask, half, (outer + half) / 2, 0.15);
  return {
    contrast: 1 - meanColour(bare, ruts).luminance / meanColour(bare, between).luminance,
    pixels: [ruts.length, between.length],
  };
}

/** How green over red a track's middle is against its ruts, in a
 *  bare-ground frame. */
function stripGreen({ bare, mask }, { row, half }) {
  const green = (pixels) => {
    const { rgb, count } = meanColour(bare, pixels);
    return { ratio: rgb[1] / rgb[0], count };
  };
  return {
    middle: green(lane(mask, half, 0, row.centre_strip.half_width_m / 2)),
    ruts: green(lane(mask, half, row.ruts.offsets_m[0], 0.07)),
  };
}

/** Clumps per metre-wide band beside the village's straight road east of the
 *  bend, from its edge outward: the band's distance is the export's. */
async function clumpBands(page) {
  await shoot(page, "village", "bend-65");
  const roots = await lab(page, async () =>
    (await window.__lab.grass().clumps())
      .map((c) => [c.root[0], c.root[1]])
      .filter(([x, y]) => x > 445 && x < 480 && Math.abs(y - 420) < 16),
  );
  // A pixel as wide as the map reads every distance exactly.
  const paved = await villageExport(
    page,
    roots.map((xy) => ({ xy, footprint: 1e9 })),
  );
  const bands = Array.from({ length: 8 }, () => 0);
  for (const { paved: inside } of paved) {
    const band = Math.floor(-inside);
    if (inside < 0 && band < bands.length) bands[band]++;
  }
  return bands;
}

/** The mean colour difference (summed over channels) between two shots at
 *  pairs of pixels. */
function difference(a, b, pairs) {
  let sum = 0;
  for (const [p, q] of pairs)
    sum += pixel(a, ...p).reduce((s, v, k) => s + Math.abs(v - pixel(b, ...q)[k]), 0);
  return sum / pairs.length;
}

/** Whether the worn ground beside the road is painted on the world: the same
 *  ground from a camera moved a few metres shows the same wear at the same
 *  world points, and not at the same pixels. */
async function wornGroundStaysPut(page, first) {
  const pixels = [...groundPixels(first.mask, (c) => c.roadSd > 0.25 && c.roadSd < 3)].filter(
    ([x, y], i) => i % 211 === 0 && x > 200 && x < 1720 && y > 150 && y < 930,
  );
  const world = await groundUnder(page, pixels);
  const pose = STATION_MAPS.village.stations["bend-65"];
  await aim(page, [pose.target[0] + 3.7, pose.target[1] - 2.3], pose);
  await lab(page, () => window.__lab.frame());
  const moved = decode(await page.screenshot());
  const canvas = await page.evaluate(() => {
    const box = document.querySelector("canvas").getBoundingClientRect();
    return [box.left, box.top];
  });
  const projected = await lab(
    page,
    (points) =>
      points.map(([x, y]) => window.__lab.projectToCss(x, y, window.__lab.route.surfaceZ(x, y))),
    world.map((w) => w.xy),
  );
  const pairs = [];
  pixels.forEach((p, i) => {
    const q = [
      Math.round(projected[i][0] - canvas[0] - 0.5),
      Math.round(projected[i][1] - canvas[1] - 0.5),
    ];
    if (q[0] >= 0 && q[1] >= 0 && q[0] < moved.width && q[1] < moved.height) pairs.push([p, q]);
  });
  return {
    points: pairs.length,
    sameWorld: difference(first.bare, moved, pairs),
    samePixel: difference(
      first.bare,
      moved,
      pairs.map(([p]) => [p, p]),
    ),
  };
}

/** The fields' own texture off: a road is judged against plain ground, its
 *  one variable. A furrow or a wheeling lying along a road's edge would put
 *  its own stripe in a band of the walk. */
const plainFields = (page) => lab(page, () => window.__lab.suppressFieldTexture(true));

export async function roadLooks(ctx) {
  const village = await openStations(ctx, "village");
  await plainFields(village);
  const bend = await stationFrame(village, "village", "bend-65");
  const bands = { "village bend-65": coreAgainstGrass(bend) };
  const walks = {
    "village bend-65 bare": walk(bend.bare, bend.mask),
    "village bend-65": walk(bend.shot, bend.mask),
  };
  const clumps = await clumpBands(village);
  // Grass off again, as `bend.bare` was shot.
  await shoot(village, "village", "bend-65", { grass: false, trees: false });
  const anchored = await wornGroundStaysPut(village, bend);
  await village.close();
  const river = await openStations(ctx, "river");
  await plainFields(river);
  const track = await stationFrame(river, "river", "track-65");
  bands["river track-65"] = coreAgainstGrass(track);
  walks["river track-65 bare"] = walk(track.bare, track.mask);
  walks["river track-65"] = walk(track.shot, track.mask);
  // The lab's dirt track, from the ground camera and the tactical one.
  const dirtTrack = roadOf("river", 1);
  const close = await stationFrame(river, "river", "track-25");
  const strip = stripGreen(close, dirtTrack);
  const far = await stationFrame(river, "river", "track-250");
  const farStrip = stripGreen(far, dirtTrack);
  const ruts = {
    "track-25": rutContrast(close, dirtTrack),
    "track-250": rutContrast(far, dirtTrack),
  };
  const junction = await stationFrame(river, "river", "junction-65");
  bands["river junction-65"] = coreAgainstGrass(junction);
  ctx.check(
    "no road's core is darker than the ground beside it, unless it is clearly apart from it in hue (nothing on the ground reads as shadow)",
    Object.values(bands).every(
      (b) =>
        (b.core >= b.grass || (b.core >= DARKER_APART * b.grass && b.apart > HUE_APART)) &&
        b.pixels.every((count) => count >= BAND_PIXELS),
    ),
    JSON.stringify(bands),
  );

  // The junction holds both kinds: the country road runs north along x = 60
  // (9 m wide), and the dirt track leaves it eastward.
  const core = [...groundPixels(junction.mask, (c) => c.roadSd < CORE_M)].filter(
    (_, i) => i % 97 === 0,
  );
  const world = await groundUnder(river, core);
  const on = (keep) => core.filter((_, i) => keep(world[i].xy));
  const gravel = meanColour(
    junction.shot,
    on(([x]) => Math.abs(x - 60) < 4),
  );
  const dirt = meanColour(
    junction.shot,
    on(([x]) => x > 67),
  );
  ctx.check(
    "each road kind is drawn as itself: the dirt track warm brown beside the country road's grey",
    gravel.count > 100 &&
      dirt.count > 100 &&
      dirt.rgb[0] > dirt.rgb[1] &&
      dirt.rgb[1] > dirt.rgb[2] &&
      warmth(dirt.rgb) > warmth(gravel.rgb) + 0.08,
    JSON.stringify({
      road: { ...gravel, warmth: warmth(gravel.rgb) },
      track: { ...dirt, warmth: warmth(dirt.rgb) },
    }),
  );
  await river.close();

  ctx.check(
    "from a road's core out to the field no band is darker than both the core and the field: a road has no dark outline",
    Object.values(walks).every((w) => w.noTrough && w.counted),
    JSON.stringify(walks),
  );
  // The field's own density, from the two bands farthest out.
  const field = (clumps[6] + clumps[7]) / 2;
  ctx.check(
    "grass thins across the shoulder: more clumps band by band from the road's edge, few at the edge",
    field > 100 &&
      // Across the shoulder the bands only thicken; past it the field's own
      // patches make a band a fifth thinner or thicker than the next.
      clumps.slice(0, SHOULDER_BANDS).every((count, i) => i === 0 || count >= clumps[i - 1]) &&
      clumps.slice(SHOULDER_BANDS).every((count) => count > 0.7 * field) &&
      clumps[0] < 0.45 * field &&
      clumps[1] < 0.85 * field,
    JSON.stringify({ clumps, field }),
  );
  ctx.check(
    "a track's ruts show from the ground camera and are gone at the tactical one",
    ruts["track-25"].contrast > 0.02 &&
      Math.abs(ruts["track-250"].contrast) < 0.006 &&
      Object.values(ruts).every((r) => r.pixels.every((count) => count > 200)),
    JSON.stringify(ruts),
  );
  ctx.check(
    "a narrow dirt track's middle is a strip of grass between its ruts, and plain track at the tactical camera",
    strip.middle.count > 300 &&
      strip.ruts.count > 300 &&
      strip.middle.ratio > strip.ruts.ratio + 0.05 &&
      farStrip.middle.count > 300 &&
      Math.abs(farStrip.middle.ratio - farStrip.ruts.ratio) < 0.02,
    JSON.stringify({ "track-25": strip, "track-250": farStrip }),
  );
  ctx.check(
    "the worn ground beside a road is anchored to the world, not to the screen",
    anchored.points > 100 &&
      anchored.sameWorld < 12 &&
      anchored.sameWorld < 0.6 * anchored.samePixel,
    JSON.stringify(anchored),
  );
}

/** The pairs a station's cost is measured over. */
const COST_PAIRS = 4;

/** ROAD_COST=1: what the roads' wear (their surface detail, their shoulders,
 *  the grass thinned across them, a street's curb, lines and slab joints)
 *  costs a frame, at stations where roads fill much of the view
 *  (`pairedCost`). */
export async function roadCost(ctx) {
  const result = { stations: {} };
  for (const [map, stations] of [
    ["village", ["bend-65"]],
    ["generated", ["town-65", "junction-65", "road-join-65"]],
  ]) {
    const page = await openStations(ctx, map);
    for (const station of stations) {
      await shoot(page, map, station);
      const cost = await pairedCost(page, "suppressRoadWear", COST_PAIRS);
      result.stations[`${map} ${station}`] = {
        plainMs: +cost.plainMs.toFixed(3),
        wearMs: +cost.costMs.toFixed(3),
        differences: cost.differences.map((v) => +v.toFixed(3)),
      };
    }
    result.adapter = await page.evaluate(() => window.__lab.adapter);
    await page.close();
  }
  await ctx.writeEvidence("road-cost.json", result);
  ctx.check(
    "the roads' wear costs under half a millisecond a frame wherever a road fills the view",
    Object.values(result.stations).every((s) => s.wearMs < 0.5),
    JSON.stringify(result.stations),
  );
}
