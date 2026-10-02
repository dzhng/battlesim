// The template line-up: every building template of the installed art library
// on flat ground, drawn through the frame's own buildings path. The scene
// holds each template to its contract at each detail tier (it is drawn, it
// stands on the ground, and nothing of it leaves its physical parts grown by
// its set's fit), and writes the pictures the art is judged by:
//
// - `sheet-<category>-tier<k>.png`: each of the category's templates alone,
//   framed to fit, at tier k;
// - `row-<category>-tier<k>.png`: the category's row with its labels;
// - `pairs-<station>-<category>.png` and `pair-<station>-<template>.png`:
//   each template at a station at two tiers, side by side, and each half
//   alone under `crops/`. By default the stations are the three tier
//   boundaries, each with the tier before and after it.
//
// A state other than intact adds its name to the files.
//
// Narrow a run with `CITY_SET=homes` or `CITY_CATEGORY=highrise` (what
// stands), `CITY_TIERS=2,3` (which tiers), `CITY_STATE=ruin` (one state; by
// default every state the library has rows for), and `CITY_PAIRS=` (a comma
// list of `station:tier:tier`, or `none`).
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { buildingsSettled, lab } from "./_lab.mjs";
import { crop, decode, pixel } from "./_png.mjs";
import { writeSheet } from "./_sheet.mjs";

const REPO = new URL("../../", import.meta.url);
const HIDE_PANEL = "[data-testid=city-lineup-panel] { display: none !important; }";
/** The ground-classes view is black where a pixel is not wholly bare ground. */
const isBody = (rgb) => rgb.every((v) => v === 0);
/** How far past a projected outline a covered pixel may lie: the outline is
 *  projected in floats and the frame is rasterised. */
const OUTLINE_PX = 1.5;
/** The ground searched for stray art round a template's projected parts. */
const SEARCH_PX = 16;
/** How far in from a face's edges its samples lie, as a share of its half
 *  size: eaves and corner posts are not the wall. */
const INSET = 0.6;
/** How far above its base a part's wall is sampled, metres: low enough that
 *  a building off the ground shows ground there, high enough that the ray
 *  through the sample meets a wall set a little inside the face. */
const FOOT_M = 1;
/** The share of a part's inside samples that must be covered: a part may be
 *  a roof on posts. Its foot must show somewhere along the wall. */
const COVERED = 2 / 3;
/** Samples along the foot of a wall, from end to end. */
const FOOT_SAMPLES = 41;
/** Clear picture round a template in a pair's crop, metres and pixels. */
const CROP_M = 3;
const CROP_PX = 6;
const PAIR_WIDTH = 520;
const DEFAULT_PAIRS = "transition-1:0:1,transition-2:1:2,transition-3:2:3";

/** Each source set's fit (how far its art may reach past a part's faces),
 *  from the set's own templates file. */
async function fits() {
  const catalog = JSON.parse(await readFile(new URL("assets/catalog.json", REPO), "utf8"));
  const out = {};
  for (const [set, { templates }] of Object.entries(catalog.city_sets)) {
    const text = await readFile(new URL(templates, REPO), "utf8");
    if (text.startsWith("version https://git-lfs"))
      throw new Error(
        `${templates} is a Git LFS pointer; run: git lfs pull --include="assets/source/city/*/templates.json"`,
      );
    out[set] = JSON.parse(text).fit;
  }
  return out;
}

/** The convex outline of `points`, counter-clockwise (monotone chain). */
function outline(points) {
  const sorted = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const cross = (o, a, b) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const half = (list) => {
    const hull = [];
    for (const p of list) {
      while (hull.length >= 2 && cross(hull.at(-2), hull.at(-1), p) <= 0) hull.pop();
      hull.push(p);
    }
    hull.pop();
    return hull;
  };
  return [...half(sorted), ...half(sorted.reverse())];
}

