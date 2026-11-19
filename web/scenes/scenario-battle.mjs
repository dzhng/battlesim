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
  await restartChapter(ctx, page);
  await page.close();
  await startedLater(ctx);
  await victory(ctx);
}

/** The story's end: started where the last chapter's match is decided, the
 *  battle ends in the player's victory and the coach's closing step stays up
 *  over the result. */
async function victory(ctx) {
  const last = COACH.chapters.at(-1);
  const won = [...last.steps]
    .reverse()
    .find((s) => typeof s.until === "object" && "beat" in s.until).until.beat;
  const page = await ctx.newPage();
  const url = new URL(ctx.url);
  url.search = `?scenario=${SCENARIO}&chapter=${last.id}&beat=${won}`;
  await page.goto(url.href);
  await playable(page, ctx.readinessTimeout, 1);
  await page.getByRole("dialog", { name: "Match result" }).waitFor({ timeout: 60000 });
  const card = page.getByRole("complementary", { name: "Coach" });
  // Continue through what the player hasn't dismissed in this run.
  const closing = last.steps.at(-1).id;
  for (
    let i = 0;
    i < last.steps.length && (await card.getAttribute("data-step")) !== closing;
    i++
  ) {
    const go = card.getByRole("button", { name: "Continue" });
    if (await go.count()) await go.click();
    else await page.waitForTimeout(500);
  }
  ctx.check(
    "the battle ends in victory with the coach's closing step up",
    (await card.getAttribute("data-step")) === closing,
    await card.getAttribute("data-step"),
  );
  await snapshot(ctx, page, "victory.png");
  await page.close();
}

/** Where each later chapter's UI-gated steps begin: the chapter's opening,
 *  or the beat step just before them. The walkthrough plays the beats, never
 *  the UI, so these are played here. */
function uiStarts() {
  return COACH.chapters.slice(1).flatMap((chapter) => {
    const starts = new Map();
    chapter.steps.forEach((step, i) => {
      if (typeof step.until !== "object" || !("ui" in step.until)) return;
      const before = chapter.steps
        .slice(0, i)
        .reverse()
        .find((s) => typeof s.until === "object" && "beat" in s.until);
      starts.set(before?.until.beat ?? null, step.id);
    });
    return [...starts].map(([beat, last]) => ({ chapter, beat, last }));
  });
}

/** The chapters' dev loop: a battle started at a chapter, or where one of
 *  its beats fired, shows the coach's next step, and the player plays on
 *  from there: each UI step the coach shows is done, up to a beat step. */
async function startedLater(ctx) {
  for (const { chapter, beat, last } of uiStarts()) {
    const page = await ctx.newPage();
    const url = new URL(ctx.url);
    url.search = `?scenario=${SCENARIO}&chapter=${chapter.id}${beat ? `&beat=${beat}` : ""}`;
    await page.goto(url.href);
    await playable(page, ctx.readinessTimeout, 1);
    const card = page.getByRole("complementary", { name: "Coach" });
    await card.waitFor({ timeout: 60000 });
    const done = [];
    for (;;) {
      const at = await card.getAttribute("data-step");
      const step = chapter.steps.find((s) => s.id === at);
      if (!step || typeof step.until !== "object" || !("ui" in step.until)) break;
      await snapshot(ctx, page, `coach-${chapter.id}-${step.id}.png`);
      for (const signal of step.until.ui) await perform(page, card, signal);
      await page.waitForFunction(
        (id) => document.querySelector("[aria-label=Coach]")?.getAttribute("data-step") !== id,
        step.id,
        { timeout: 10000 },
      );
      done.push(step.id);
    }
    ctx.check(
      `${chapter.id}, started ${beat ? `after ${beat}` : "at its opening"}: its UI steps through ${last} are done by doing them`,
      done.includes(last),
      done.join(", "),
    );
    await page.close();
  }
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

/** "Restart chapter" from the pause menu takes the battle back to the open
 *  chapter's start: the coach shows the chapter's first step again, having
 *  forgotten what the player did in it. */
async function restartChapter(ctx, page) {
  const [chapter] = COACH.chapters;
  const card = page.getByRole("complementary", { name: "Coach" });
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  const restart = page.getByRole("button", { name: "Restart chapter", exact: true });
  await restart.waitFor({ timeout: 30000 });
  await restart.click();
  await page.getByRole("button", { name: "Restart chapter", exact: true }).click();
  const asked = Date.now();
  await page.waitForFunction(
    (first) => document.querySelector("[aria-label=Coach]")?.getAttribute("data-step") === first,
    chapter.steps[0].id,
    { timeout: 60000 },
  );
  ctx.check(
    "restarting the chapter brings back its first step",
    (await card.getAttribute("data-step")) === chapter.steps[0].id,
    `${((Date.now() - asked) / 1000).toFixed(1)} s from the confirmation`,
  );
  // The view draws again once its world is back.
  await lab(page, () => window.__lab.frame());
  await snapshot(ctx, page, "coach-restarted.png");
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
/** The card's placeholder whose first key does what each signal hears. */
const SIGNAL_KEYS = {
  camera_pan: "camera:forward",
  camera_rotate: "camera:turn",
  camera_tilt: "camera:pitch",
  contact_focus: "key:contact_focus",
  group_set: "key:group_assign",
  show_orders: "key:show_orders",
};
/** Does what a UI signal hears: keys as the card names them. */
async function perform(page, card, signal) {
  const named = SIGNAL_KEYS[signal];
  const [key] = named ? await card.locator(`[data-keys="${named}"] kbd`).allInnerTexts() : [];
  await page.mouse.move(640, 400);
  if (named?.startsWith("camera:")) await hold(page, `Key${key}`);
  else if (signal === "camera_zoom") await page.mouse.wheel(0, -300);
  else if (signal === "shop_opened")
    await page.getByRole("button", { name: "Reinforcements" }).click();
  else if (signal === "selected")
    await page.locator('[data-hud-anchor="army"] button:not([disabled])').first().click();
  else if (signal === "moved_with_facing") {
    await page.mouse.move(640, 380);
    await page.mouse.down({ button: "right" });
    await page.waitForTimeout(100);
    await page.mouse.move(800, 300, { steps: 20 });
    await page.waitForTimeout(100);
    await page.mouse.up({ button: "right" });
  } else if (signal === "contact_focus") await page.keyboard.press(`Key${key}`);
  else if (signal === "group_set") {
    // Select a unit, then the group key the card names.
    await page.locator('[data-hud-anchor="army"] button:not([disabled])').first().click();
    await page.keyboard.press(key.replace("Ctrl", "Control").replace(/\+(\d)$/, "+Digit$1"));
  } else if (signal === "show_orders") {
    // The card names the key to hold.
    await page.keyboard.down(key);
    await page.waitForTimeout(500);
    await page.keyboard.up(key);
  } else throw Error(`the scene can't do ${signal}`);
}
