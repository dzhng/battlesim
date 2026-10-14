// What the air-war checkpoints (`/labs/air-*`) share: opening a variant,
// playing the paused battle tick by tick while a scene watches its side's
// observation, and reading the published state the slices' claims are about.
import { writeFile } from "node:fs/promises";
import { lab, obs, advance, presented, openBattle } from "./_lab.mjs";
import { loadEncounter } from "../src/maps/node.ts";
import { game } from "./_units.mjs";

/** The air map's saved encounter `name`, as the route plays it. */
export const encounter = (name) => loadEncounter("air", name);

/** The air map's size, metres. */
export const FIELD = [640, 520];

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
export const onMap = (p) => p[0] >= 0 && p[1] >= 0 && p[0] <= FIELD[0] && p[1] <= FIELD[1];

/** The cruise height every aircraft holds over the ground. */
export const CRUISE = game.air.rotor.cruise_agl_m;
export const JET_CRUISE = game.air.jet.cruise_agl_m;

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
