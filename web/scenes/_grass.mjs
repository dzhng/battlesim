// What the grass field grows, checked at the ground rig's stations on the
// village: each kind of ground grows the mix its biome row names (and
// ploughed earth nothing), a drilled crop keeps to its rows, a field varies
// in hue and not in brightness, a clump stays the grass it was as the camera
// closes, and nothing grows on a road or under a wood. The field's seating,
// residency and cost are the village scene's.
import { readFile } from "node:fs/promises";
import { aim, lab } from "./_lab.mjs";
import { decode } from "./_png.mjs";
import { classAt, openStations, shoot, stationPose, stationReport } from "./_groundStations.mjs";

const MAP = "village";
/** A plot's own grass is judged this near its middle, clear of its verge. */
const PLOT_HEART_M = 8;
/** The road's verge, as metres outside the road's edge: past the road's
 *  bare margin, and short of the verge's half width by more than the class
 *  mask's step and a pixel. */
const vergeBand = (biome) => [0.4, biome.verge.width_m / 2 - 0.2];
/** A clump's colour is stored as bytes of its square root: how far its
 *  luminance may read from the ground's under a hue shift that keeps it. */
const LUMINANCE_SLACK = 0.04;
/** How far the variation check's dried field may lighten: its patches dry
 *  by half, a wholly dry clump lighter by this share. */
const DRY_LIFT = 0.2;
const DRY = 0.5;
/** The grain the variation check plants: each clump's brightness within
 *  this share of the ground's, either way. */
const CLUMP_VALUE = 0.1;
/** A clump counts as dried when its red over blue rises by this share. */
const DRIED = 0.06;

const biome = JSON.parse(
  await readFile(new URL("../../fixtures/biomes/summer.json", import.meta.url), "utf8"),
);

/** The frame drawn at the page's pose, then its clumps, each with the page
 *  pixel its root stands on (null when off screen). */
const clumpsNow = (page) =>
  lab(page, async () => {
    await window.__lab.frame();
    const canvas = document.querySelector("canvas").getBoundingClientRect();
    const clumps = await window.__lab.grass().clumps();
    return clumps.map((c) => {
      const css = window.__lab.projectToCss(c.root[0], c.root[1], c.root[2]);
      const pixel = css && [Math.floor(css[0] - canvas.left), Math.floor(css[1] - canvas.top)];
      const shown =
        pixel &&
        pixel[0] >= 0 &&
        pixel[1] >= 0 &&
        pixel[0] < canvas.width &&
        pixel[1] < canvas.height;
      return { ...c, pixel: shown ? pixel : null };
    });
  });

async function clumpsAt(page, station) {
  await shoot(page, MAP, station);
  return clumpsNow(page);
}

const kindsOf = (clumps) => [...new Set(clumps.map((c) => c.kind))].sort();
const rootKey = (c) => c.root.map((v) => v.toFixed(3)).join(",");
const byRoot = (clumps) => new Map(clumps.map((c) => [rootKey(c), c]));
const luminance = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;