/** Whether (x, y) is inside the convex `hull`, or within `slack` of it. */
function within(hull, x, y, slack) {
  for (let i = 0; i < hull.length; i++) {
    const [a, b] = [hull[i], hull[(i + 1) % hull.length]];
    const [ex, ey] = [b[0] - a[0], b[1] - a[1]];
    // Signed distance to the edge's line, positive inside.
    if (((x - a[0]) * ey - (y - a[1]) * ex) / Math.hypot(ex, ey) > slack) return false;
  }
  return true;
}

const settle = async (page) => {
  await lab(page, () => window.__lab.frame());
  await buildingsSettled(page);
};
const stats = (page) => lab(page, () => window.__lab.stats().buildings);
const route = (page, method, ...args) =>
  lab(page, ([method, args]) => window.__lab.route[method](...args), [method, args]);
/** Tell the route what to show, and wait until it shows it. */
async function show(page, method, key, value) {
  await route(page, method, value);
  await page.waitForFunction(
    ([key, value]) => window.__lab.route.shown()[key] === value,
    [key, value],
  );
  await settle(page);
}

async function frame(page) {
  await lab(page, () => window.__lab.frame());
  return page.screenshot();
}
async function groundClasses(page) {
  await lab(page, () => window.__lab.setFrameView("ground-classes"));
  const png = await page.screenshot();
  await lab(page, () => window.__lab.setFrameView("final"));
  return decode(png);
}

/** A template's parts as the camera sees them: per part, the corners of the
 *  part grown by `fit`, sample points inside it at half its height (a ray to
 *  one has come through its roof or a wall) and at the foot of the wall
 *  nearest the camera; then the corners of the box round the template grown
 *  by `CROP_M`. All in page pixels. */
function projected(page, entry, fit) {
  return lab(
    page,
    ({ entry, fit, INSET, FOOT_M, FOOT_SAMPLES, CROP_M }) => {
      const px = (p) => window.__lab.projectToCss(p[0], p[1], p[2]);
      const eye = window.__lab.camera();
      const toEye = [Math.cos(eye.yaw), Math.sin(eye.yaw)];
      const corners = (min, max) =>
        [0, 1, 2, 3, 4, 5, 6, 7].map((k) =>
          px([k & 1 ? max[0] : min[0], k & 2 ? max[1] : min[1], k & 4 ? max[2] : min[2]]),
        );
      return {
        parts: entry.parts.map((part) => {
          const [cos, sin] = [Math.cos(part.yaw), Math.sin(part.yaw)];
          // A point of the part's own frame, in the world.
          const at = (u, v, z) => [
            part.center[0] + cos * u - sin * v,
            part.center[1] + sin * u + cos * v,
            z,
          ];
          const [hx, hy, hz] = part.half;
          const [gx, gy] = [hx + fit.side_m, hy + fit.side_m];
          const top = part.baseZ + 2 * hz;
          const grown = [];
          for (const u of [-gx, gx])
            for (const v of [-gy, gy])
              for (const z of [part.baseZ, top + fit.top_m]) grown.push(px(at(u, v, z)));
          const core = [];
          for (const u of [-INSET, 0, INSET])
            for (const v of [-INSET, 0, INSET]) core.push(px(at(u * hx, v * hy, part.baseZ + hz)));
          // The wall that faces the camera most squarely.
          const faces = [
            { normal: [cos, sin], out: [hx, 0], along: [0, hy] },
            { normal: [-cos, -sin], out: [-hx, 0], along: [0, hy] },
            { normal: [-sin, cos], out: [0, hy], along: [hx, 0] },
            { normal: [sin, -cos], out: [0, -hy], along: [hx, 0] },
          ];
          const facing = faces.reduce((a, b) =>
            a.normal[0] * toEye[0] + a.normal[1] * toEye[1] >=
            b.normal[0] * toEye[0] + b.normal[1] * toEye[1]
              ? a
              : b,
          );
          const foot = Array.from({ length: FOOT_SAMPLES }, (_, i) => {
            const t = (2 * i) / (FOOT_SAMPLES - 1) - 1;
            return px(
              at(
                facing.out[0] + t * facing.along[0],
                facing.out[1] + t * facing.along[1],
                part.baseZ + FOOT_M,
              ),
            );
          });
          return { grown, core, foot };
        }),
        box: corners(
          [entry.min[0] - CROP_M, entry.min[1] - CROP_M, entry.min[2]],
          [entry.max[0] + CROP_M, entry.max[1] + CROP_M, entry.max[2] + CROP_M],
        ),
      };
    },
    { entry, fit, INSET, FOOT_M, FOOT_SAMPLES, CROP_M },
  );
}

