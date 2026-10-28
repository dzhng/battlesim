// C69: the river lab. The drawn river is measured for SG2's question: does
// a river carved into the 4 m grid read round, or stepped? C70's water's edge is measured here too (BANKS_ONLY=1
// alone): its bands against the grass beside them, the grass thinning across
// the bare earth, and a waterline that holds still.
import { writeFile } from "node:fs/promises";
import { PNG } from "pngjs";
import { chroma, linear, luminance, median } from "./_colour.mjs";
import { aim, lab, openBattle, snapshot } from "./_lab.mjs";
import { decode, pixel, writeCrop } from "./_png.mjs";
import { BIOME, classAt, openStations, shoot } from "./_groundStations.mjs";
import { installInside } from "./_water.mjs";

const VIEWPORT = { width: 1920, height: 1080 };
/** Looking north, as the lab opens. */
const YAW = -Math.PI / 2;
/** The play camera's framing (Defilade's): 65 m at 0.85 rad. */
const PLAY = { distance: 65, pitch: 0.85, yaw: YAW };
const TOP = { distance: 250, pitch: 1.5, yaw: YAW };
/** SG2's bar: the largest step in the bank's shading, as a share of the flat
 *  ground's luminance. */
const FACET_STEP_MAX = 0.08;
/** The banks are walked this far outside the water's edge, in metres: on
 *  the bank (it runs 4.8 m in the lab) and just past its top. */
const BANK_LINES_M = [1, 2.5, 4, 6];
/** A band is apart from the grass in hue when their mean colours differ by
 *  this much in CIELAB's a*b* plane, whatever their lightness. */
const HUE_APART_MIN = 6;
/** The shallows on one bank may be this many times as pale against the
 *  channel as on the other: the banks are lit from different sides. */
const SHALLOWS_SIDES_MAX = 1.25;
/** A waterline pixel flickers when its luminance jumps by this much between
 *  two frames a centimetre apart (the water and the bank differ by more);
 *  the share of them that may, for the edges a nudge moves across a pixel. */
const FLICKER_STEP = 0.04;
const FLICKER_SHARE_MAX = 0.005;

/** The mean luminance of the 3×3 pixels round `p` (or the square `reach`
 *  pixels each way). */
function luminanceAt(png, p, reach = 1) {
  let sum = 0;
  for (let dy = -reach; dy <= reach; dy++)
    for (let dx = -reach; dx <= reach; dx++) sum += luminance(pixel(png, p[0] + dx, p[1] + dy));
  return sum / (2 * reach + 1) ** 2;
}

/** CIELAB a* and b* of the mean colour of the square `reach` pixels each way
 *  round `p`: its hue and chroma, apart from how light it is. */
function chromaAt(png, p, reach) {
  const mean = [0, 0, 0];
  for (let dy = -reach; dy <= reach; dy++)
    for (let dx = -reach; dx <= reach; dx++)
      pixel(png, p[0] + dx, p[1] + dy).forEach((v, c) => (mean[c] += linear(v)));
  return chroma(mean.map((v) => v / (2 * reach + 1) ** 2));
}

/** Sections across the banks, every two metres along the stream on each side
 *  (`side` -1 or 1): the page point `offsets[k]` metres outside the water's
 *  edge, for every offset, where all of them are in the frame. */
const bankSections = (page, offsets) =>
  lab(
    page,
    (offsets) => {
      const { rivers } = window.__lab.route.exports();
      const stride = window.__lab.route.layout().riverStride;
      const [width, height] = [window.innerWidth, window.innerHeight];
      const project = ([x, y]) => {
        const p = window.__lab.projectToCss(x, y, window.__lab.route.groundHeight(x, y) ?? 0);
        return p && p[0] > 8 && p[1] > 8 && p[0] < width - 8 && p[1] < height - 8 ? p : null;
      };
      const sections = [];
      for (const side of [-1, 1])
        for (let o = 0; o < rivers.length; o += stride) {
          const [ax, ay, bx, by, halfA, halfB] = rivers.subarray(o, o + 6);
          const length = Math.hypot(bx - ax, by - ay);
          for (let along = 0; along < length; along += 2) {
            const t = along / length;
            const points = offsets.map((offset) => {
              const reach = (halfA + (halfB - halfA) * t + offset) * side;
              const x = ax + (bx - ax) * t - ((by - ay) / length) * reach,
                y = ay + (by - ay) * t + ((bx - ax) / length) * reach;
              // Clear of the bridge and the wood, and truly `offset` out (the
              // inside of a bend folds a section back over itself).
              return x > 100 && x < 420 && Math.abs(window.__inside(x, y) + offset) < 0.05
                ? project([x, y])
                : null;
            });
            if (points.every(Boolean)) sections.push({ side, points });
          }
        }
      return sections;
    },
    offsets,
  );