export async function grassGrowth(ctx) {
  const page = await openStations(ctx, MAP);

  // Each plot kind, at its own station: the clumps round the plot's middle
  // are the kinds its growth row mixes, its first among them; a kind with no
  // row (ploughed earth) grows nothing.
  const grown = {};
  const bare = {};
  for (const { name } of biome.plots) {
    const station = `${name}-25`;
    const [x, y] = stationPose(page, MAP, station).target;
    const all = await clumpsAt(page, station);
    const heart = all.filter((c) => Math.hypot(c.root[0] - x, c.root[1] - y) < PLOT_HEART_M);
    if (biome.grass.growth[name]) grown[name] = { clumps: heart.length, kinds: kindsOf(heart) };
    else bare[name] = { clumps: heart.length, round: all.length };
  }
  ctx.check(
    "a kind of ground with no growth row grows nothing: ploughed earth is bare",
    Object.keys(bare).length > 0 &&
      Object.values(bare).every((b) => b.clumps === 0 && b.round > 100),
    JSON.stringify(bare),
  );
  // The road's verge at the bend, found by the ground's own class mask.
  const bend = await clumpsAt(page, "bend-65");
  const mask = decode(await shoot(page, MAP, "bend-65", { view: "ground-classes" }));
  const classed = bend
    .filter((c) => c.pixel)
    .map((c) => ({ ...c, ground: classAt(mask, c.pixel[0], c.pixel[1]) }))
    .filter((c) => c.ground);
  const [inner, outer] = vergeBand(biome);
  const verge = classed.filter((c) => c.ground.roadSd > inner && c.ground.roadSd < outer);
  grown.verge = { clumps: verge.length, kinds: kindsOf(verge) };
  const mixOf = (row) => biome.grass.growth[row].mix;
  ctx.check(
    "each kind of ground grows the grasses its biome row mixes: every plot kind and the road's verge",
    Object.entries(grown).every(
      ([row, g]) =>
        g.clumps > 100 &&
        g.kinds.includes(mixOf(row)[0].appearance) &&
        g.kinds.every((k) => mixOf(row).some((s) => s.appearance === k)),
    ),
    JSON.stringify(grown),
  );

  // Exclusions, by the same mask: a byte's step and a pixel of slack.
  const edge = await clumpsAt(page, "forest-edge-65");
  const edgeMask = decode(
    await shoot(page, MAP, "forest-edge-65", { view: "ground-classes", trees: false }),
  );
  const wooded = edge.filter(
    (c) => c.pixel && classAt(edgeMask, c.pixel[0], c.pixel[1])?.forest === "inside",
  );
  const onRoad = classed.filter((c) => c.ground.roadSd < -0.3);
  ctx.check(
    "no grass stands on the road or under the wood",
    classed.length > 1000 && edge.length > 1000 && onRoad.length === 0 && wooded.length === 0,
    JSON.stringify({
      bend: classed.length,
      onRoad: onRoad.length,
      edge: edge.length,
      wooded: wooded.length,
    }),
  );

  await cropRows(ctx, page);
  await fieldVariation(ctx, page);
  await page.close();
}

/** A drilled crop against the same crop scattered, each planted through the
 *  lab's rules: how far its clumps stand from the middle of their rows, as a
 *  share of the row's period (a quarter when scattered evenly). */
async function cropRows(ctx, page) {
  const pose = stationPose(page, MAP, "wheat-65");
  const { across } = stationReport(page).plots.wheat;
  const period = biome.plots.find((p) => p.name === "wheat").furrow_m;
  const offRow = (clumps) => {
    const heart = clumps.filter(
      (c) => Math.hypot(c.root[0] - pose.target[0], c.root[1] - pose.target[1]) < PLOT_HEART_M,
    );
    const off = heart.map((c) => {
      const phase = (c.root[0] * across[0] + c.root[1] * across[1]) / period;
      return Math.abs(phase - Math.floor(phase) - 0.5);
    });
    return { clumps: heart.length, mean: off.reduce((a, b) => a + b, 0) / Math.max(1, off.length) };
  };
  await shoot(page, MAP, "wheat-65");
  await retuned(page, "rules.growth.wheat.rows = 0;");
  const scattered = offRow(await clumpsNow(page));
  await retuned(page, "rules.growth.wheat.rows = 1;");
  const drilled = offRow(await clumpsNow(page));
  await lab(page, () => window.__lab.grass().retune(null));
  ctx.check(
    "a drilled crop's clumps stand on the plot's rows; scattered, they stand anywhere",
    scattered.clumps > 1000 &&
      drilled.clumps > 1000 &&
      Math.abs(scattered.mean - 0.25) < 0.03 &&
      drilled.mean < 0.03,
    JSON.stringify({ scattered, drilled, period, across }),
  );
}

/** Grow by the biome's rules as `change` (a function body over `rules`, run
 *  in the page) alters them. */
const retuned = (page, change) =>
  lab(
    page,
    (change) => {
      const grass = window.__lab.grass();
      window.__biomeGrass ??= structuredClone(grass.rules());
      const rules = structuredClone(window.__biomeGrass);
      new Function("rules", change)(rules);
      grass.retune(rules);
    },
    change,
  );

/** The field's own variation, each arm planted through the lab's rules so
 *  the biome's numbers can move freely:
 *  - a meadow of two evenly spread grasses stands in their shares' ratio;
 *  - a field with a grain against the same field without: no clump is
 *    lighter or darker by more than the grain;
 *  - a dried field against the same field level: clump for clump it is no
 *    darker, only so much paler, and only ever nearer straw;
 *  - the clumps a closer camera keeps are the grasses they were. */
