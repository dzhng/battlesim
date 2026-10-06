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
// A state other than intact adds its name to the files, and holds only the
// templates that end in it: a template is destroyed into one state, a ruin if
// it is low and gutted if it is tall, and is left out of the other's sheets.
//
// Each source set gets its own visit and evidence directory, so complete
// catalogue coverage keeps the shared kit download budget of a real map.
// Narrow a run with `CITY_SET=homes` or `CITY_CATEGORY=highrise` (what
// stands), `CITY_TIERS=2,3` (which tiers), `CITY_STATE=ruin` (one state; by
// default every state the library has rows for), and `CITY_PAIRS=` (a comma
// list of `station:tier:tier`, or `none`).
import { mkdir, readFile, writeFile } from "node:fs/promises";
import {
  buildingsSettled,
  buildingStats as stats,
  gpuWarnings,
  groundClasses,
  lab,
  route,
} from "./_lab.mjs";
import { cropBox, decode } from "./_png.mjs";
import { writeSheet } from "./_sheet.mjs";
import { bounds, judge, projected, setFits } from "./_templateFit.mjs";

const HIDE_PANEL =
  "[data-testid=city-lineup-panel] { display: none !important; }";
/** Clear picture round a template in a pair's crop, pixels. */
const CROP_PX = 6;
const PAIR_WIDTH = 520;
const DEFAULT_PAIRS = "transition-1:0:1,transition-2:1:2,transition-3:2:3";
/** Source sets visited side by side. */
const SETS_AT_ONCE = Number(process.env.CITY_SETS_AT_ONCE ?? 3);

const settle = async (page) => {
  await lab(page, () => window.__lab.frame());
  await buildingsSettled(page);
};
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

/** The frame's pixels in the box `at` alone. A pair's half needs only its
 *  template's crop, and encoding the whole 1920 by 1080 frame for each one
 *  was most of the scene's time. */
async function frameCrop(page, at) {
  await lab(page, () => window.__lab.frame());
  const { width, height } = page.viewportSize();
  return page.screenshot({
    clip: cropBox(width, height, at.x, at.y, at.w, at.h),
  });
}

