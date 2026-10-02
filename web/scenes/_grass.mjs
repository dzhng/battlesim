// What the grass field grows, checked at the ground rig's stations on the
// village: each kind of ground grows what its biome row names, and nothing
// grows on a road or under a wood. The field's seating, residency and cost
// are the village scene's.
import { readFile } from "node:fs/promises";
import { lab } from "./_lab.mjs";
import { decode } from "./_png.mjs";
import { classAt, openStations, shoot, stationPose } from "./_groundStations.mjs";

const MAP = "village";
/** A plot's own grass is judged this near its middle, clear of its verge. */
const PLOT_HEART_M = 8;
/** The road's verge, as metres outside the road's edge: inside the verge's
 *  width and past the road's bare margin and the thinned strip beyond it. */
const VERGE_BAND_M = [0.5, 1.3];

const biome = JSON.parse(
  await readFile(new URL("../../fixtures/biomes/summer.json", import.meta.url), "utf8"),
);

/** The station's frame drawn, then its clumps, each with the page pixel its
 *  root stands on (null when off screen). */
async function clumpsAt(page, station) {
  await shoot(page, MAP, station);
  return lab(page, async () => {
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
}

const kindsOf = (clumps) => [...new Set(clumps.map((c) => c.kind))].sort();

export async function grassGrowth(ctx) {
  const page = await openStations(ctx, MAP);

  // Each wild plot kind, at its own station: the clumps round the plot's
  // middle are the kinds its growth row names.
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
  const named = (row) => [biome.grass.growth[row].appearance];
  ctx.check(
    "each kind of wild ground grows the grass its biome row names: meadow, rough, prairie and the road's verge",
    Object.entries(grown).every(
      ([row, g]) => g.clumps > 100 && g.kinds.every((k) => named(row).includes(k)),
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
  await page.close();
}
