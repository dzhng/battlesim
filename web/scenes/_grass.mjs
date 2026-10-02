// What the grass field grows, checked at the ground rig's stations on the
// village: each kind of ground grows the mix its biome row names, a field
// varies in hue and not in brightness, a clump stays the grass it was as the
// camera closes, and nothing grows on a road or under a wood. The field's
// seating, residency and cost are the village scene's.
import { readFile } from "node:fs/promises";
import { aim, lab } from "./_lab.mjs";
import { decode } from "./_png.mjs";
import { classAt, openStations, shoot, stationPose } from "./_groundStations.mjs";

const MAP = "village";
/** A plot's own grass is judged this near its middle, clear of its verge. */
const PLOT_HEART_M = 8;
/** The road's verge, as metres outside the road's edge: inside the verge's
 *  width and past the road's bare margin and the thinned strip beyond it. */
const VERGE_BAND_M = [0.5, 1.3];
/** A clump's colour is stored as bytes of its square root: how far its
 *  luminance may read from the ground's under a hue shift that keeps it. */
const LUMINANCE_SLACK = 0.04;
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

  // Each wild plot kind, at its own station: the clumps round the plot's
  // middle are the kinds its growth row mixes, its first among them.
  const grown = {};
  for (const kind of ["meadow", "rough", "prairie"]) {
    const station = `${kind}-65`;
    const [x, y] = stationPose(page, MAP, station).target;
    const heart = (await clumpsAt(page, station)).filter(
      (c) => Math.hypot(c.root[0] - x, c.root[1] - y) < PLOT_HEART_M,
    );
    grown[kind] = { clumps: heart.length, kinds: kindsOf(heart) };
  }
  // The road's verge at the bend, found by the ground's own class mask.
  const bend = await clumpsAt(page, "bend-65");
  const mask = decode(await shoot(page, MAP, "bend-65", { view: "ground-classes" }));
  const classed = bend
    .filter((c) => c.pixel)
    .map((c) => ({ ...c, ground: classAt(mask, c.pixel[0], c.pixel[1]) }))
    .filter((c) => c.ground);
  const verge = classed.filter(
    (c) => c.ground.roadSd > VERGE_BAND_M[0] && c.ground.roadSd < VERGE_BAND_M[1],
  );
  grown.verge = { clumps: verge.length, kinds: kindsOf(verge) };
  const mixOf = (row) => biome.grass.growth[row].mix;
  ctx.check(
    "each kind of wild ground grows the grasses its biome row mixes: meadow, rough, prairie and the road's verge",
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

  await fieldVariation(ctx, page);
  await page.close();
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
 *  - a dried field against the same field level: clump for clump it keeps
 *    the level one's luminance and is only ever nearer straw;
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

  const dryness = (dry) =>
    `for (const row of Object.values(rules.growth)) {
       row.patches.dry = ${dry};
       row.mix = row.mix.map((s) => ({ ...s, dry: 0 }));
     }`;
  await retuned(page, dryness(0.5));
  const dried = byRoot(await clumpsNow(page));
  await retuned(page, dryness(0));
  const level = byRoot(await clumpsNow(page));
  await lab(page, () => window.__lab.grass().retune(null));

  const pairs = [...dried].filter(([key]) => level.has(key)).map(([key, c]) => [c, level.get(key)]);
  const worstLuminance = pairs.reduce(
    (m, [a, b]) => Math.max(m, Math.abs(luminance(a.colour) / luminance(b.colour) - 1)),
    0,
  );
  const warmth = ([r, , b]) => r / Math.max(b, 1e-4);
  const shifts = pairs.map(([a, b]) => warmth(a.colour) / warmth(b.colour) - 1);
  const cooled = shifts.filter((s) => s < 0).length;
  const driedShare = shifts.filter((s) => s > DRIED).length / Math.max(1, pairs.length);
  ctx.check(
    "a field varies in hue at the ground's own luminance, and only toward straw: dry patches, never a darker or cooler one",
    pairs.length > 5000 &&
      pairs.length === dried.size &&
      worstLuminance < LUMINANCE_SLACK &&
      cooled === 0 &&
      driedShare > 0.05 &&
      driedShare < 0.7,
    JSON.stringify({ clumps: dried.size, pairs: pairs.length, worstLuminance, cooled, driedShare }),
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
