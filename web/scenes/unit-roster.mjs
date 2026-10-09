// The player's production faction skirmish, through its ordinary picker and cursor.
import assert from "node:assert/strict";
import { modelGhostAgreement } from "./_modelGhost.mjs";
import { advance, obs, presented, snapshot } from "./_lab.mjs";
export async function run(ctx) {
  await modelGhostAgreement(ctx);
  const page = await ctx.newPage();
  const failures = [];
  page.on("response", async (response) => {
    if (response.status() >= 400)
      failures.push({
        url: response.url(),
        status: response.status(),
        body: (await response.text()).slice(0, 4000),
      });
  });
  try {
    await ctx.openLab(page, ctx.url, 45000);
  } catch (error) {
    await ctx.writeEvidence("startup-failures.json", failures);
    await page.screenshot({ path: ctx.evidencePath("startup.png") });
    throw error;
  }
  await page.evaluate(() => window.__lab.route.pause());
  await presented(page);
  const initial = await obs(page);
  assert(
    initial.skirmish?.phase === "preparation" && initial.own.length === 0,
    "skirmish starts in empty preparation",
  );
  assert(
    initial.skirmish.credits === 1000 && initial.skirmish.occupiedSlots === 0,
    "starting wallet and cap are observed",
  );
  await snapshot(ctx, page, "preparation.png");
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "VEH", exact: true }).click();
  await snapshot(ctx, page, "vehicle-families.png");
  // Hovering a family of variants opens their menu standing on its card,
  // 3 px above it, over the picker; clicking the card would buy at once.
  const abrams = page.getByRole("button", { name: "M1 Abrams", exact: true });
  await abrams.hover();
  const card = await abrams.boundingBox();
  const menu = await page.locator(".hud-purchase-flyout-panel").boundingBox();
  assert(
    menu !== null &&
      Math.abs(card.y - (menu.y + menu.height) - 3) < 0.5 &&
      Math.abs(menu.x - card.x) < 0.5,
    `the variant menu stands 3 px above its card: ${JSON.stringify({ card, menu })}`,
  );
  await snapshot(ctx, page, "abrams-variants.png");
  const variant = page.getByRole("button", { name: /^SEP v2 — \d+ credits$/ });
  await variant.click();
  await page.mouse.move(640, 340);
  await page.waitForTimeout(100);
  await snapshot(ctx, page, "purchase-ghost.png");
  await page.keyboard.press("Escape");
  assert((await obs(page)).skirmish.credits === 1000, "free placement and Escape spend no credits");
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("button", { name: "M1 Abrams", exact: true }).hover();
  await page.getByRole("button", { name: /^SEP v2 — \d+ credits$/ }).click();
  await page.mouse.move(640, 340);
  await page.waitForTimeout(100);
  await page.mouse.click(640, 340);
  await advance(page, 1);
  await page.waitForFunction(() => window.__lab.route.observation().skirmish.pending.length === 1);
  const reserved = await obs(page);
  assert(
    reserved.own.length === 0 &&
      reserved.skirmish.credits === 650 &&
      reserved.skirmish.occupiedSlots === 1,
    "cursor confirmation atomically reserves one variant during preparation",
  );
  await page.getByRole("button", { name: "START BATTLE", exact: true }).click();
  await advance(page, 60 * 30);
  const active = await obs(page);
  assert(
    active.own.length === 1 && active.skirmish.pending.length === 0,
    "reinforcement physically enters through the ordinary road-edge spawn",
  );
  await snapshot(ctx, page, "first-reinforcement.png");
  assert(active.skirmish.phase === "active", "ready/timeout starts the match");
  await ctx.writeEvidence("match.json", { initial: initial.skirmish, active: active.skirmish });
  await page.close();
}