/** C70: the water's edge, on the ground rig's frozen page. */
async function waterEdge(ctx) {
  const { shore } = BIOME;
  const page = await openStations(ctx, "river");
  await installInside(page);
  // Where each band is whole: the middle of the wet bank, the bare earth
  // short of where its outer line can wander in to, and the grass past it.
  const earthEnds = shore.mud_m * (1 - shore.wander);
  const offsets = {
    wet: shore.wet_m / 2,
    earth: shore.wet_m + 0.25 * (earthEnds - shore.wet_m),
    grass: shore.mud_m + 3,
    // A metre inside the water, and well out in it.
    shallows: -1,
    channel: -5,
  };

  // --- the bands against the grass beside them --------------------------------
  const bands = {};
  for (const station of ["bend-65", "wide-65"]) {
    const frame = decode(await shoot(page, "river", station));
    await writeFile(ctx.evidencePath(`shore-${station}.png`), PNG.sync.write(frame));
    const sections = await bankSections(page, Object.values(offsets));
    for (const side of [-1, 1]) {
      const here = sections.filter((s) => s.side === side);
      const light = (k) => here.map((s) => luminanceAt(frame, s.points[k], 3));
      const [wet, earth, grass, shallows, channel] = [0, 1, 2, 3, 4].map(light);
      const apart = (k) =>
        median(
          here.map((s) => {
            const [a, b] = [chromaAt(frame, s.points[k], 3), chromaAt(frame, s.points[2], 3)];
            return Math.hypot(a[0] - b[0], a[1] - b[1]);
          }),
        );
      bands[`${station}/${side}`] = {
        sections: here.length,
        wetOverGrass: +median(wet.map((v, i) => v / grass[i])).toFixed(3),
        earthOverGrass: +median(earth.map((v, i) => v / grass[i])).toFixed(3),
        wetHueApart: +apart(0).toFixed(1),
        earthHueApart: +apart(1).toFixed(1),
        shallowsOverChannel: +median(shallows.map((v, i) => v / channel[i])).toFixed(3),
      };
    }
  }
  await ctx.writeEvidence("shore.json", { offsets, bands });
  const sides = (station) =>
    bands[`${station}/1`].shallowsOverChannel / bands[`${station}/-1`].shallowsOverChannel;
  ctx.check(
    "the shallows are as pale against the channel on one bank as on the other",
    ["bend-65", "wide-65"].every(
      (station) => sides(station) < SHALLOWS_SIDES_MAX && sides(station) > 1 / SHALLOWS_SIDES_MAX,
    ),
    JSON.stringify({
      "bend-65": +sides("bend-65").toFixed(3),
      "wide-65": +sides("wide-65").toFixed(3),
    }),
  );
  ctx.check(
    "on either bank, the wet bank and the bare earth are no darker than the grass beside them, and apart from it in hue",
    Object.values(bands).every(
      (b) =>
        b.sections > 20 &&
        b.wetOverGrass >= 1 &&
        b.earthOverGrass >= 1 &&
        b.wetHueApart > HUE_APART_MIN &&
        b.earthHueApart > HUE_APART_MIN,
    ),
    JSON.stringify(bands),
  );

  // --- the bands are where the simulation's distance puts them ----------------
  // The class mask's water distance, which the bands are painted by, against
  // the exported stretches' at seeded pixels within the bands' reach.
  const mask = decode(await shoot(page, "river", "bend-65", { view: "ground-classes" }));
  // --- grass thins across the bare earth ---------------------------------------
  await shoot(page, "river", "bend-65");
  const outside = await lab(page, async () =>
    (await window.__lab.grass().clumps()).map((c) => -window.__inside(c.root[0], c.root[1])),
  );
  // Clumps a metre of band width: the wet bank, each half of the bare earth
  // that is always there, and the field past the earth's farthest line.
  const within = (start, stop) =>
    outside.filter((d) => d >= start && d < stop).length / (stop - start);
  const middle = (shore.wet_m + earthEnds) / 2;
  const density = {
    wet: within(0, shore.wet_m),
    earthNear: within(shore.wet_m, middle),
    earthFar: within(middle, earthEnds),
    field: within(shore.mud_m + 1, shore.mud_m + 4),
  };
  ctx.check(
    "no grass stands on the wet bank, and it thickens across the bare earth to the field's own",
    outside.length > 2000 &&
      density.wet === 0 &&
      density.earthNear < 0.35 * density.field &&
      density.earthFar > density.earthNear &&
      density.earthFar < 0.9 * density.field,
    JSON.stringify({ clumps: outside.length, aMetre: density }),
  );

  // --- the waterline holds still ----------------------------------------------
  // The water's surface meets the ground exactly on the waterline, where two
  // surfaces at one depth could flicker. Nudging the camera a centimetre a
  // frame moves a fighting pixel by the whole difference between water and
  // bank; a still one barely at all.
  const still = [];
  for (let k = 0; k < 6; k++) {
    await aim(page, [150 + k * 0.01, 262], { distance: 65, pitch: 0.85, yaw: -Math.PI / 2 });
    await lab(page, async () => {
      await window.__lab.suppressGrass(true);
      await window.__lab.frame();
    });
    still.push(decode(await page.screenshot()));
  }
  let waterline = 0,
    jumped = 0;
  for (let y = 2; y < mask.height - 2; y++)
    for (let x = 2; x < mask.width - 2; x++) {
      const c = classAt(mask, x, y);
      if (!c || Math.abs(c.riverSd) > 0.5) continue;
      waterline++;
      let most = 0;
      for (let k = 1; k < still.length; k++)
        most = Math.max(
          most,
          Math.abs(luminance(pixel(still[k], x, y)) - luminance(pixel(still[k - 1], x, y))),
        );
      if (most > FLICKER_STEP) jumped++;
    }
  ctx.check(
    "nudging the camera a centimetre a frame, no pixel of the waterline flickers",
    waterline > 2000 && jumped / waterline < FLICKER_SHARE_MAX,
    JSON.stringify({ waterline, jumped }),
  );
  await page.close();
}

