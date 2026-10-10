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

/** Never pinned: the menu's live 3D backdrop (its film, grade and veil, all
 *  of it) and the build's identity. */
const MENU_UNPINNED = ".menu-backdrop, [data-testid=menu-build] { visibility: hidden !important; }";

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
  // Hovering a family shows its info cards, one per variant, in a menu
  // standing on its card; the picker itself never changes.
  await page.getByRole("tab", { name: "VEH", exact: true }).click();
  const picker = page.locator(".hud-purchase-picker");
  const tank = page.getByRole("button", { name: "Tank", exact: true });
  const before = { picker: await picker.boundingBox(), tank: await tank.boundingBox() };
  await page.getByRole("button", { name: "Supply Truck", exact: true }).hover();
  ctx.check(
    `${shot}: a family of one variant shows its one info card`,
    (await page
      .getByRole("group", { name: "Supply Truck variants" })
      .locator(".hud-purchase-info")
      .count()) === 1,
  );
  // A menu is open (the supply truck's), so the hover moves after a rest.
  await tank.hover();
  await page.waitForTimeout(400);
  const after = { picker: await picker.boundingBox(), tank: await tank.boundingBox() };
  const menu = await page.locator(".hud-purchase-stack").boundingBox();
  ctx.check(
    `${shot}: hovering a family leaves the picker and its card where they were`,
    ["x", "y", "width", "height"].every(
      (k) =>
        Math.abs(after.picker[k] - before.picker[k]) < 0.5 &&
        Math.abs(after.tank[k] - before.tank[k]) < 0.5,
    ),
    JSON.stringify({ before, after }),
  );
  // Like a web hover menu: it stands right on top of its card, over the
  // picker's own header if it must, never off at the picker's edge.
  const gap = after.tank.y - (menu?.y ?? 0) - (menu?.height ?? 0);
  ctx.check(
    `${shot}: the variants open right above the hovered card`,
    menu !== null &&
      gap >= 0 &&
      gap <= 6 &&
      menu.x < after.tank.x + after.tank.width &&
      menu.x + menu.width > after.tank.x,
    JSON.stringify({ gap, menu, tank: after.tank }),
  );

  // One info card per variant, stacked at one left edge and one width,
  // each plate opaque (a dimmed one dims its words, not what's behind it).
  const stacked = await page
    .getByRole("group", { name: "Tank variants" })
    .locator(".hud-purchase-info")
    .evaluateAll((cards) =>
      cards.map((c) => {
        const r = c.getBoundingClientRect();
        return {
          x: r.left,
          w: r.width,
          top: r.top,
          bottom: r.bottom,
          opacity: getComputedStyle(c).opacity,
        };
      }),
    );
  // One gap throughout: between the stacked cards as between the stack and
  // the unit card under it.
  const tankBox = await tank.boundingBox();
  const gaps = [
    ...stacked.slice(1).map((c, k) => c.top - stacked[k].bottom),
    tankBox.y - stacked.at(-1).bottom,
  ];
  ctx.check(
    `${shot}: the info cards keep one 3 px gap, between them and above the card`,
    gaps.every((g) => Math.abs(g - 3) < 0.5),
    JSON.stringify(gaps),
  );
  // The dearest at the top, the cheapest at the bottom, nearest the pointer.
  const prices = await page
    .getByRole("group", { name: "Tank variants" })
    .locator(".hud-purchase-price")
    .evaluateAll((ps) => ps.map((p) => parseInt(p.textContent)));
  ctx.check(
    `${shot}: variants run from the dearest at the top to the cheapest at the bottom`,
    prices.length === 2 && prices.every((p, k) => k === 0 || p <= prices[k - 1]),
    JSON.stringify(prices),
  );
  ctx.check(
    `${shot}: the variants' info cards stack at one edge and width, opaque`,
    stacked.length === 2 &&
      stacked.every(
        (c, k) =>
          Math.abs(c.x - stacked[0].x) < 0.5 &&
          Math.abs(c.w - stacked[0].w) < 0.5 &&
          c.opacity === "1" &&
          (k === 0 || c.top >= stacked[k - 1].bottom),
      ),
    JSON.stringify(stacked),
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
  await contained(ctx, page, `${shot} variants`);
  await ctx.matchBaseline(page, `${shot}-veh-variants`);
  // The pointer travels up from the card into the menu without closing it,
  // and leaving the picker closes it.
  const variant = page.getByRole("button", { name: /^Trophy — / });
  const to = await variant.boundingBox();
  await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 20 });
  ctx.check(
    `${shot}: the pointer reaches a variant with its menu still open`,
    await variant.isVisible(),
  );
  // However the menu moves to another family, by resting on it or by
  // clicking it at once, it stands on that family's own card.
  const standsOn = async (family) => {
    const card = await page.getByRole("button", { name: family, exact: true }).boundingBox();
    const menu = await page.locator(".hud-purchase-stack").boundingBox();
    const named = (await page.locator(".hud-purchase-stack").getAttribute("aria-label")).replace(
      / variants$/,
      "",
    );
    const gap = card.y - (menu?.y ?? 0) - (menu?.height ?? 0);
    return {
      ok:
        menu !== null &&
        named.toLowerCase() === family.toLowerCase() &&
        gap >= 0 &&
        gap <= 6 &&
        Math.abs(menu.x - card.x) < 0.5,
      detail: JSON.stringify({ named, gap, menu, card }),
    };
  };
  const jeep = page.getByRole("button", { name: "Jeep", exact: true });
  await tank.hover();
  await jeep.hover();
  await page.waitForTimeout(400);
  let at = await standsOn("Jeep");
  ctx.check(`${shot}: resting on another family moves its menu onto that card`, at.ok, at.detail);
  // A click that buys nothing (the tank is beyond the credits) keeps the
  // picker open; its menu still stands on the clicked card.
  if (short) {
    await tank.click();
    at = await standsOn("Tank");
    ctx.check(`${shot}: clicking another family moves its menu onto that card`, at.ok, at.detail);
  }
  await pointerAway(page);
  ctx.check(
    `${shot}: leaving the picker closes the variants`,
    (await page.locator(".hud-purchase-flyout").count()) === 0,
  );
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
  // A picture of a page whose art never loaded (an unfetched LFS pointer
  // decodes to nothing) approves a broken page: every image must decode.
  const undecoded = await page.evaluate(async () => {
    // <img>s, and every CSS background (pseudo-elements too, where the
    // menu's card art lives).
    const urls = new Set([...document.images].map((i) => i.src));
    for (const e of document.querySelectorAll("*"))
      for (const pseudo of [null, "::before", "::after"])
        for (const [, url] of getComputedStyle(e, pseudo).backgroundImage.matchAll(
          /url\("?([^")]+)"?\)/g,
        ))
          urls.add(url);
    const failed = [];
    for (const url of urls) {
      const image = new Image();
      image.src = url;
      await image.decode().catch(() => failed.push(url.split("/").pop()));
    }
    return failed;
  });
  ctx.check(
    "every menu image decodes (art fetched, not an LFS pointer)",
    undecoded.length === 0,
    undecoded.join(", "),
  );
  const body = page.locator("main.menu .menu-body");
  await ctx.matchBaseline(body, "menu", { style: MENU_UNPINNED });
  // The developer page lists every lab: it changes as labs come and go.
  for (const entry of ["skirmish", "replay", "tutorial", "settings"]) {
    await page.locator(`[data-page="${entry}"]`).click();
    await pointerAway(page);
    await contained(ctx, page, `menu ${entry}`);
    await ctx.matchBaseline(body, `menu-${entry}`, { style: MENU_UNPINNED });
    await page.keyboard.press("Escape");
  }
  await page.close();
}
