// How the roads read (C66 to C68), measured on the ground rig's stations: each
// station's frame beside the terrain's own class mask, so a band is the
// pixels the material says it is, never a hand-drawn crop.
import {
  STATION_MAPS,
  classAt,
  groundUnder,
  openStations,
  shoot,
  villageExport,
} from "./_groundStations.mjs";
import { readFileSync } from "node:fs";
import { aim, lab } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";

const fixture = (path) =>
  JSON.parse(readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8"));
/** The biome's road row for a map's `n`th surface, and that surface's half
 *  width. */
function roadOf(map, n) {
  const roads = fixture("biomes/summer.json").roads;
  const surface = fixture(`maps/${map}/map.json`).surfaces[n];
  return { row: roads[surface.kind] ?? roads.default, half: surface.shape.width_m / 2 };
}

/** A road's core: this far inside its edge, clear of the feather. */
const CORE_M = -0.5;
/** The grass beside a road: this far outside its edge, past anything worn. */
const GRASS_M = [5, 9];
/** A band is judged on at least this many pixels. */
const BAND_PIXELS = 2000;
/** The road-to-field transition is walked in bands this wide, from inside
 *  the core out to the grass. */
const STEP_M = 0.5;
const WALK_M = [-1, GRASS_M[0]];
/** A band may be this much brighter than the one inside it and still count
 *  as level: the grain, and blades over the ground. */
const LEVEL = 0.03;
/** A band may be this much darker than the grass band and not count as under
 *  it: open field differs that much from band to band by which plots lie in
 *  each. */
const FIELD_LEVEL = 0.08;

const linear = (v) => {
  const c = v / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
};
/** A displayed pixel's relative luminance. */
const luminance = ([r, g, b]) => 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);

/** The pixels of `mask` that are open ground (no wood, no bank) and pass
 *  `keep(class)`. */
function* groundPixels(mask, keep) {
  for (let y = 0; y < mask.height; y++)
    for (let x = 0; x < mask.width; x++) {
      const c = classAt(mask, x, y);
      if (c && c.forest === "none" && c.riverSd > 3 && keep(c)) yield [x, y];
    }
}

/** The mean displayed colour and luminance of `shot` over `pixels`. */
function mean(shot, pixels) {
  const sum = [0, 0, 0];
  let light = 0,
    count = 0;
  for (const [x, y] of pixels) {
    const rgb = pixel(shot, x, y);
    rgb.forEach((v, k) => (sum[k] += v));
    light += luminance(rgb);
    count++;
  }
  return { count, rgb: sum.map((v) => v / count), luminance: light / count };
}

/** How far a colour leans from blue toward red, as a share of its red. */
const warmth = ([r, , b]) => (r - b) / r;

/** A station's frame, its bare ground and its class mask. */
async function frame(page, map, station) {
  return {
    mask: decode(await shoot(page, map, station, { view: "ground-classes" })),
    bare: decode(await shoot(page, map, station, { grass: false, trees: false })),
    shot: decode(await shoot(page, map, station)),
  };
}

/** L-G3 at one station: the road's core against the grass beside it. */
function coreAgainstGrass({ shot, mask }) {
  const core = mean(
    shot,
    groundPixels(mask, (c) => c.roadSd < CORE_M),
  );
  const grass = mean(
    shot,
    groundPixels(mask, (c) => c.roadSd >= GRASS_M[0] && c.roadSd <= GRASS_M[1]),
  );
  return { core: core.luminance, grass: grass.luminance, pixels: [core.count, grass.count] };
}

/** The walk from a road's core out to the grass in `shot`: each band's mean
 *  luminance, the grass band's, and whether the walk only ever falls (or
 *  holds level) and never dips under the grass. */
function walk(shot, mask) {
  const bands = [];
  for (let from = WALK_M[0]; from < WALK_M[1]; from += STEP_M)
    bands.push(
      mean(
        shot,
        groundPixels(mask, (c) => c.roadSd >= from && c.roadSd < from + STEP_M),
      ),
    );
  const grass = mean(
    shot,
    groundPixels(mask, (c) => c.roadSd >= GRASS_M[0] && c.roadSd <= GRASS_M[1]),
  ).luminance;
  const light = bands.map((b) => b.luminance);
  return {
    light: light.map((v) => +v.toFixed(4)),
    grass: +grass.toFixed(4),
    falls: light.every((v, i) => i === 0 || v <= light[i - 1] * (1 + LEVEL)),
    aboveGrass: light.every((v) => v >= grass * (1 - FIELD_LEVEL)),
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
    contrast: 1 - mean(bare, ruts).luminance / mean(bare, between).luminance,
    pixels: [ruts.length, between.length],
  };
}

