// An authored scenario's battle, opened by its address as the main menu's
// Tutorial tile will: the scenario map and document prepare under this
// build's rules, the battle opens in preparation over the player's road in,
// and the shop offers only what the document offers, a squad only in its
// offered ride.
import { lab, snapshot } from "./_lab.mjs";
import { playable } from "./generated.mjs";

const SCENARIO = "ferryman-valley/operation-ferryman";

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

/** The coach's card: the briefing's first step waits on the player's
 *  Continue, the next on the camera moving. */
async function coach(ctx, page) {
  const card = page.getByRole("complementary", { name: "Coach" });
  const first = await card.innerText();
  ctx.check("the coach opens the briefing", first.length > 0, first);
  await snapshot(ctx, page, "coach-continue.png");
  await card.getByRole("button", { name: "Continue" }).click();
  const second = await card.innerText();
  ctx.check("Continue moves the coach on", second !== first, second);
  await snapshot(ctx, page, "coach.png");
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
  const panStep = await card.textContent();
  await page.mouse.click(640, 400);
  await page.keyboard.down("KeyW");
  await page.waitForTimeout(400);
  await page.keyboard.up("KeyW");
  await page.waitForFunction(
    (before) => document.querySelector("[aria-label=Coach]")?.textContent !== before,
    panStep,
    { timeout: 10000 },
  );
  ctx.check("panning the camera finishes the pan step", true, await card.innerText());
}
