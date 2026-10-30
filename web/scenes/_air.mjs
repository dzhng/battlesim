// What the air-war checkpoints (`/labs/air-*`) share: opening a variant,
// playing the paused battle tick by tick while a scene watches its side's
// observation, and reading the published state the slices' claims are about.
import { readFileSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import FIXTURES from "../../apps/battle-lab/src/fixtures.json" with { type: "json" };
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { PNG } from "pngjs";
import { lab, obs, advance, presented, openBattle } from "./_lab.mjs";
import { loadEncounter } from "../src/maps/node.ts";
import { rules } from "./_units.mjs";
const ffmpeg = promisify(execFile);

/** A saved encounter `name` on the fixture's map (`fixtures.json`), as the
 *  route plays it. */
export const encounter = (name, fixture) => loadEncounter(mapOf(fixture), name);

/** The fixture's saved map. */
export const mapOf = (fixture) => FIXTURES.find((f) => f.id === fixture).map;

/** The fixture's field size, metres, from its map's metadata. */
export const field = (fixture) =>
  JSON.parse(
    readFileSync(new URL(`../../fixtures/maps/${mapOf(fixture)}/meta.json`, import.meta.url)),
  ).size_m;

/** The checkpoint's battle opened on `variant` (its default without one),
 *  paused past its first ticks. */
export function openAir(ctx, variant) {
  const url = new URL(ctx.url);
  if (variant) url.searchParams.set("variant", variant);
  return openBattle(ctx, { url: url.href });
}

/** Step the paused battle one tick at a time for up to `limit` ticks,
 *  handing `each` every observation; stops early when `each` returns true.
 *  Returns the last observation. */
export async function play(page, limit, each = () => false) {
  let o = await obs(page);
  for (let t = 0; t < limit; t++) {
    await advance(page, 1);
    o = await obs(page);
    if (each(o)) break;
  }
  return o;
}

/** The own units of type `kind`. */
export const own = (o, kind) => o.own.filter((u) => u.kind === kind);

/** Rounds `u`'s mount `mount` has launched (its pose's counter). */
export const shots = (u, mount = 0) => u.weaponPoses.find((p) => p.mount === mount)?.shots ?? 0;

/** The ground under `p`, as the lab's walkable surface reads it. */
export const surface = (page, p) => lab(page, (q) => window.__lab.route.surfaceZ(q[0], q[1]), p);

/** Whether `p` lies on the map. */
export const onMap = (p, size) => p[0] >= 0 && p[1] >= 0 && p[0] <= size[0] && p[1] <= size[1];

/** Record selected observed unit tracks and heights as a compact review PNG. */
export async function trace(page, ticks, ids, file) {
  const observations = [];
  observations.push(await obs(page));
  for (let i = 0; i < ticks; i++) {
    await advance(page, 1);
    observations.push(await obs(page));
  }
  const wanted = new Set(ids);
  const points = observations.flatMap((o) =>
    o.own
      .filter((u) => wanted.has(u.id))
      .map((u) => ({ id: u.id, x: u.position[0], y: u.position[1], z: u.position[2] })),
  );
  const png = new PNG({ width: 900, height: 500 });
  png.data.fill(18);
  const maxX = Math.max(1, ...points.map((p) => p.x)),
    maxY = Math.max(1, ...points.map((p) => p.y)),
    maxZ = Math.max(1, ...points.map((p) => p.z));
  const paint = (x, y, r, g, b) => {
    if (x < 0 || y < 0 || x >= png.width || y >= png.height) return;
    const i = (y * png.width + x) * 4;
    png.data[i] = r;
    png.data[i + 1] = g;
    png.data[i + 2] = b;
    png.data[i + 3] = 255;
  };
  observations.forEach((o, t) =>
    o.own
      .filter((u) => wanted.has(u.id))
      .forEach((u) => {
        const x = Math.round((u.position[0] / maxX) * 880 + 10);
        const y = Math.round((1 - u.position[1] / maxY) * 300 + 10);
        paint(x, y, 80, 210, 255);
        const hx = Math.round((t / Math.max(1, ticks)) * 880 + 10);
        const hy = Math.round(490 - (u.position[2] / maxZ) * 160);
        paint(hx, hy, 255, 190, 70);
      }),
  );
  const bytes = PNG.sync.write(png);
  await writeFile(file, bytes);
  return bytes;
}

/** Capture a short sequence and assemble it into a GIF with ffmpeg. */
export async function strip(page, ticks, file) {
  const frames = [];
  for (let i = 0; i < ticks; i++) {
    await advance(page, 1);
    frames.push(await page.screenshot());
  }
  const prefix = file.replace(/\.gif$/i, "");
  for (let i = 0; i < frames.length; i++)
    await writeFile(`${prefix}-${String(i).padStart(4, "0")}.png`, frames[i]);
  await ffmpeg("ffmpeg", [
    "-loglevel",
    "error",
    "-y",
    "-framerate",
    "30",
    "-i",
    `${prefix}-%04d.png`,
    file,
  ]);
  return file;
}

/** The high-air layer's cruise height over the ground. */
export const HIGH_AIR_CRUISE = rules.air.high_air.cruise_agl_m;

/** Visible ground cells in the side's fog field. */
export function visibleCells(o) {
  let n = 0;
  for (const word of o.fog.bits) {
    let w = word >>> 0;
    while (w) {
      w &= w - 1;
      n++;
    }
  }
  return n;
}

/** Draw the presented tick and save it as evidence `name`; returns the PNG. */
export async function shoot(ctx, page, name) {
  await presented(page);
  await page.evaluate(() => window.__lab.frame());
  const png = await page.screenshot();
  await writeFile(ctx.evidencePath(`${name}.png`), png);
  return png;
}

/** The vehicles the last frame drew: each one's appearance and where. */
export const drawnVehicles = (page) => lab(page, () => window.__lab.route.vehicles());

/** Issue `order` as blue's (or the drawn side's) player does, through the
 *  battle's input; resolves to the authority's acknowledgement. */
export const command = (page, order) =>
  lab(page, (order) => window.__lab.route.command(order), order);

/** The text of the floating card `selector` picks (`[data-unit="3"]`, …;
 *  empty, the whole card layer), after a fresh frame placed the cards; null
 *  when none is shown. */
export async function cardText(page, selector) {
  await page.evaluate(() => window.__lab.frame());
  const card = page.locator(`[data-testid=readouts] ${selector}`.trim()).first();
  return (await card.count()) ? card.textContent() : null;
}
