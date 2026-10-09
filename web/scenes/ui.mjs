// The UI gallery, scripted: every shot the gallery lists, drawn alone at
// 1280×800 and matched against its approved picture, with no panel's
// content outside the panel; the reinforcements picker on each category and
// with a family's variants open; then every main-menu page. The panel
// workbench (`panels.mjs`) pins the info panels and the deck, and the cursor
// lab (`cursor.mjs`) the cursor.
import { spills } from "./_spills.mjs";

/** The panels whose content must stay inside them. */
const PANELS = ".hud-panel, .hud-card, .hud-rejected, .frame-rate";

/** Never pinned: the menu's live 3D backdrop and the build's identity. */
const MENU_UNPINNED = "canvas, [data-testid=menu-build] { visibility: hidden !important; }";

async function contained(ctx, page, name) {
  const out = await spills(page, PANELS);
  ctx.check(
    `${name}: every panel holds its content`,
    out.length === 0,
    out.slice(0, 6).join(" | "),
  );
}

/** The picker's states on one gallery shot, each pinned. */
async function picker(ctx, page, shot) {
  await page.getByRole("button", { name: "Reinforcements" }).click();
  for (const tab of ["REC", "INF", "VEH", "HEL"]) {
    await page.getByRole("tab", { name: tab, exact: true }).click();
    await page.mouse.move(0, 0);
    await contained(ctx, page, `${shot} ${tab}`);
    await ctx.matchBaseline(page, `${shot}-${tab.toLowerCase()}`);
  }
  // A family of several variants lists them while the pointer is on it.
  await page.getByRole("tab", { name: "VEH", exact: true }).click();
  await page.getByRole("button", { name: "Tank", exact: true }).hover();
  ctx.check(
    `${shot}: hovering a family of variants lists them`,
    (await page.getByRole("group", { name: "Tank variants" }).count()) === 1,
  );
  await contained(ctx, page, `${shot} variants`);
  await ctx.matchBaseline(page, `${shot}-veh-variants`);
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1280, height: 800 } });
  await ctx.openLab(page, ctx.url);
  const shots = await page
    .getByTestId("shots")
    .locator("a")
    .evaluateAll((links) => links.map((a) => new URL(a.href).searchParams.get("shot")));
  ctx.check("the gallery lists its shots", shots.length > 10, String(shots.length));
  for (const shot of shots) {
    // Not `openLab`: the loading failure shot draws the error a lab's
    // failure would, on purpose.
    await page.goto(`${ctx.url}?shot=${shot}`);
    await page.locator(`[data-shot="${shot}"]`).waitFor();
    await page.evaluate(() => document.fonts.ready);
    await page.mouse.move(0, 0);
    if (shot.startsWith("purchase")) {
      await picker(ctx, page, shot);
      continue;
    }
    await contained(ctx, page, shot);
    await ctx.matchBaseline(page, shot);
  }

  // The main menu, each page of it, over its plain ground.
  const origin = new URL(ctx.url).origin;
  await page.goto(`${origin}/`);
  await page.locator("main.menu:not([inert])").waitFor({ timeout: 120_000 });
  // The loading screen's own plate leaves after the menu wakes.
  await page.waitForFunction(() => document.querySelectorAll(".menu-body").length === 1);
  await page.evaluate(() => document.fonts.ready);
  await page.mouse.move(0, 0);
  const body = page.locator("main.menu .menu-body");
  await ctx.matchBaseline(body, "menu", { style: MENU_UNPINNED });
  // The developer page lists every lab: it changes as labs come and go.
  for (const entry of ["skirmish", "replay", "settings"]) {
    await page.locator(`[data-page="${entry}"]`).click();
    await page.mouse.move(0, 0);
    await contained(ctx, page, `menu ${entry}`);
    await ctx.matchBaseline(body, `menu-${entry}`, { style: MENU_UNPINNED });
    await page.keyboard.press("Escape");
  }
  await page.close();
}