/** Lines along the river's banks, each a fixed distance outside the water's
 *  edge (`offsets`, metres) on one side, sampled every metre from the first
 *  bend to the wood: page points under the current camera, in order along the
 *  stream (`lines`, a break where a point leaves the frame); and points of
 *  flat ground well clear of the bank (`flat`). Walking a line, the ground's
 *  shading changes only as the bank turns: a jump from one metre to the next
 *  is a step. */
const bankLines = (page, offsets) =>
  lab(
    page,
    (offsets) => {
      const { rivers } = window.__lab.route.exports();
      const stride = window.__lab.route.layout().riverStride;
      const [width, height] = [window.innerWidth, window.innerHeight];
      const project = ([x, y]) => {
        const p = window.__lab.projectToCss(x, y, window.__lab.route.groundHeight(x, y) ?? 0);
        return p && p[0] > 4 && p[1] > 4 && p[0] < width - 4 && p[1] < height - 4 ? p : null;
      };
      const inside = (x, y) => {
        let best = -Infinity;
        for (let o = 0; o < rivers.length; o += stride) {
          const ax = rivers[o],
            ay = rivers[o + 1],
            dx = rivers[o + 2] - ax,
            dy = rivers[o + 3] - ay;
          const t = Math.min(1, Math.max(0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)));
          const half = rivers[o + 4] + (rivers[o + 5] - rivers[o + 4]) * t;
          best = Math.max(best, half - Math.hypot(x - ax - dx * t, y - ay - dy * t));
        }
        return best;
      };
      const lines = [];
      for (const offset of offsets)
        for (const side of [-1, 1]) {
          let line = [];
          for (let o = 0; o < rivers.length; o += stride) {
            const [ax, ay, bx, by, halfA, halfB] = rivers.subarray(o, o + 6);
            const length = Math.hypot(bx - ax, by - ay);
            for (let along = 0; along < length; along += 1) {
              const t = along / length;
              const reach = (halfA + (halfB - halfA) * t + offset) * side;
              const x = ax + (bx - ax) * t - ((by - ay) / length) * reach,
                y = ay + (by - ay) * t + ((bx - ax) / length) * reach;
              // West of the wood, clear of the bridge, and truly `offset`
              // out (the inside of a bend folds a line back over itself).
              const p =
                x > 100 && x < 400 && Math.abs(inside(x, y) + offset) < 0.05
                  ? project([x, y])
                  : null;
              if (p) line.push(p);
              else if (line.length) {
                lines.push(line);
                line = [];
              }
            }
          }
          if (line.length) lines.push(line);
        }
      const flat = [];
      for (let y = 100; y < 400; y += 12)
        for (let x = 100; x < 400; x += 12) {
          const at = inside(x, y);
          if (at < -20 && at > -60) flat.push(project([x, y]));
        }
      return { lines, flat: flat.filter(Boolean) };
    },
    offsets,
  );