async function fieldVariation(ctx, page) {
  const pose = stationPose(page, MAP, "meadow-65");
  const [first, second] = biome.grass.growth.meadow.mix.map((s) => s.appearance);
  await shoot(page, MAP, "meadow-65");
  await retuned(
    page,
    `rules.growth.meadow.mix = [
       { appearance: "${first}", share: 3, drift: 0, dry: 0 },
       { appearance: "${second}", share: 1, drift: 0, dry: 0 },
     ];`,
  );
  const mixed = (await clumpsNow(page)).filter(
    (c) => Math.hypot(c.root[0] - pose.target[0], c.root[1] - pose.target[1]) < PLOT_HEART_M,
  );
  const share = mixed.filter((c) => c.kind === first).length / Math.max(1, mixed.length);
  ctx.check(
    "a row's clumps are its grasses by share: three to one planted, three to one grown",
    mixed.length > 1000 && Math.abs(share - 0.75) < 0.05,
    JSON.stringify({ clumps: mixed.length, share }),
  );

  const field = (dry, value) =>
    `rules.clump_value = ${value};
     rules.dry_lift = ${DRY_LIFT};
     for (const row of Object.values(rules.growth)) {
       row.patches.dry = ${dry};
       row.mix = row.mix.map((s) => ({ ...s, dry: 0 }));
     }`;
  await retuned(page, field(DRY, CLUMP_VALUE));
  const dried = byRoot(await clumpsNow(page));
  await retuned(page, field(0, CLUMP_VALUE));
  const level = byRoot(await clumpsNow(page));
  await retuned(page, field(0, 0));
  const even = byRoot(await clumpsNow(page));
  await lab(page, () => window.__lab.grass().retune(null));

  // Clump by clump against the same field with no grain: each is lighter
  // or darker by no more than the grain asked for, and they do differ.
  const grain = [...level]
    .filter(([key]) => even.has(key))
    .map(([key, c]) => luminance(c.colour) / luminance(even.get(key).colour) - 1);
  const widest = grain.reduce((m, g) => Math.max(m, Math.abs(g)), 0);
  const spread = Math.sqrt(grain.reduce((s, g) => s + g * g, 0) / Math.max(1, grain.length));
  ctx.check(
    "a clump's brightness varies from the ground's by no more than the grain the field asks for",
    grain.length > 5000 &&
      widest < CLUMP_VALUE + LUMINANCE_SLACK &&
      spread > CLUMP_VALUE / 4 &&
      spread < CLUMP_VALUE,
    JSON.stringify({ clumps: grain.length, widest, spread }),
  );

  const pairs = [...dried].filter(([key]) => level.has(key)).map(([key, c]) => [c, level.get(key)]);
  const lifts = pairs.map(([a, b]) => luminance(a.colour) / luminance(b.colour) - 1);
  const [darkest, lightest] = [Math.min(...lifts), Math.max(...lifts)];
  const warmth = ([r, , b]) => r / Math.max(b, 1e-4);
  const shifts = pairs.map(([a, b]) => warmth(a.colour) / warmth(b.colour) - 1);
  const cooled = shifts.filter((s) => s < 0).length;
  const driedShare = shifts.filter((s) => s > DRIED).length / Math.max(1, pairs.length);
  ctx.check(
    "a field's dry patches stand toward straw, paler by no more than the lift asked for: never a darker or cooler patch",
    pairs.length > 5000 &&
      pairs.length === dried.size &&
      darkest > -LUMINANCE_SLACK &&
      lightest < DRY * DRY_LIFT + LUMINANCE_SLACK &&
      cooled === 0 &&
      driedShare > 0.05 &&
      driedShare < 0.7,
    JSON.stringify({
      clumps: dried.size,
      pairs: pairs.length,
      darkest,
      lightest,
      cooled,
      driedShare,
    }),
  );

  // Closer on the same ground: every clump both cameras draw is one grass.
  const far = byRoot(await clumpsAt(page, "meadow-65"));
  await aim(page, pose.target, { ...pose, distance: 40 });
  const near = byRoot(await clumpsNow(page));
  const kept = [...near].filter(([key]) => far.has(key));
  const changed = kept.filter(([key, c]) => far.get(key).kind !== c.kind).length;
  ctx.check(
    "the clumps a closer camera keeps are the grasses they were",
    kept.length > 2000 && changed === 0,
    JSON.stringify({ far: far.size, near: near.size, kept: kept.length, changed }),
  );
}
