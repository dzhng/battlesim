// The Armory in the real menu, on the game's roster: the tile opens it, every
// pick (a tile, a group, a faction) rewrites `?unit=` in place, with no new
// history entry and no remount, a refresh reopens the same unit, an unknown
// unit opens the first tile, and Back returns to the menu on the Armory's
// tile. Its approved pictures are the UI gallery's, on fixed content
// (`ui.mjs`); this scene leaves shots of the real page in its evidence.
import { menuPage } from "../../apps/battle-lab/src/menu/pages.ts";

const MENU = "main.menu:not(.loading):not([inert])";

function seedGraphics() {
  localStorage.setItem(
    "battle.graphics",
    JSON.stringify({
      mode: "high",
      detectedMode: "high",
      lastResult: { averageFps: 60, onePercentLowFps: 60, selectedMode: "high" },
    }),
  );
}

/** The menu woken at `path`, its cover gone. */
async function openMenu(page, path) {
  await page.goto(path);
  await page.locator(MENU).waitFor({ timeout: 120_000 });
  await page.getByTestId("loading").waitFor({ state: "hidden", timeout: 120_000 });
}

const unit = (page) => new URL(page.url()).searchParams.get("unit");

export async function run(ctx) {
  const origin = new URL(ctx.url).origin;
  const armory = menuPage("armory").path;
  const page = await ctx.newPage({ viewport: { width: 1280, height: 800 } });
  await page.addInitScript(seedGraphics);
  await openMenu(page, `${origin}/`);
  await page.locator(`${MENU} [data-page="armory"]`).click();
  const list = page.getByRole("region", { name: "Units" });
  await list.waitFor();
  const opened = unit(page);
  ctx.check(
    "the tile opens the Armory at its own address, on a unit",
    new URL(page.url()).pathname === armory && opened !== null,
    page.url(),
  );

  await page.screenshot({ path: ctx.evidencePath("armory-1280.png") });

  // Picks change the page in place: the list's element survives them all.
  await list.evaluate((e) => (window.__armoryList = e));
  const length = await page.evaluate(() => history.length);
  const tiles = list.locator("button[data-family]");
  await tiles.nth(1).click();
  const picked = unit(page);
  await list
    .getByRole("tablist", { name: "Unit groups" })
    .locator('[role="tab"]:not([aria-selected="true"]):not([disabled])')
    .first()
    .click();
  const grouped = unit(page);
  await list.getByRole("tablist", { name: "Factions" }).getByRole("tab").nth(1).click();
  ctx.check(
    "a pick of a tile, a group and a faction each rewrite the address",
    picked !== opened && grouped !== picked && unit(page) !== null,
    JSON.stringify({ opened, picked, grouped, faction: unit(page) }),
  );
  ctx.check(
    "picks add no history entry and never remount the page",
    (await page.evaluate(() => history.length)) === length &&
      (await list.evaluate((e) => e === window.__armoryList)),
  );

  // A refresh reopens the same unit, its tile pressed.
  const kept = unit(page);
  await openMenu(page, page.url());
  await list.waitFor();
  const pressed = await list.locator('button[data-family][aria-pressed="true"]').count();
  ctx.check(
    "a refresh reopens the same unit",
    unit(page) === kept && pressed === 1,
    `${kept} → ${unit(page)}`,
  );

  // An unknown unit opens the first tile and says so in the address.
  await openMenu(page, `${origin}${armory}?unit=no_such_card`);
  await list.waitFor();
  ctx.check(
    "an unknown unit opens the first tile",
    unit(page) !== "no_such_card" && (await tiles.first().getAttribute("aria-pressed")) === "true",
    page.url(),
  );

  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.screenshot({ path: ctx.evidencePath("armory-1920.png") });

  // Escape leaves for the menu, on the Armory's tile.
  await page.keyboard.press("Escape");
  await page.locator(`${MENU} [data-page="armory"]`).waitFor();
  ctx.check(
    "Escape returns to the menu on the Armory's tile",
    new URL(page.url()).pathname === "/" &&
      (await page.evaluate(() => document.activeElement?.getAttribute("data-page"))) === "armory",
    page.url(),
  );
  await page.close();
}