async function runSet(ctx, page, set, fit) {
  const warnings = gpuWarnings(page);
  const query = new URLSearchParams();
  query.set("set", set);
  if (process.env.CITY_CATEGORY)
    query.set("category", process.env.CITY_CATEGORY);
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
  const costs = Object.fromEntries(
    (await route(page, "costs")).map((c) => [c.id, c]),
  );
  const boundaries = await route(page, "boundaries");
  const built = await stats(page);
  const categories = lineup.rows.map((row) => row.category);
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
      lineup.entries.every((e) =>
        costs[e.id].states.intact.rows.every((n) => n > 0),
      ),
    JSON.stringify({
      missing: lineup.missing,
      stood: lineup.entries.length,
      drawn: built,
    }),
  );

  // Every template has its label where its front meets the ground.
  await route(page, "stand", "all");
  await settle(page);
  const labels = await lab(page, () =>
    window.__lab.route.lineup().entries.map((e) => {
      const node = [
        ...document.querySelectorAll("[data-testid=city-lineup-labels] span"),
      ].find((n) => n.textContent === e.id);
      const box = node?.getBoundingClientRect();
      const anchor = window.__lab.projectToCss(
        (e.min[0] + e.max[0]) / 2,
        e.min[1] - 3,
        0,
      );
      // How far its middle is from the anchor's column, and how far below
      // the anchor its top is (a row's labels alternate between two lines).
      return box
        ? [box.left + box.width / 2 - anchor[0], box.top - anchor[1]]
        : null;
    }),
  );
  ctx.check(
    "every template is labelled with its id, at its front",
    labels.every(
      (off) =>
        off !== null && Math.abs(off[0]) < 2 && off[1] > -2 && off[1] < 24,
    ),
    JSON.stringify(
      labels.map((off) => off && off.map((px) => Number(px.toFixed(1)))),
    ),
  );
  await writeFile(ctx.evidencePath("all-1920x1080.png"), await frame(page));

  const tiers = (process.env.CITY_TIERS ?? "0,1,2,3").split(",").map(Number);
  const states = (
    process.env.CITY_STATE
      ? [process.env.CITY_STATE]
      : ["intact", "ruin", "gutted"]
  ).filter((state) => lineup.entries.some((e) => costs[e.id].states[state]));
  const pairs = (process.env.CITY_PAIRS ?? DEFAULT_PAIRS)
    .split(",")
    .filter((spec) => spec && spec !== "none")
    .map((spec) => {
      const [station, a, b] = spec.split(":");
      return { station, tiers: [Number(a), Number(b)] };
    });

  const verdicts = [];
  const honoured = [];
  const stood = [];
  for (const state of states) {
    const tag = state === "intact" ? "" : `-${state}`;
    await show(page, "setState", "state", state);
    // What stands in this state: the templates with rows for it.
    const standing = lineup.entries.filter((e) => costs[e.id].states[state]);
    const inCategory = (category) =>
      standing.filter((e) => e.category === category);
    const rows = categories.filter(
      (category) => inCategory(category).length > 0,
    );
    stood.push({
      state,
      templates: standing.length,
      drawn: (await stats(page)).buildings,
    });
    /** Per pair and template: its crop at each of the pair's tiers. */
    const crops = pairs.map(() => new Map());
    for (const tier of tiers) {
      await route(page, "drawAt", tier);
      await settle(page);

      // The rows, labelled.
      await show(page, "setLabels", "labels", true);
      for (const category of rows) {
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
      for (const category of rows) {
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
          const verdict = judge(decode(await groundClasses(page)), seen);
          verdicts.push({ id: entry.id, state, tier, ...verdict });
          const cost = costs[entry.id].states[state].triangles;
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
        for (const entry of standing) {
          await route(page, "stand", pair.station, entry.id);
          await settle(page);
          const { box } = await projected(page, entry, fit[entry.set]);
          const halves = crops[p].get(entry.id) ?? {};
          halves[tier] = await frameCrop(page, bounds(box, CROP_PX));
          crops[p].set(entry.id, halves);
          // The crop alone too, a folder a tier: what a comparison measures.
          const folder = `crops/${pair.station}${tag}-tier${tier}`;
          await mkdir(ctx.evidencePath(folder), { recursive: true });
          await writeFile(
            ctx.evidencePath(`${folder}/${entry.id}.png`),
            halves[tier],
          );
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
      for (const entry of standing)
        await writeSheet(ctx, `pair-${name}-${entry.id}.png`, {
          title: `${entry.id}, ${state}, at ${where}`,
          columns: 2,
          cellWidth: PAIR_WIDTH,
          cells: cellsOf(entry),
          pixelated: true,
        });
      for (const category of rows)
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
    "a line-up in a state draws the templates that have rows for it, and leaves the others out",
    stood.every((s) => s.templates > 0 && s.drawn === s.templates),
    JSON.stringify(stood),
  );
  ctx.check(
    "a forced tier has rows at the requested tier, no refused chunks, and no other fine tier",
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
  // A ruin fills no part and a gutted shell may be blown open: only what
  // stands intact is held to filling its parts.
  const adrift = verdicts.filter((v) => v.state === "intact" && !v.stands);
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
  return { ids: lineup.entries.map((entry) => entry.id), states };
}

export async function run(ctx) {
  const repo = new URL("../../", import.meta.url);
  const catalog = JSON.parse(
    await readFile(new URL("assets/catalog.json", repo), "utf8"),
  );
  const templates = JSON.parse(
    await readFile(
      new URL("fixtures/prototype-building-templates.json", repo),
      "utf8",
    ),
  );
  const { fit, setOf } = await setFits();
  const expected = templates.filter(
    (template) =>
      !process.env.CITY_CATEGORY ||
      template.category === process.env.CITY_CATEGORY,
  );
  const sets = process.env.CITY_SET
    ? [process.env.CITY_SET]
    : Object.entries(catalog.city_sets)
        .filter(
          ([set, source]) =>
            source.catalogue === "generated" &&
            expected.some((template) => setOf[template.id] === set),
        )
        .map(([set]) => set);
  const visited = [];
  const drawnStates = new Set();
  // The sets are independent pages: a few at once keep the GPU busy while
  // another page encodes its screenshots.
  const queue = [...sets];
  const visit = async () => {
    for (let set = queue.shift(); set; set = queue.shift()) {
      await mkdir(ctx.evidencePath(set), { recursive: true });
      const scoped = {
        ...ctx,
        evidencePath: (path) => ctx.evidencePath(`${set}/${path}`),
        writeEvidence: (path, value) =>
          ctx.writeEvidence(`${set}/${path}`, value),
        check: (name, ok, detail) => ctx.check(`${set}: ${name}`, ok, detail),
      };
      const page = await ctx.newPage({
        viewport: { width: 1920, height: 1080 },
      });
      try {
        const result = await runSet(scoped, page, set, fit);
        visited.push(...result.ids);
        for (const state of result.states) drawnStates.add(state);
      } finally {
        await page.context().close();
      }
    }
  };
  await Promise.all(Array.from({ length: SETS_AT_ONCE }, visit));
  if (!process.env.CITY_SET) {
    const ids = expected.map((template) => template.id).sort();
    ctx.check(
      "separate source-set visits cover the exact generated catalogue once",
      ids.length > 0 && JSON.stringify(visited.sort()) === JSON.stringify(ids),
      JSON.stringify({ sets, visited, expected: ids }),
    );
  }
  if (process.env.CITY_STATE)
    ctx.check(
      "the requested state has templates in the selected catalogue",
      drawnStates.has(process.env.CITY_STATE),
      JSON.stringify({
        requested: process.env.CITY_STATE,
        drawn: [...drawnStates],
      }),
    );
  await ctx.writeEvidence("coverage.json", {
    sets,
    templates: visited,
    states: [...drawnStates],
  });
}
