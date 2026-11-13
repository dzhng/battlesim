// The UI gallery, scripted: every shot the gallery lists, drawn alone at
// 1280×800 and matched against its approved picture, with no panel's
// content outside the panel; the reinforcements picker on its groups and
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
/** Also never pinned here: the lobby's map, a fresh seed each visit (the
 *  map-picture scene judges the picture). */
const LOBBY_UNPINNED = `${MENU_UNPINNED} .lobby-picture, .lobby-seed { visibility: hidden !important; }`;

async function contained(ctx, page, name) {
  const out = await spills(page, PANELS);
  ctx.check(
    `${name}: every panel holds its content`,
    out.length === 0,
    out.slice(0, 6).join(" | "),
  );
}

/** A picture of a page whose art never loaded (an unfetched LFS pointer
 *  decodes to nothing) approves a broken page: every image and video the page
 *  names must decode, whether or not it has scrolled into view. */
async function mediaDecodes(ctx, page, name) {
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
    for (const { src } of document.querySelectorAll("video")) {
      const video = document.createElement("video");
      video.muted = true;
      video.src = src;
      const loaded = await new Promise((resolve) => {
        video.onloadeddata = () => resolve(true);
        video.onerror = () => resolve(false);
      });
      if (!loaded) failed.push(src.split("/").pop());
    }
    return failed;
  });
  ctx.check(
    `${name}: every image and video decodes (fetched, not an LFS pointer)`,
    undecoded.length === 0,
    undecoded.join(", "),
  );
}

/** The match report's geometry: it stands inside the window, every
 *  player's row reads on one line, and the battle's story has room for
 *  several lines however many players a side fields. */
async function report(ctx, page, shot) {
  const m = await page.evaluate(() => {
    const box = (e) => e.getBoundingClientRect();
    const modal = box(document.querySelector(".match-result-modal"));
    const rows = [...document.querySelectorAll(".match-result-side tbody tr")].map((r) => ({
      height: box(r).height,
      clipped: [...r.children].some((c) => c.scrollWidth > c.clientWidth + 0.5),
    }));
    const timeline = document.querySelector(".match-result-timeline");
    const line = timeline.querySelector("li");
    return {
      inside:
        modal.top >= 0 &&
        modal.left >= 0 &&
        modal.bottom <= innerHeight &&
        modal.right <= innerWidth,
      rows,
      timeline: box(timeline).height,
      line: line ? box(line).height : 0,
    };
  });
  ctx.check(`${shot}: the report stands inside the window`, m.inside);
  ctx.check(
    `${shot}: every player's row reads whole on one line`,
    m.rows.length > 0 && m.rows.every((r) => !r.clipped && r.height < 32),
    JSON.stringify(m.rows),
  );
  ctx.check(
    `${shot}: the timeline shows at least four lines`,
    m.line > 0 && m.timeline >= 4 * m.line,
    JSON.stringify({ timeline: m.timeline, line: m.line }),
  );
}

/** The tabs a picture is pinned on: a family of several variants, two
 *  families, one family, and none. A baseline names its tab without the
 *  label's dot. */
const PINNED_TABS = ["REC", "INF", "MBT", "IFV·APC", "HELI"];
const tabFile = (tab) => tab.toLowerCase().replace("·", "-");

/** The tab row holds every group and the search, at the gallery's 1280 px:
 *  one row, no label clipped or wrapped, and the search beside the tabs.
 *  The containment check beside it proves the row stays inside the picker. */
async function tabRow(ctx, page, shot) {
  const row = await page.evaluate(() => {
    const box = (e) => e.getBoundingClientRect();
    const tabs = [...document.querySelectorAll(".hud-purchase-groups [role=tab]")].map((t) => ({
      label: t.textContent,
      top: box(t).top,
      right: box(t).right,
      height: box(t).height,
      clipped: t.scrollWidth > t.clientWidth + 0.5,
    }));
    return { tabs, search: box(document.querySelector(".hud-purchase-search")).left };
  });
  const [first] = row.tabs;
  ctx.check(
    `${shot}: every group's tab and the search share one row, each label whole`,
    row.tabs.length === 9 &&
      row.tabs.every(
        (t) => Math.abs(t.top - first.top) < 0.5 && t.height === first.height && !t.clipped,
      ) &&
      row.tabs.at(-1).right <= row.search,
    JSON.stringify(row),
  );
}

