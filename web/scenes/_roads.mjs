// How the roads read (C66), measured on the ground rig's stations: each
// station's frame beside the terrain's own class mask, so a band is the
// pixels the material says it is, never a hand-drawn crop.
import { classAt, groundUnder, openStations, shoot } from "./_groundStations.mjs";
import { decode, pixel } from "./_png.mjs";

/** A road's core: this far inside its edge, clear of the feather. */
const CORE_M = -0.5;
/** The grass beside a road: this far outside its edge, past anything worn. */
const GRASS_M = [5, 9];
/** A band is judged on at least this many pixels. */
const BAND_PIXELS = 2000;

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

/** A station's frame and its class mask. */
async function frame(page, map, station) {
  return {
    mask: decode(await shoot(page, map, station, { view: "ground-classes" })),
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

export async function roadLooks(ctx) {
  const village = await openStations(ctx, "village");
  const bands = { "village bend-65": coreAgainstGrass(await frame(village, "village", "bend-65")) };
  await village.close();
  const river = await openStations(ctx, "river");
  bands["river track-65"] = coreAgainstGrass(await frame(river, "river", "track-65"));
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
  const road = mean(
    junction.shot,
    on(([x]) => Math.abs(x - 60) < 4),
  );
  const track = mean(
    junction.shot,
    on(([x]) => x > 67),
  );
  ctx.check(
    "each road kind is drawn as itself: the dirt track warm brown beside the country road's grey",
    road.count > 100 &&
      track.count > 100 &&
      track.rgb[0] > track.rgb[1] &&
      track.rgb[1] > track.rgb[2] &&
      warmth(track.rgb) > warmth(road.rgb) + 0.08,
    JSON.stringify({
      road: { ...road, warmth: warmth(road.rgb) },
      track: { ...track, warmth: warmth(track.rgb) },
    }),
  );
  await river.close();
}
