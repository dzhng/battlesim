// The UI gallery, scripted: every shot the gallery lists, drawn alone at
// 1280×800 and matched against its approved picture, with no panel's
// content outside the panel; the reinforcements picker on each category and
// with a family's variants open; then every main-menu page. The panel
// workbench (`panels.mjs`) pins the info panels and the deck, and the cursor
// lab (`cursor.mjs`) the cursor.
import { pointerAway } from "./_baseline.mjs";
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
  const sizes = [];
  for (const tab of ["REC", "INF", "VEH", "HEL"]) {
    await page.getByRole("tab", { name: tab, exact: true }).click();
    await pointerAway(page);
    await contained(ctx, page, `${shot} ${tab}`);
    sizes.push(
      ...(await page
        .locator(".hud-purchase-family")
        .evaluateAll((cards) =>
          cards.map((c) => [Math.round(c.offsetWidth), Math.round(c.offsetHeight)]),
        )),
    );
    await ctx.matchBaseline(page, `${shot}-${tab.toLowerCase()}`);
  }
  // A unit for sale is the army's own vertical card: the deck's card size,
  // whether its tab holds one family or many.
  const deckCard = await page
    .locator(".hud-army .hud-army-card")
    .first()
    .evaluate((c) => [Math.round(c.offsetWidth), Math.round(c.offsetHeight)]);
  ctx.check(
    `${shot}: every family is the army's card, at its size`,
    sizes.length > 2 && sizes.every(([w, h]) => w === deckCard[0] && h === deckCard[1]),
    JSON.stringify({ deckCard, sizes }),
  );
  // A family of several variants lists them while the pointer is on it.
  await page.getByRole("tab", { name: "VEH", exact: true }).click();
  // Opening a family's variants never moves the cards under the pointer.
  const tank = page.getByRole("button", { name: "Tank", exact: true });
  const before = await tank.boundingBox();
  await tank.hover();
  const after = await tank.boundingBox();
  ctx.check(
    `${shot}: hovering a family leaves its card where the pointer found it`,
    Math.abs(after.y - before.y) < 0.5 && Math.abs(after.x - before.x) < 0.5,
    JSON.stringify({ before, after }),
  );
  ctx.check(
    `${shot}: hovering a family of variants lists them`,
    (await page.getByRole("group", { name: "Tank variants" }).count()) === 1,
  );
  // Variants side by side share their top edge and their first line,
  // however many lines each has.
  const variants = await page
    .getByRole("group", { name: "Tank variants" })
    .locator("button")
    .evaluateAll((buttons) =>
      buttons.map((b) => [
        b.getBoundingClientRect().top,
        b.querySelector("span").getBoundingClientRect().top,
      ]),
    );
  ctx.check(
    `${shot}: variants share their top edge and first line`,
    variants.length > 1 &&
      variants.every(
        ([box, line]) =>
          Math.abs(box - variants[0][0]) < 0.5 && Math.abs(line - variants[0][1]) < 0.5,
      ),
    JSON.stringify(variants),
  );
  // What can't be bought now is dimmed, as its variants are; the rest is not.
  const opacity = (family) =>
    page
      .getByRole("button", { name: family, exact: true })
      .evaluate((b) => Number(getComputedStyle(b).opacity));
  const short = shot === "purchase-short-of-credits";
  ctx.check(
    `${shot}: a family is dimmed exactly when it can't be bought now`,
    (await opacity("Tank")) < 1 === short && (await opacity("Jeep")) === 1,
    JSON.stringify({ tank: await opacity("Tank"), jeep: await opacity("Jeep") }),
  );
  // A price the player can't pay is the warning that says why, on the
  // family and on its variant alike; one they can pay is not.
  const warned = (locator) =>
    locator.locator(".hud-purchase-price").evaluate((price) => {
      const probe = document.createElement("span");
      probe.style.color = "rgb(var(--hud-warn))";
      price.parentElement.append(probe);
      const warn = getComputedStyle(probe).color;
      probe.remove();
      return getComputedStyle(price).color === warn;
    });
  const tankWarned = await warned(page.getByRole("button", { name: "Tank", exact: true }));
  const baseWarned = await warned(page.getByRole("button", { name: /^Base — / }));
  const jeepWarned = await warned(page.getByRole("button", { name: "Jeep", exact: true }));
  ctx.check(
    `${shot}: a price is the warning colour exactly when it can't be paid`,
    tankWarned === short && baseWarned === short && !jeepWarned,
    JSON.stringify({ tankWarned, baseWarned, jeepWarned }),
  );
  // Each variant's diamond marks its name, its first line.
  const diamonds = await page
    .getByRole("group", { name: "Tank variants" })
    .locator("button")
    .evaluateAll((buttons) =>
      buttons.map((b) => {
        const mark = getComputedStyle(b, "::before");
        const centre =
          b.getBoundingClientRect().top + parseFloat(mark.top) + parseFloat(mark.marginTop);
        const line = b.querySelector("span").getBoundingClientRect();
        return centre - (line.top + line.height / 2);
      }),
    );
  ctx.check(
    `${shot}: each variant's diamond sits on its name's line (within 1.5 px)`,
    diamonds.length > 1 && diamonds.every((d) => Math.abs(d) <= 1.5),
    JSON.stringify(diamonds),
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
    await pointerAway(page);
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
  await pointerAway(page);
  const body = page.locator("main.menu .menu-body");
  await ctx.matchBaseline(body, "menu", { style: MENU_UNPINNED });
  // The developer page lists every lab: it changes as labs come and go.
  for (const entry of ["skirmish", "replay", "settings"]) {
    await page.locator(`[data-page="${entry}"]`).click();
    await pointerAway(page);
    await contained(ctx, page, `menu ${entry}`);
    await ctx.matchBaseline(body, `menu-${entry}`, { style: MENU_UNPINNED });
    await page.keyboard.press("Escape");
  }
  await page.close();
}