/** The picker's states on one gallery shot, each pinned. */
async function picker(ctx, page, shot) {
  await page.getByRole("button", { name: "Reinforcements" }).click();
  await tabRow(ctx, page, shot);
  const sizes = [];
  for (const tab of PINNED_TABS) {
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
    await ctx.matchBaseline(page, `${shot}-${tabFile(tab)}`);
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
  const short = shot === "purchase-short-of-credits";
  // What can't be bought now is dimmed, as its variants are; the rest is not.
  const opacity = (family) =>
    page
      .getByRole("button", { name: family, exact: true })
      .evaluate((b) => Number(getComputedStyle(b).opacity));
  // A price the player can't pay is the "can't" red that says why, on the
  // family and on its variant alike; one they can pay is the credits gold;
  // what is never sold is plain text.
  const priced = (locator, role) =>
    locator.locator(".hud-purchase-price").evaluate((price, role) => {
      const probe = document.createElement("span");
      probe.style.color = `rgb(var(--hud-${role}))`;
      price.parentElement.append(probe);
      const colour = getComputedStyle(probe).color;
      probe.remove();
      return getComputedStyle(price).color === colour;
    }, role);

  // Two families on one tab. Hovering a family shows its info cards, one per
  // variant, in a menu standing on its card; the picker itself never changes.
  await page.getByRole("tab", { name: "IFV·APC", exact: true }).click();
  const picker = page.locator(".hud-purchase-picker");
  const jeep = page.getByRole("button", { name: "Jeep", exact: true });
  const apc = page.getByRole("button", { name: "APC", exact: true });
  const before = { picker: await picker.boundingBox(), jeep: await jeep.boundingBox() };
  await apc.hover();
  ctx.check(
    `${shot}: a family of one variant shows its one info card`,
    (await page
      .getByRole("group", { name: "APC variants" })
      .locator(".hud-purchase-info")
      .count()) === 1,
  );
  // A menu is open (the APC's), so the hover moves after a rest.
  await jeep.hover();
  await page.waitForTimeout(400);
  const after = { picker: await picker.boundingBox(), jeep: await jeep.boundingBox() };
  const menu = await page.locator(".hud-purchase-stack").boundingBox();
  ctx.check(
    `${shot}: hovering a family leaves the picker and its card where they were`,
    ["x", "y", "width", "height"].every(
      (k) =>
        Math.abs(after.picker[k] - before.picker[k]) < 0.5 &&
        Math.abs(after.jeep[k] - before.jeep[k]) < 0.5,
    ),
    JSON.stringify({ before, after }),
  );
  // Like a web hover menu: it stands right on top of its card, over the
  // picker's own header if it must, never off at the picker's edge.
  const gap = after.jeep.y - (menu?.y ?? 0) - (menu?.height ?? 0);
  ctx.check(
    `${shot}: the variants open right above the hovered card`,
    menu !== null &&
      gap >= 0 &&
      gap <= 6 &&
      menu.x < after.jeep.x + after.jeep.width &&
      menu.x + menu.width > after.jeep.x,
    JSON.stringify({ gap, menu, jeep: after.jeep }),
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
  let at = await standsOn("Jeep");
  ctx.check(`${shot}: resting on another family moves its menu onto that card`, at.ok, at.detail);
  // A click that buys nothing (the APC is beyond the credits) keeps the
  // picker open; its menu still stands on the clicked card.
  if (short) {
    await apc.click();
    at = await standsOn("APC");
    ctx.check(`${shot}: clicking another family moves its menu onto that card`, at.ok, at.detail);
  }
  const jeepPrice = {
    opacity: await opacity("Jeep"),
    refused: await priced(jeep, "bad"),
    gold: await priced(jeep, "credits"),
  };
  await pointerAway(page);
  ctx.check(
    `${shot}: leaving the picker closes the variants`,
    (await page.locator(".hud-purchase-flyout").count()) === 0,
  );

  // A family of two variants, one never sold: its info cards stacked.
  await page.getByRole("tab", { name: "MBT", exact: true }).click();
  const tank = page.getByRole("button", { name: "Tank", exact: true });
  await tank.hover();
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
  const tankOpacity = await opacity("Tank");
  ctx.check(
    `${shot}: a family is dimmed exactly when it can't be bought now`,
    tankOpacity < 1 === short && jeepPrice.opacity === 1,
    JSON.stringify({ tank: tankOpacity, jeep: jeepPrice.opacity }),
  );
  {
    const base = page.getByRole("button", { name: /^Base — / });
    const refused = {
      tank: await priced(tank, "bad"),
      base: await priced(base, "bad"),
      jeep: jeepPrice.refused,
    };
    const gold = { tank: await priced(tank, "credits"), jeep: jeepPrice.gold };
    const unsold = page.getByRole("button", { name: / — Unavailable$/ }).first();
    const plain =
      (await unsold.count()) === 0 ||
      (!(await priced(unsold, "bad")) && !(await priced(unsold, "credits")));
    ctx.check(
      `${shot}: a price is the "can't" colour exactly when it can't be paid, gold when it can, plain when never sold`,
      refused.tank === short &&
        refused.base === short &&
        !refused.jeep &&
        gold.jeep &&
        gold.tank === !short &&
        plain,
      JSON.stringify({ refused, gold, plain }),
    );
  }
  await contained(ctx, page, `${shot} variants`);
  await ctx.matchBaseline(page, `${shot}-mbt-variants`);
  // The pointer travels up from the card into the menu without closing it,
  // and leaving the picker closes it.
  const variant = page.getByRole("button", { name: /^Trophy — / });
  const to = await variant.boundingBox();
  await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 20 });
  ctx.check(
    `${shot}: the pointer reaches a variant with its menu still open`,
    await variant.isVisible(),
  );
  await pointerAway(page);
  ctx.check(
    `${shot}: leaving the picker closes the variants`,
    (await page.locator(".hud-purchase-flyout").count()) === 0,
  );
  // The filter narrows the grid across groups; the picture pins the field
  // in the tab row. Captured once, on the full-wallet shot.
  if (!short) {
    const field = page.getByRole("textbox", { name: "Search units" });
    await field.fill("tank");
    await pointerAway(page);
    await contained(ctx, page, `${shot} search`);
    await ctx.matchBaseline(page, "purchase-search");
    await field.fill("");
  }
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1280, height: 800 } });
  await page.addInitScript(() => {
    localStorage.setItem(
      "battle.graphics",
      JSON.stringify({
        mode: "high",
        detectedMode: "high",
        lastResult: { averageFps: 60, onePercentLowFps: 60, selectedMode: "high" },
      }),
    );
  });
  await ctx.openLab(page, ctx.url);
  const shots = await page
    .getByTestId("shots")
    .locator("a")
    .evaluateAll((links) => links.map((a) => new URL(a.href).searchParams.get("shot")));
  ctx.check("the gallery lists its shots", shots.length > 10, String(shots.length));
  for (const shot of shots) {
    // Not `openLab`: the loading failure shot draws the error a lab's
    // failure would, on purpose.
    await page.goto(ctx.urlFor({ shot }));
    await page.locator(`[data-shot="${shot}"]`).waitFor();
    await page.evaluate(() => document.fonts.ready);
    await pointerAway(page);
    if (shot.startsWith("purchase")) {
      await picker(ctx, page, shot);
      continue;
    }
    await contained(ctx, page, shot);
    if (shot.startsWith("report")) await report(ctx, page, shot);
    await ctx.matchBaseline(page, shot);
  }

  // The main menu, each page of it, over its plain ground.
  const origin = new URL(ctx.url).origin;
  await page.goto(`${origin}/`);
  await page.locator("main.menu:not(.loading):not([inert])").waitFor({ timeout: 120_000 });
  // The loading screen's own plate leaves after the menu wakes.
  await page.getByTestId("loading").waitFor({ state: "hidden", timeout: 120_000 });
  await page.evaluate(() => document.fonts.ready);
  await pointerAway(page);
  await mediaDecodes(ctx, page, "menu");
  const body = page.locator("main.menu:not(.loading):not([inert]) .menu-body");
  await ctx.matchBaseline(body, "menu", { style: MENU_UNPINNED });
  const home = await body.boundingBox();
  // Every page is centred by its own measured height, including after the
  // first-launch graphics detector has replaced the plate.
  for (const entry of ["skirmish", "settings", "developer"]) {
    await page.locator(`main.menu:not(.loading):not([inert]) [data-page="${entry}"]`).click();
    const box = await body.boundingBox();
    const viewportHeight = await page.evaluate(() => innerHeight);
    ctx.check(
      `menu ${entry}: the plate is vertically centred`,
      box !== null && Math.abs(box.y + box.height / 2 - viewportHeight / 2) < 1,
      JSON.stringify({ box, viewportHeight }),
    );
    await page.keyboard.press("Escape");
  }
  // The developer page lists every lab: it changes as labs come and go.
  for (const entry of ["skirmish", "tutorial", "settings"]) {
    await page.locator(`main.menu:not(.loading):not([inert]) [data-page="${entry}"]`).click();
    await pointerAway(page);
    await mediaDecodes(ctx, page, `menu ${entry}`);
    // The page's pictures, drawn before the picture of the page is taken.
    await page.waitForFunction(() =>
      [...document.querySelectorAll(".menu-body img")].every((i) => i.complete),
    );
    await contained(ctx, page, `menu ${entry}`);
    if (entry === "settings") {
      // Settings is a page of the menu's own plate, opened in its place: the
      // main page's edge and width (its height is centred above).
      const settingsBox = await body.boundingBox();
      ctx.check(
        "menu settings opens as a page of the plate, in the main page's place",
        (await body.getByRole("heading", { name: "Settings", level: 2 }).count()) === 1 &&
          settingsBox !== null &&
          Math.abs(settingsBox.x - home.x) < 0.5 &&
          Math.abs(settingsBox.width - home.width) < 0.5,
        JSON.stringify({ settingsBox, home }),
      );
      const choices = await page
        .getByRole("radio")
        .evaluateAll((buttons) => buttons.map((button) => button.getAttribute("aria-label")));
      ctx.check(
        "menu settings exposes exactly the four graphics choices",
        JSON.stringify(choices) === JSON.stringify(["Auto-detect", "Low", "Medium", "High"]),
        JSON.stringify(choices),
      );
      ctx.check(
        "menu settings has one active graphics choice",
        (await page.getByRole("radio", { checked: true }).count()) === 1,
      );
    }
    if (entry === "skirmish")
      await page.locator('[data-testid=lobby-status][data-status="ready"]').waitFor();
    await ctx.matchBaseline(body, `menu-${entry}`, {
      style: entry === "skirmish" ? LOBBY_UNPINNED : MENU_UNPINNED,
    });
    await page.keyboard.press("Escape");
  }
  await page.close();
}