const bounds = (points, pad) => {
  const [xs, ys] = [points.map((p) => p[0]), points.map((p) => p[1])];
  return {
    x: Math.min(...xs) - pad,
    y: Math.min(...ys) - pad,
    w: Math.max(...xs) - Math.min(...xs) + 2 * pad,
    h: Math.max(...ys) - Math.min(...ys) + 2 * pad,
  };
};

/** Hold one template, standing alone and framed to fit, to its contract in
 *  the ground-classes view `mask`. */
function judge(mask, seen, intact) {
  const hulls = seen.parts.map((part) => outline(part.grown));
  const search = bounds(
    seen.parts.flatMap((part) => part.grown),
    SEARCH_PX,
  );
  let covered = 0;
  let stray = 0;
  let worst = null;
  for (
    let y = Math.max(0, Math.floor(search.y));
    y < Math.min(mask.height, search.y + search.h);
    y++
  )
    for (
      let x = Math.max(0, Math.floor(search.x));
      x < Math.min(mask.width, search.x + search.w);
      x++
    ) {
      if (!isBody(pixel(mask, x, y))) continue;
      covered++;
      if (hulls.some((hull) => within(hull, x + 0.5, y + 0.5, OUTLINE_PX))) continue;
      stray++;
      worst ??= [x, y];
    }
  const share = (points) => points.filter((p) => isBody(pixel(mask, ...p))).length / points.length;
  const cores = seen.parts.map((part) => share(part.core));
  const feet = seen.parts.map((part) => share(part.foot));
  return {
    covered,
    stray,
    worst,
    cores,
    feet,
    inside: covered > 0 && stray === 0,
    stands: !intact || (cores.every((c) => c >= COVERED) && feet.every((f) => f > 0)),
  };
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  const warnings = [];
  page.on("console", (m) => {
    if (m.type() === "warning" && /webgpu|validation|gpu\w*error/i.test(m.text()))
      warnings.push(m.text().slice(0, 200));
  });
  const query = new URLSearchParams();
  if (process.env.CITY_SET) query.set("set", process.env.CITY_SET);
  if (process.env.CITY_CATEGORY) query.set("category", process.env.CITY_CATEGORY);
  await ctx.openLab(page, `${ctx.url}?${query}`, 120000);
  await page.waitForFunction(
    async () => {
      await window.__lab.frame();
      return window.__lab.stats().buildings.buildings > 0;
    },
    undefined,
    { timeout: 120000, polling: 100 },
  );
  await page.addStyleTag({ content: HIDE_PANEL });

  const lineup = await route(page, "lineup");
  const costs = Object.fromEntries((await route(page, "costs")).map((c) => [c.id, c]));
  const fit = await fits();
  const boundaries = await route(page, "boundaries");
  const built = await stats(page);
  const categories = lineup.rows.map((row) => row.category);
  const inCategory = (category) => lineup.entries.filter((e) => e.category === category);
  console.log(
    `METRIC city-lineup: ${lineup.entries.length} templates in ${categories.length} rows on a ${lineup.size.join(" x ")} m map; tiers change at ${boundaries.map((m) => m.toFixed(0)).join(", ")} m`,
  );
  for (const entry of lineup.entries) {
    const { rows, triangles } = costs[entry.id].states.intact;
    console.log(
      `METRIC city-lineup ${entry.id} (${entry.set}): rows ${rows.join("/")}, triangles ${triangles.join("/")} at tiers 0/1/2/3`,
    );
  }
  await ctx.writeEvidence("costs.json", { boundaries, costs });
  ctx.check(
    "every catalogue template has art, stands in the line-up and has rows at every tier",
    lineup.missing.length === 0 &&
      built.buildings === lineup.entries.length &&
      built.coarse >= built.buildings &&
      lineup.entries.every((e) => costs[e.id].states.intact.rows.every((n) => n > 0)),
    JSON.stringify({ missing: lineup.missing, stood: lineup.entries.length, drawn: built }),
  );

  // Every template has its label where its front meets the ground.
  await route(page, "stand", "all");
  await settle(page);
  const labels = await lab(page, () =>
    window.__lab.route.lineup().entries.map((e) => {
      const node = [...document.querySelectorAll("[data-testid=city-lineup-labels] span")].find(
        (n) => n.textContent === e.id,
      );
      const box = node?.getBoundingClientRect();
      const anchor = window.__lab.projectToCss((e.min[0] + e.max[0]) / 2, e.min[1] - 3, 0);
      // How far its middle is from the anchor's column, and how far below
      // the anchor its top is (a row's labels alternate between two lines).
      return box ? [box.left + box.width / 2 - anchor[0], box.top - anchor[1]] : null;
    }),
  );
  ctx.check(
    "every template is labelled with its id, at its front",
    labels.every((off) => off !== null && Math.abs(off[0]) < 2 && off[1] > -2 && off[1] < 24),
    JSON.stringify(labels.map((off) => off && off.map((px) => Number(px.toFixed(1))))),
  );
  await writeFile(ctx.evidencePath("all-1920x1080.png"), await frame(page));

  const tiers = (process.env.CITY_TIERS ?? "0,1,2,3").split(",").map(Number);
  const states = process.env.CITY_STATE
    ? [process.env.CITY_STATE]
    : ["intact", "ruin", "gutted"].filter((state) =>
        lineup.entries.some((e) => costs[e.id].states[state]),
      );
  const pairs = (process.env.CITY_PAIRS ?? DEFAULT_PAIRS)
    .split(",")
    .filter((spec) => spec && spec !== "none")
    .map((spec) => {
      const [station, a, b] = spec.split(":");
      return { station, tiers: [Number(a), Number(b)] };
    });

  const verdicts = [];
  const honoured = [];
  for (const state of states) {
    const tag = state === "intact" ? "" : `-${state}`;
    await show(page, "setState", "state", state);
    /** Per pair and template: its crop at each of the pair's tiers. */
    const crops = pairs.map(() => new Map());
    for (const tier of tiers) {
      await route(page, "drawAt", tier);
      await settle(page);

      // The rows, labelled.
      await show(page, "setLabels", "labels", true);
      for (const category of categories) {
        await route(page, "stand", "row", inCategory(category)[0].id);
        await settle(page);
        await writeFile(
          ctx.evidencePath(`row-${category}-tier${tier}${tag}.png`),
          await frame(page),
        );
      }
      await route(page, "stand", "all");
      await settle(page);
      const drawn = await stats(page);
      honoured.push({
        state,
        tier,
        tiers: drawn.tiers,
        refused: drawn.refused,
        // Only the forced tier draws: the coarse rows of a resident chunk
        // are counted at the last tier, hidden where they stand.
        ok:
          drawn.refused === 0 &&
          drawn.tiers[tier] > 0 &&
          drawn.tiers.every((n, t) => t === tier || t === 3 || n === 0),
      });

      // Each template alone, framed to fit: its contract, and its picture.
      await show(page, "setLabels", "labels", false);
      for (const category of categories) {
        const cells = [];
        for (const entry of inCategory(category)) {
          await show(page, "solo", "solo", entry.id);
          await route(page, "stand", "fit", entry.id);
          await page.waitForFunction(
            async () => {
              await window.__lab.frame();
              return window.__lab.stats().buildings.buildings === 1;
            },
            undefined,
            { polling: 50 },
          );
          await settle(page);
          const picture = await frame(page);
          const seen = await projected(page, entry, fit[entry.set]);
          const verdict = judge(await groundClasses(page), seen, state === "intact");
          verdicts.push({ id: entry.id, state, tier, ...verdict });
          const cost = (costs[entry.id].states[state] ?? costs[entry.id].states.intact).triangles;
          cells.push({
            caption: `${entry.id} · tier ${tier} · ${Math.round(cost[tier])} triangles`,
            png: picture,
          });
        }
        await writeSheet(ctx, `sheet-${category}-tier${tier}${tag}.png`, {
          title: `${category}, ${state}, tier ${tier}: each template alone, framed to fit`,
          columns: Math.min(3, cells.length),
          cellWidth: 640,
          cells,
        });
      }
      await show(page, "solo", "solo", null);

      // The pairs' halves at this tier, each cropped to its template.
      for (const [p, pair] of pairs.entries()) {
        if (!pair.tiers.includes(tier)) continue;
        for (const entry of lineup.entries) {
          await route(page, "stand", pair.station, entry.id);
          await settle(page);
          const picture = decode(await frame(page));
          const { box } = await projected(page, entry, fit[entry.set]);
          const at = bounds(box, CROP_PX);
          const halves = crops[p].get(entry.id) ?? {};
          halves[tier] = crop(picture, at.x, at.y, at.w, at.h);
          crops[p].set(entry.id, halves);
          // The crop alone too, a folder a tier: what a comparison measures.
          const folder = `crops/${pair.station}${tag}-tier${tier}`;
          await mkdir(ctx.evidencePath(folder), { recursive: true });
          await writeFile(ctx.evidencePath(`${folder}/${entry.id}.png`), halves[tier]);
        }
      }
    }

    for (const [p, pair] of pairs.entries()) {
      if (!pair.tiers.every((tier) => tiers.includes(tier))) continue;
      const range = /^transition-([123])$/.exec(pair.station);
      const where = range
        ? `${boundaries[Number(range[1]) - 1].toFixed(0)} m, where tier ${pair.tiers[0]} gives way to tier ${pair.tiers[1]}`
        : `the ${pair.station} station`;
      const name = `${pair.station}${range ? "" : `-${pair.tiers.join("v")}`}${tag}`;
      const cellsOf = (entry) =>
        pair.tiers.map((tier) => ({
          caption: `${entry.id} · tier ${tier}`,
          png: crops[p].get(entry.id)[tier],
        }));
      for (const entry of lineup.entries)
        await writeSheet(ctx, `pair-${name}-${entry.id}.png`, {
          title: `${entry.id}, ${state}, at ${where}`,
          columns: 2,
          cellWidth: PAIR_WIDTH,
          cells: cellsOf(entry),
          pixelated: true,
        });
      for (const category of categories)
        await writeSheet(ctx, `pairs-${name}-${category}.png`, {
          title: `${category}, ${state}, at ${where}`,
          columns: 2,
          cellWidth: PAIR_WIDTH,
          cells: inCategory(category).flatMap(cellsOf),
          pixelated: true,
        });
    }
  }
  await route(page, "drawAt", null);

  ctx.check(
    "a forced tier draws every building at that tier and no other",
    honoured.every((h) => h.ok),
    JSON.stringify(honoured),
  );
  const brief = (v) => ({
    id: v.id,
    state: v.state,
    tier: v.tier,
    covered: v.covered,
    stray: v.stray,
    worst: v.worst,
    cores: v.cores.map((c) => Number(c.toFixed(2))),
    feet: v.feet.map((f) => Number(f.toFixed(2))),
  });
  const outside = verdicts.filter((v) => !v.inside);
  ctx.check(
    "every template, at every tier, is drawn and nothing of it lies outside its parts grown by its set's fit",
    outside.length === 0,
    `${verdicts.length} views; ${JSON.stringify(outside.map(brief))}`,
  );
  const adrift = verdicts.filter((v) => !v.stands);
  ctx.check(
    "every intact template, at every tier, fills each of its parts and meets the ground along the foot of its wall",
    adrift.length === 0,
    `${verdicts.length} views; ${JSON.stringify(adrift.map(brief))}`,
  );
  ctx.check(
    "no WebGPU validation warning was logged",
    warnings.length === 0,
    warnings.slice(0, 3).join(" | "),
  );
  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [1920, 1080],
    dpr: 1,
    lineup,
    boundaries,
    fit,
    tiers,
    states,
    pairs,
    verdicts: verdicts.map(brief),
  });
}