export async function run(ctx) {
  await waterEdge(ctx);
  if (process.env.BANKS_ONLY) return;
  // A pinned tick: the orders below then land on the same tick every run.
  const page = await openBattle(ctx, { viewport: VIEWPORT, grass: true, timeout: 60000, tick: 30 });
  await page.addStyleTag({ content: ".lab-panel { display: none }" });

  // --- the look: frames for review and pixel measures ------------------------
  await lab(page, () => window.__lab.suppressFog(true));
  await aim(page, [260, 255], TOP);
  await snapshot(ctx, page, "top-250.png");
  for (const [name, at] of [
    ["meander", [240, 300]],
    ["bridge", [60, 232]],
    ["wide", [500, 222]],
    ["narrow", [100, 236]],
  ]) {
    await aim(page, at, PLAY);
    await snapshot(ctx, page, `play-65-${name}.png`);
  }
  await aim(page, [150, 258], { ...PLAY, distance: 25 });
  await snapshot(ctx, page, "close-25-bank.png");
  await aim(page, [300, 150], { distance: 200, pitch: 0.3, yaw: YAW });
  await snapshot(ctx, page, "low-200.png");

  // The ground's shading alone: every dry triangle one tint, lit as the
  // ground is, so a step from one metre of a bank to the next is shading and
  // nothing else. SG2's bar is 8% of the flat ground's luminance.
  await lab(page, () => window.__lab.route.setGroundView("shading"));
  await page.waitForTimeout(300);
  const facets = {};
  for (const [name, at, view] of [
    ["top-250", [260, 255], TOP],
    ["play-65", [240, 300], PLAY],
  ]) {
    await aim(page, at, view);
    const frame = decode(await snapshot(ctx, page, `facets-${name}.png`));
    const bank = await bankLines(page, BANK_LINES_M);
    const flats = bank.flat.map((p) => luminanceAt(frame, p)).sort((x, y) => x - y);
    const flat = flats[flats.length >> 1];
    const steps = bank.lines
      .flatMap((line) =>
        line.slice(1).map((p, k) => ({
          at: p,
          step: Math.abs(luminanceAt(frame, p) - luminanceAt(frame, line[k])) / flat,
        })),
      )
      .sort((x, y) => x.step - y.step);
    const share = (q) => +(steps[Math.floor((steps.length - 1) * q)]?.step ?? 0).toFixed(4);
    facets[name] = {
      steps: steps.length,
      flat: +flat.toFixed(4),
      median: share(0.5),
      p95: share(0.95),
      p99: share(0.99),
      max: share(1),
      overBar: steps.filter((s) => s.step > FACET_STEP_MAX).length,
    };
    const worst = steps[steps.length - 1];
    if (worst)
      await writeCrop(
        frame,
        ctx.evidencePath(`facets-${name}-worst-4x.png`),
        worst.at[0],
        worst.at[1],
        60,
        40,
        4,
      );
  }
  await lab(page, () => window.__lab.route.setGroundView("surface"));
  await page.waitForTimeout(300);
  await ctx.writeEvidence("sg2.json", { facets });
  ctx.check(
    "walking a bank, the ground's shading never steps by more than 8% of the flat ground's luminance",
    facets["top-250"].steps > 1000 &&
      facets["play-65"].steps > 100 &&
      Object.values(facets).every((f) => f.max <= FACET_STEP_MAX),
    JSON.stringify(facets),
  );

  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [VIEWPORT.width, VIEWPORT.height],
    dpr: 1,
  });

  // The same frames under a low sun, where a facet shows most.
  await page.close();
  const dusk = await openBattle(ctx, {
    viewport: VIEWPORT,
    url: ctx.urlFor({ sun: 0.3 }),
    grass: true,
    timeout: 60000,
  });
  await dusk.addStyleTag({ content: ".lab-panel { display: none }" });
  await lab(dusk, () => window.__lab.suppressFog(true));
  await aim(dusk, [260, 255], TOP);
  await snapshot(ctx, dusk, "low-sun-top-250.png");
  await aim(dusk, [240, 300], PLAY);
  await snapshot(ctx, dusk, "low-sun-play-65-meander.png");
  await aim(dusk, [150, 258], { ...PLAY, distance: 25 });
  await snapshot(ctx, dusk, "low-sun-close-25-bank.png");
}