/** How green over red a track's middle is against its ruts, in a
 *  bare-ground frame. */
function stripGreen({ bare, mask }, { row, half }) {
  const green = (pixels) => {
    const { rgb, count } = mean(bare, pixels);
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

export async function roadLooks(ctx) {
  const village = await openStations(ctx, "village");
  const bend = await frame(village, "village", "bend-65");
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
  const track = await frame(river, "river", "track-65");
  bands["river track-65"] = coreAgainstGrass(track);
  walks["river track-65 bare"] = walk(track.bare, track.mask);
  walks["river track-65"] = walk(track.shot, track.mask);
  // The lab's dirt track, from the ground camera and the tactical one.
  const dirtTrack = roadOf("river", 1);
  const close = await frame(river, "river", "track-25");
  const strip = stripGreen(close, dirtTrack);
  const far = await frame(river, "river", "track-250");
  const farStrip = stripGreen(far, dirtTrack);
  const ruts = {
    "track-25": rutContrast(close, dirtTrack),
    "track-250": rutContrast(far, dirtTrack),
  };
  const junction = await frame(river, "river", "junction-65");
  bands["river junction-65"] = coreAgainstGrass(junction);
  ctx.check(
    "no road's core is darker than the grass beside it (nothing on the ground reads as shadow)",
    Object.values(bands).every(
      (b) => b.core >= b.grass && b.pixels.every((count) => count >= BAND_PIXELS),
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
  const gravel = mean(
    junction.shot,
    on(([x]) => Math.abs(x - 60) < 4),
  );
  const dirt = mean(
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
    "from a road's core out to the grass the ground only gets darker, band by band, and never darker than the grass",
    Object.values(walks).every((w) => w.falls && w.aboveGrass && w.counted),
    JSON.stringify(walks),
  );
  // The field's own density, from the two bands farthest out.
  const field = (clumps[6] + clumps[7]) / 2;
  ctx.check(
    "grass thins across the shoulder: more clumps band by band from the road's edge, few at the edge",
    field > 200 &&
      clumps.every((count, i) => i === 0 || count >= clumps[i - 1] * 0.9) &&
      clumps[0] < 0.35 * field &&
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

/** Frames a cost batch draws before its GPU time is read, and the pairs a
 *  station is measured over. */
const COST_FRAMES = 120;
const COST_PAIRS = 4;
const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];

/** ROAD_COST=1: what the roads' wear (their surface detail, their shoulders,
 *  the grass thinned across them) costs a frame, at two stations where roads
 *  fill much of the view: the same frozen frame with the wear on and off in
 *  a few interleaved batches, the median of the paired differences. Run it
 *  alone, under the GPU lock. */
export async function roadCost(ctx) {
  const result = { stations: {} };
  for (const [map, stations] of [
    ["village", ["bend-65"]],
    ["generated", ["town-65"]],
  ]) {
    const page = await openStations(ctx, map);
    for (const station of stations) {
      await shoot(page, map, station);
      // The page draws on demand, and the timer's mean runs over the frames
      // drawn: a view change resets it, then every frame is forced.
      const batch = (off) =>
        lab(
          page,
          async ({ off, frames }) => {
            await window.__lab.suppressRoadWear(off);
            await window.__lab.setFrameView("final");
            for (let i = 0; i < frames; i++) await window.__lab.frame();
            return window.__lab.stats().gpu.meanMs;
          },
          { off, frames: COST_FRAMES },
        );
      const rows = { on: [], off: [] };
      for (let pair = 0; pair < COST_PAIRS; pair++) {
        rows.off.push(await batch(true));
        rows.on.push(await batch(false));
      }
      result.stations[`${map} ${station}`] = {
        plainMs: +median(rows.off).toFixed(3),
        wearMs: +median(rows.on.map((v, i) => v - rows.off[i])).toFixed(3),
        differences: rows.on.map((v, i) => +(v - rows.off[i]).toFixed(3)),
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
