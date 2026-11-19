// An authored scenario's battle, opened by its address as the main menu's
// Tutorial tile will: the scenario map and document prepare under this
// build's rules, the battle opens in preparation over the player's road in,
// and the shop offers only what the document offers, a squad only in its
// offered ride.
import { readFileSync } from "node:fs";
import { groundCss, hideHud, lab, snapshot } from "./_lab.mjs";
import { decode, mostChanged } from "./_png.mjs";
import { playable } from "./generated.mjs";

const [MAP, NAME] = ["ferryman-valley", "operation-ferryman"];
const SCENARIO = `${MAP}/${NAME}`;

export async function run(ctx) {
  const page = await ctx.newPage();
  const url = new URL(ctx.url);
  url.search = `?scenario=${SCENARIO}`;
  await page.goto(url.href);
  await playable(page, ctx.readinessTimeout, 1);
  await lab(page, () => window.__lab.route.pause());
  const observed = await lab(page, () => window.__lab.route.observation());
  const me = observed.skirmish?.players?.[0];
  ctx.check(
    "the scenario's battle opens in preparation",
    observed.skirmish?.phase === "preparation",
    JSON.stringify(observed.skirmish?.phase),
  );
  ctx.check(
    "its player's shop is narrowed to what the document offers",
    Array.isArray(me?.offered) && me.offered.length > 0,
    JSON.stringify(me?.offered),
  );
  await snapshot(ctx, page, "preparation.png");
  await page.getByRole("button", { name: "Reinforcements" }).click();
  const shown = await page
    .getByRole("tabpanel")
    .getByRole("button")
    .evaluateAll((cards) => cards.map((card) => card.getAttribute("data-card")));
  const sold = new Set(me.offered.map((o) => o.kind));
  ctx.check(
    "the shop shows only offered cards",
    shown.every((id) => sold.has(id)),
    JSON.stringify(shown),
  );
  await snapshot(ctx, page, "shop.png");
  await page.keyboard.press("Escape");
  await coach(ctx, page);
}

/** The coach through the briefing, step by step from its own document:
 *  each step's rings show with it, a tour flies the camera and hands it
 *  back, and a marked zone is painted on the ground while its step shows.
 *  The player does what each step asks, until a step waits on a purchase
 *  (the walkthrough plays those). */
async function coach(ctx, page) {
  const card = page.getByRole("complementary", { name: "Coach" });
  const shown = () => card.getAttribute("data-step");
  const [chapter] = COACH.chapters;
  const served = await page.evaluate(async () => {
    const pack = await (await fetch("/audio/voices/narration/lantern/pack.json")).json();
    const take = await fetch(pack.lines[0].url);
    return { lines: pack.lines.length, take: take.status, type: take.headers.get("content-type") };
  });
  ctx.check(
    "the narrator's pack and its takes are served",
    served.lines > 0 && served.take === 200,
    JSON.stringify(served),
  );
  for (let at = await shown(); ; ) {
    const step = chapter.steps.find((s) => s.id === at);
    ctx.check(`the coach shows a briefing step (${at})`, !!step, at);
    if (!step || (typeof step.until === "object" && "beat" in step.until)) break;
    await snapshot(ctx, page, `coach-${step.id}.png`);
    for (const anchor of step.highlight ?? [])
      ctx.check(
        `${step.id}: ${anchor} is ringed`,
        (await page.locator(`[data-highlight="${anchor}"]`).count()) === 1,
      );
    if (step.camera) await tour(ctx, page, step);
    // A mark is judged against the same view once its step has gone, so the
    // camera is handed back only after.
    const marks = step.marks && {
      points: await Promise.all(ring(ZONES[step.marks[0]]).map((p) => groundCss(page, p))),
      png: await world(page),
    };
    if (step.camera && !marks) await released(ctx, page, step);
    if (step.until === "continue") await card.getByRole("button", { name: "Continue" }).click();
    else for (const signal of step.until.ui) await perform(page, card, signal);
    await page.waitForFunction(
      (before) =>
        document.querySelector("[aria-label=Coach]")?.getAttribute("data-step") !== before,
      at,
      { timeout: 10000 },
    );
    at = await shown();
    if (marks) {
      const after = await world(page);
      const painted = marks.points.filter((p) => p && mostChanged(marks.png, after, p, 4) > 150);
      ctx.check(
        `${step.id}: its zone is painted while it shows`,
        painted.length >= 4,
        `${painted.length} of 8`,
      );
      if (step.camera) await released(ctx, page, step);
    }
  }
}

const COACH = JSON.parse(
  readFileSync(new URL(`../../fixtures/maps/${MAP}/scenarios/${NAME}.coach.json`, import.meta.url)),
);
const ZONES = JSON.parse(
  readFileSync(new URL(`../../fixtures/maps/${MAP}/scenarios/${NAME}.json`, import.meta.url)),
).zones;

const camera = (page) => lab(page, () => window.__lab.camera());
const moved = (a, b) =>
  a.target.some((v, i) => Math.abs(v - b.target[i]) > 0.5) ||
  Math.abs(a.distance - b.distance) > 0.5;
/** Waits until the camera holds still for half a second. */
async function settled(page, limit = 30000) {
  const deadline = Date.now() + limit;
  let last = await camera(page);
  while (Date.now() < deadline) {
    await page.waitForTimeout(500);
    const now = await camera(page);
    if (!moved(last, now)) return true;
    last = now;
  }
  return false;
}

/** A step's tour moves the camera with no input, and ends. */
async function tour(ctx, page, step) {
  const before = await camera(page);
  await page.waitForTimeout(1200);
  ctx.check(`${step.id}: its tour flies the camera`, moved(before, await camera(page)));
  await snapshot(ctx, page, `coach-${step.id}-tour.png`);
  ctx.check(`${step.id}: its tour ends`, await settled(page));
  await snapshot(ctx, page, `coach-${step.id}-toured.png`);
}
/** Once a tour ends the camera is the player's again. */
async function released(ctx, page, step) {
  const after = await camera(page);
  await hold(page, "KeyW");
  ctx.check(`${step.id}: the player has the camera back`, moved(after, await camera(page)));
  await settled(page);
}

/** Eight points round a zone's edge. */
const ring = (zone) =>
  Array.from({ length: 8 }, (_, i) => [
    zone.center[0] + zone.radius_m * Math.cos((i * Math.PI) / 4),
    zone.center[1] + zone.radius_m * Math.sin((i * Math.PI) / 4),
  ]);
/** The world's pixels, with the HUD hidden. */
async function world(page) {
  const hide = await hideHud(page);
  try {
    await lab(page, () => window.__lab.frame());
    return decode(await page.screenshot());
  } finally {
    await hide.evaluate((el) => el.remove());
  }
}

const hold = async (page, code) => {
  await page.keyboard.down(code);
  await page.waitForTimeout(350);
  await page.keyboard.up(code);
};
/** Does what a UI signal hears: keys as the card names them. */
async function perform(page, card, signal) {
  const keys = await card.locator("kbd").allInnerTexts();
  await page.mouse.move(640, 400);
  if (signal === "camera_pan") await hold(page, "KeyW");
  else if (signal === "camera_rotate") await hold(page, `Key${keys[0]}`);
  else if (signal === "camera_tilt") await hold(page, `Key${keys[2]}`);
  else if (signal === "camera_zoom") await page.mouse.wheel(0, -300);
  else if (signal === "shop_opened")
    await page.getByRole("button", { name: "Reinforcements" }).click();
  else throw Error(`the scene can't do ${signal}`);
}
