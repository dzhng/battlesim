// The plots' own ground texture (grain, broken rows, wheelings), checked at
// the ground rig's stations on the village with the texture on and off in
// one page: every kind of field in view carries texture inside it, its mean
// colour stays where its palette put it, nothing broad is darker than its
// plot, and from the strategic height it has faded to its mean.
// FIELD_COST=1 measures what it costs a frame instead (alone, under the GPU
// lock).
import { median, rec709 } from "./_colour.mjs";
import { lab } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";
import { BIOME, classAt, openStations, pairedCost, shoot } from "./_groundStations.mjs";

const MAP = "village";
/** A plot's interior is judged in blocks this many pixels a side: about a
 *  metre and a half from the play camera, nine metres from 250 m. */
const BLOCK = { fine: 32, broad: 48 };
/** Ground this near a road is not a plot's interior. */
const ROAD_CLEAR_M = 2;
/** A field carries texture when its blocks' luminance spreads this many
 *  times wider than its plain rows' alone, and by this many display levels. */
const TEXTURE_GAIN = 1.25;
const TEXTURE_LEVELS = 1;
/** A kind is judged where at least this many blocks of it are in view: a
 *  corner of a plot at the frame's edge says little. */
const KIND_BLOCKS = 100;
/** A kind's mean luminance moves no more than this share with its texture
 *  (the wheelings are bare, so a drilled crop is a little darker), and no
 *  broad block is darker than this share of its plain self. */
const MEAN_SLACK = 0.025;
const BROAD_DARKEST = 0.97;
/** From the strategic height a pixel moves less than this many levels. */
const FAR_LEVELS = 1;
const COST_PAIRS = 6;

/** A displayed pixel's luma, 0 to 255. */
const luma = (png, x, y) => rec709(pixel(png, x, y));

/** The bare ground of `station` with the plots' texture on and off, and its
 *  class mask. */
async function pair(page, station) {
  const bare = { grass: false, trees: false };
  const mask = decode(await shoot(page, MAP, station, { ...bare, view: "ground-classes" }));
  const on = decode(await shoot(page, MAP, station, bare));
  await lab(page, () => window.__lab.suppressFieldTexture(true));
  const off = decode(await shoot(page, MAP, station, bare));
  await lab(page, () => window.__lab.suppressFieldTexture(false));
  return { mask, on, off };
}

/** Every `size`-pixel block of the frame that lies inside one plot, clear of
 *  roads and woods: its plot kind, and its luminance's mean and spread with
 *  the texture on and off. */
function blocks({ mask, on, off }, size) {
  const out = [];
  for (let y0 = 0; y0 + size <= mask.height; y0 += size)
    for (let x0 = 0; x0 + size <= mask.width; x0 += size) {
      const first = classAt(mask, x0, y0);
      let inside = !!first;
      for (let y = y0; y < y0 + size && inside; y += 4)
        for (let x = x0; x < x0 + size; x += 4) {
          const c = classAt(mask, x, y);
          if (
            !c ||
            c.forest !== "none" ||
            c.roadSd < ROAD_CLEAR_M ||
            c.plotKind !== first.plotKind ||
            c.plotHash !== first.plotHash
          ) {
            inside = false;
            break;
          }
        }
      if (!inside) continue;
      const stats = (png) => {
        let [sum, squares] = [0, 0];
        for (let y = y0; y < y0 + size; y++)
          for (let x = x0; x < x0 + size; x++) {
            const l = luma(png, x, y);
            sum += l;
            squares += l * l;
          }
        const mean = sum / (size * size);
        return { mean, spread: Math.sqrt(Math.max(0, squares / (size * size) - mean * mean)) };
      };
      out.push({ kind: BIOME.plots[first.plotKind].name, on: stats(on), off: stats(off) });
    }
  return out;
}

const mean = (values) => values.reduce((s, v) => s + v, 0) / values.length;
const round = (v) => Math.round(v * 1000) / 1000;

export async function fieldTexture(ctx) {
  const page = await openStations(ctx, MAP);

  // From the play camera: texture in every field, at its palette's mean.
  const near = {};
  for (const station of ["field-65", "wheat-65", "meadow-65"])
    for (const b of blocks(await pair(page, station), BLOCK.fine)) (near[b.kind] ??= []).push(b);
  const kinds = Object.fromEntries(
    Object.entries(near)
      .filter(([, list]) => list.length >= KIND_BLOCKS)
      .map(([kind, list]) => [
        kind,
        {
          blocks: list.length,
          spread: round(median(list.map((b) => b.on.spread))),
          plain: round(median(list.map((b) => b.off.spread))),
          mean: round(mean(list.map((b) => b.on.mean)) / mean(list.map((b) => b.off.mean))),
        },
      ]),
  );
  ctx.check(
    "from the play camera every kind of field in view carries texture inside it",
    Object.keys(kinds).length >= 3 &&
      Object.values(kinds).every(
        (k) => k.spread >= TEXTURE_GAIN * k.plain && k.spread - k.plain >= TEXTURE_LEVELS,
      ),
    JSON.stringify(kinds),
  );
  ctx.check(
    "a field's texture leaves its mean luminance where its palette put it",
    Object.values(kinds).every((k) => Math.abs(k.mean - 1) <= MEAN_SLACK),
    JSON.stringify(Object.fromEntries(Object.entries(kinds).map(([k, v]) => [k, v.mean]))),
  );

  // From 250 m: nothing broad is darker than its plot.
  const broad = blocks(await pair(page, "field-250"), BLOCK.broad);
  const darkest = Math.min(...broad.map((b) => b.on.mean / b.off.mean));
  ctx.check(
    "no broad patch of a field is darker for its texture",
    broad.length >= 100 && darkest >= BROAD_DARKEST,
    JSON.stringify({ blocks: broad.length, darkest: round(darkest) }),
  );

  // From the strategic height the texture is its mean.
  const far = await pair(page, "patchwork-1100");
  let moved = 0;
  let ground = 0;
  for (let y = 0; y < far.mask.height; y += 2)
    for (let x = 0; x < far.mask.width; x += 2) {
      if (!classAt(far.mask, x, y)) continue;
      ground++;
      moved += Math.abs(luma(far.on, x, y) - luma(far.off, x, y));
    }
  ctx.check(
    "from the strategic height a field's texture has faded to its mean",
    ground > 100000 && moved / ground < FAR_LEVELS,
    JSON.stringify({ ground, levels: round(moved / ground) }),
  );
  await page.close();
}

/** FIELD_COST=1: what the plots' texture costs a frame at two stations where
 *  fields fill the view, grass and all (`pairedCost`). */
export async function fieldCost(ctx) {
  const page = await openStations(ctx, MAP);
  const result = { stations: {} };
  for (const station of ["field-65", "field-250"]) {
    await shoot(page, MAP, station);
    const cost = await pairedCost(page, "suppressFieldTexture", COST_PAIRS);
    result.stations[station] = {
      plainMs: round(cost.plainMs),
      textureMs: round(cost.costMs),
      differences: cost.differences.map(round),
    };
  }
  result.adapter = await page.evaluate(() => window.__lab.adapter);
  await ctx.writeEvidence("field-cost.json", result);
  ctx.check(
    "the field texture's frame cost was measured",
    Object.keys(result.stations).length > 0 &&
      Object.values(result.stations).every(
        (station) =>
          Number.isFinite(station.plainMs) &&
          Number.isFinite(station.textureMs) &&
          station.differences.length > 0 &&
          station.differences.every(Number.isFinite),
      ),
    JSON.stringify(result.stations),
  );
  await page.close();
}
