// How a town's streets read (C28 to C30), measured on the ground rig's
// generated town: a frame beside the terrain's own class mask, each pixel put
// on a street or on the country road by how wide the way under it is.
import { readFileSync } from "node:fs";
import { classAt, groundUnder, openStations } from "./_groundStations.mjs";
import { frame, mean } from "./_roads.mjs";
import { lab } from "./_lab.mjs";
import { pixel } from "./_png.mjs";

const fixture = (path) =>
  JSON.parse(readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8"));
const presets = fixture("map-presets.json");
const street = fixture("biomes/summer.json").roads.road;
/** A way narrower than this is a town street; wider, up to the second, the
 *  country road: the generator's own widths, and the middle between them. */
const STREET_M = presets.parcels.street_width_m;
const COUNTRY_M = presets.roads.country_road_width_m;
const isStreet = (width) => width > STREET_M - 1 && width < (STREET_M + COUNTRY_M) / 2;
const isCountry = (width) => width >= (STREET_M + COUNTRY_M) / 2 && width < COUNTRY_M + 1;

/** A road's core: this far inside its edge. */
const CORE_M = -1;
/** A street's walk, clear of its edges, and the ground past it. */
const WALK_M = [0.4, street.walk.width_m - 0.4];
const LAWN_M = [street.walk.width_m + 1, street.walk.width_m + 4];
/** A group is judged on at least this many pixels. */
const GROUP_PIXELS = 150;

/** For each point `{ xy, out }` (`out` metres outside the paving, or under
 *  0 on it), the width of the way it lies on or beside: the shortest chord
 *  of road through it, or through the nearest road `out` from it. */
const wayWidths = (page, points) =>
  lab(
    page,
    (points) => {
      const road = (x, y) => window.__lab.route.surfaceAt(x, y)?.kind === "road";
      const reach = (x, y, dx, dy) => {
        let m = 0;
        while (m < 12 && road(x + dx * (m + 0.25), y + dy * (m + 0.25))) m += 0.25;
        return m;
      };
      const width = (x, y) => {
        let least = Infinity;
        for (let k = 0; k < 24; k++) {
          const [dx, dy] = [Math.cos((k / 24) * Math.PI), Math.sin((k / 24) * Math.PI)];
          least = Math.min(least, reach(x, y, dx, dy) + reach(x, y, -dx, -dy));
        }
        return least;
      };
      return points.map(({ xy: [x, y], out }) => {
        if (out <= 0) return width(x, y);
        let least = Infinity;
        for (let k = 0; k < 32; k++) {
          const a = (k / 32) * 2 * Math.PI;
          const [qx, qy] = [x + (out + 1) * Math.cos(a), y + (out + 1) * Math.sin(a)];
          if (road(qx, qy)) least = Math.min(least, width(qx, qy));
        }
        return least;
      });
    },
    points,
  );

/** The pixels of `mask` whose class passes `keep`, one in `every`. */
function sample(mask, every, keep) {
  const out = [];
  let n = 0;
  for (let y = 0; y < mask.height; y++)
    for (let x = 0; x < mask.width; x++) {
      const c = classAt(mask, x, y);
      if (c && c.forest === "none" && c.riverSd > 3 && keep(c) && n++ % every === 0)
        out.push([x, y]);
    }
  return out;
}

/** The mean summed colour difference between two shots over `pixels`. */
const changed = (a, b, pixels) =>
  pixels.reduce(
    (sum, [x, y]) =>
      sum + pixel(a, x, y).reduce((s, v, k) => s + Math.abs(v - pixel(b, x, y)[k]), 0),
    0,
  ) / Math.max(1, pixels.length);

/** How far a colour leans from blue toward red, as a share of its red. */
const warmth = ([r, , b]) => (r - b) / r;

export async function streetLooks(ctx) {
  const page = await openStations(ctx, "generated");
  const shot = await frame(page, "generated", "junction-65");
  const within =
    ([lo, hi]) =>
    (c) =>
      c.roadSd >= lo && c.roadSd <= hi;
  const picked = {
    core: sample(shot.mask, 41, (c) => c.roadSd < CORE_M),
    walk: sample(shot.mask, 13, within(WALK_M)),
    lawn: sample(shot.mask, 29, within(LAWN_M)),
  };
  const groups = {};
  for (const [name, pixels] of Object.entries(picked)) {
    const world = await groundUnder(page, pixels);
    const out = name === "core" ? 0 : name === "walk" ? WALK_M[1] : LAWN_M[1];
    const widths = await wayWidths(
      page,
      world.map(({ xy }) => ({ xy, out })),
    );
    groups[name] = {
      street: pixels.filter((_, i) => isStreet(widths[i])),
      country: pixels.filter((_, i) => isCountry(widths[i])),
    };
  }
  await page.close();
  const of = (pixels) => {
    const { count, rgb, luminance } = mean(shot.shot, pixels);
    return { count, luminance: +luminance.toFixed(4), warmth: +warmth(rgb).toFixed(3) };
  };
  const roadbed = of(groups.core.street);
  const gravel = of(groups.core.country);
  const walk = of(groups.walk.street);
  const lawn = of(groups.lawn.street);
  const counted = [roadbed, gravel, walk, lawn].every((g) => g.count >= GROUP_PIXELS);
  ctx.check(
    "a town street is pavement: darker and greyer than the country road it meets, and never darker than the ground beside it",
    counted &&
      roadbed.luminance <= 0.85 * gravel.luminance &&
      roadbed.warmth < gravel.warmth &&
      roadbed.luminance >= lawn.luminance,
    JSON.stringify({ roadbed, gravel, lawn }),
  );
  // Grass is drawn over the ground it grows on: where none grows, the frame
  // is the bare ground's.
  const grown = {
    walk: +changed(shot.shot, shot.bare, groups.walk.street).toFixed(2),
    lawn: +changed(shot.shot, shot.bare, groups.lawn.street).toFixed(2),
  };
  ctx.check(
    "a street has a walk each side: paler than its roadbed, and bare of grass",
    counted && walk.luminance >= 1.2 * roadbed.luminance && grown.walk < 3 && grown.lawn > 10,
    JSON.stringify({ walk, roadbed, grown }),
  );
  await ctx.writeEvidence("street-looks.json", { roadbed, gravel, walk, lawn, grown });
}
