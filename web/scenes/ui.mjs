// The UI gallery, scripted: every shot the gallery lists, drawn alone at
// 1280×800 and matched against its approved picture, with no panel's
// content outside the panel; the reinforcements picker on its groups and
// with a card's info card open; a squad's transport menu with a ride
// pointed at; the Armory on fixed content; then every
// main-menu page. The panel workbench (`panels.mjs`) pins the info panels and
// the deck, and the cursor lab (`cursor.mjs`) the cursor.
import { pointerAway } from "./_baseline.mjs";
import { spills } from "./_spills.mjs";
import { menuPage } from "../../apps/battle-lab/src/menu/pages.ts";

/** The panels whose content must stay inside them. */
const PANELS = ".hud-panel, .hud-card, .hud-rejected, .frame-rate";

/** Never pinned: the menu's live 3D backdrop (its film, grade and veil, all
 *  of it) and the build's identity. */
const MENU_UNPINNED = ".menu-backdrop, [data-testid=menu-build] { visibility: hidden !important; }";
/** Also never pinned here: the lobby's map, a fresh seed each visit (the
 *  map-picture scene judges the picture). Every mark in it too: a mark the
 *  lobby reveals (your lane's band) would otherwise show through. */
const LOBBY_UNPINNED = `${MENU_UNPINNED} .lobby-picture, .lobby-picture * { visibility: hidden !important; }`;

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

/** The tabs a picture is pinned on: two sibling variants, siblings beside
 *  another family, one card, and none. A baseline names its tab without the
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
  const picker = page.locator(".hud-purchase-picker");
  await tabRow(ctx, page, shot);
  const sizes = [];
  for (const tab of PINNED_TABS) {
    await page.getByRole("tab", { name: tab, exact: true }).click();
    await pointerAway(page);
    await contained(ctx, page, `${shot} ${tab}`);
    sizes.push(
      ...(await picker
        .getByRole("tabpanel")
        .getByRole("button")
        .evaluateAll((cards) =>
          cards.map((c) => [Math.round(c.offsetWidth), Math.round(c.offsetHeight)]),
        )),
    );
    await ctx.matchBaseline(page, `${shot}-${tabFile(tab)}`);
  }
  // A unit for sale is the army's own vertical card: the deck's card size,
  // whether its tab holds one card or many.
  const deckCard = await page
    .locator(".hud-army .hud-army-card")
    .first()
    .evaluate((c) => [Math.round(c.offsetWidth), Math.round(c.offsetHeight)]);
  ctx.check(
    `${shot}: every card is the army's card, at its size`,
    sizes.length > 2 && sizes.every(([w, h]) => w === deckCard[0] && h === deckCard[1]),
    JSON.stringify({ deckCard, sizes }),
  );
  const short = shot === "purchase-short-of-credits";
  const card = (name) => picker.getByRole("button", { name, exact: true });
  // What can't be bought now is dimmed; the rest is not.
  const opacity = (name) => card(name).evaluate((b) => Number(getComputedStyle(b).opacity));
  // A price the player can't pay is the "can't" red that says why, on the
  // card and on its info card alike; one they can pay is the credits gold;
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
  const info = picker.getByRole("tooltip");
  // The info card stands on its card: right on top of it, a few pixels
  // clear, at its left edge, over the picker's own header if it must,
  // naming that card, and alone.
  const standsOn = async (name) => {
    const at = await card(name).boundingBox();
    const named = await info.evaluateAll((cards) => cards.map((c) => c.getAttribute("aria-label")));
    const box = named.length === 1 ? await info.boundingBox() : null;
    const gap = at.y - (box?.y ?? 0) - (box?.height ?? 0);
    return {
      ok: box !== null && named[0] === name && gap >= 0 && gap <= 6 && Math.abs(box.x - at.x) < 0.5,
      detail: JSON.stringify({ named, gap, box, card: at }),
    };
  };

  // Siblings beside another family. Hovering a card shows its one info
  // card, standing on it; the picker itself never changes.
  await page.getByRole("tab", { name: "IFV·APC", exact: true }).click();
  const jeep = card("Jeep Base");
  const before = { picker: await picker.boundingBox(), jeep: await jeep.boundingBox() };
  await card("APC Base").hover();
  let at = await standsOn("APC Base");
  ctx.check(`${shot}: hovering a card stands its one info card on it`, at.ok, at.detail);
  // Straight onto another card: the info card follows at once, no rest.
  await jeep.hover();
  at = await standsOn("Jeep Base");
  ctx.check(`${shot}: moving onto another card moves the info card at once`, at.ok, at.detail);
  const after = { picker: await picker.boundingBox(), jeep: await jeep.boundingBox() };
  ctx.check(
    `${shot}: hovering a card leaves the picker and its card where they were`,
    ["x", "y", "width", "height"].every(
      (k) =>
        Math.abs(after.picker[k] - before.picker[k]) < 0.5 &&
        Math.abs(after.jeep[k] - before.jeep[k]) < 0.5,
    ),
    JSON.stringify({ before, after }),
  );
  // A click that buys nothing (the APC is beyond the credits) keeps the
  // picker open, its info card on the clicked card.
  if (short) {
    // It refuses the purchase (`aria-disabled`), not the pointer.
    await card("APC Base").click({ force: true });
    at = await standsOn("APC Base");
    ctx.check(`${shot}: clicking a card it can't buy keeps its info card on it`, at.ok, at.detail);
  }
  const jeepPrice = {
    opacity: await opacity("Jeep Base"),
    refused: await priced(jeep, "bad"),
    gold: await priced(jeep, "credits"),
  };
  await pointerAway(page);
  ctx.check(`${shot}: leaving the picker puts the info card away`, (await info.count()) === 0);

  // Two siblings, one never sold, its info card open: an opaque plate (a
  // dimmed one dims its words, not what's behind it).
  await page.getByRole("tab", { name: "MBT", exact: true }).click();
  const tank = card("Tank Base");
  const trophy = card("Tank Trophy");
  await tank.hover();
  const tankInfo = { refused: await priced(info, "bad"), gold: await priced(info, "credits") };
  await trophy.hover();
  at = await standsOn("Tank Trophy");
  ctx.check(`${shot}: a card never sold still shows its info card`, at.ok, at.detail);
  const plate = await info.evaluate((c) => getComputedStyle(c).opacity);
  ctx.check(`${shot}: the info card's plate is opaque`, plate === "1", plate);
  const tankOpacity = await opacity("Tank Base");
  ctx.check(
    `${shot}: a card is dimmed exactly when it can't be bought now`,
    tankOpacity < 1 === short && jeepPrice.opacity === 1,
    JSON.stringify({ tank: tankOpacity, jeep: jeepPrice.opacity }),
  );
  {
    const refused = {
      tank: await priced(tank, "bad"),
      info: tankInfo.refused,
      jeep: jeepPrice.refused,
    };
    const gold = { tank: await priced(tank, "credits"), info: tankInfo.gold, jeep: jeepPrice.gold };
    // The trophy's card and, still hovered, its info card.
    const plainly = async (l) => !(await priced(l, "bad")) && !(await priced(l, "credits"));
    const plain = { trophy: await plainly(trophy), info: await plainly(info) };
    ctx.check(
      `${shot}: a price is the "can't" colour exactly when it can't be paid, gold when it can, plain when never sold`,
      refused.tank === short &&
        refused.info === short &&
        !refused.jeep &&
        gold.jeep &&
        gold.tank === !short &&
        gold.info === !short &&
        plain.trophy &&
        plain.info,
      JSON.stringify({ refused, gold, plain }),
    );
  }
  await contained(ctx, page, `${shot} info`);
  await ctx.matchBaseline(page, `${shot}-mbt-info`);
  await pointerAway(page);
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

/** A squad's transport menu with a ride pointed at: the menu stands on the
 *  squad's card, every option is the shop's own card, and the ride's info
 *  stack (its card, the squad's aboard, the total) stands on the whole menu
 *  at the ride's left edge, inside the window. */
async function transport(ctx, page, shot) {
  await page.getByRole("button", { name: "Reinforcements" }).click();
  await page.getByRole("tab", { name: "INF", exact: true }).click();
  const squad = page.getByRole("button", { name: "AT Team Base", exact: true });
  await squad.hover();
  const menu = page.getByRole("region", { name: "Transport" });
  const opened = { info: await page.getByRole("tooltip").count(), menu: await menu.count() };
  ctx.check(
    `${shot}: hovering a squad opens its menu with no info card showing`,
    opened.menu === 1 && opened.info === 0,
    JSON.stringify(opened),
  );
  const ride = menu.getByRole("button", { name: "APC Base", exact: true });
  await ride.hover();
  const m = await page.evaluate(() => {
    const box = (e) => e.getBoundingClientRect();
    const menu = document.querySelector("[aria-label=Transport]");
    const info = [...document.querySelectorAll("[role=tooltip]")].map((t) => box(t));
    const total = box(document.querySelector("[aria-label=Total]"));
    const stack = info.length
      ? { top: info[0].top, left: info[0].left, bottom: total.bottom }
      : null;
    const card = (e) => [Math.round(e.offsetWidth), Math.round(e.offsetHeight)];
    return {
      menu: box(menu),
      squad: box(document.querySelector('.hud-purchase-cards [aria-label="AT Team Base"]')),
      ride: box(menu.querySelector('[aria-label="APC Base"]')),
      stack,
      cards: [...menu.querySelectorAll("button")].map(card),
      deck: card(document.querySelector(".hud-army .hud-army-card")),
    };
  });
  ctx.check(
    `${shot}: the menu stands on its squad's card, a few pixels clear`,
    m.squad.top - m.menu.bottom >= 0 && m.squad.top - m.menu.bottom <= 6,
    JSON.stringify({ menu: m.menu, squad: m.squad }),
  );
  ctx.check(
    `${shot}: every option is the army's card, at its size`,
    m.cards.length > 2 && m.cards.every(([w, h]) => w === m.deck[0] && h === m.deck[1]),
    JSON.stringify({ deck: m.deck, cards: m.cards }),
  );
  ctx.check(
    `${shot}: the ride's info stands on the whole menu at the ride's left edge, inside the window`,
    m.stack !== null &&
      m.menu.top - m.stack.bottom >= 0 &&
      m.menu.top - m.stack.bottom <= 6 &&
      Math.abs(m.stack.left - m.ride.left) < 0.5 &&
      m.stack.top >= 0,
    JSON.stringify({ stack: m.stack, menu: m.menu, ride: m.ride }),
  );
  await contained(ctx, page, shot);
  await ctx.matchBaseline(page, shot);
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
    if (shot === "transport-menu") {
      await transport(ctx, page, shot);
      continue;
    }
    await contained(ctx, page, shot);
    if (shot.startsWith("report")) await report(ctx, page, shot);
    await ctx.matchBaseline(page, shot);
  }

  // The Armory on its fixed content at full HD: a hull's sheet, its sibling
  // variant's tile beside it (the gallery's 1280 shot holds a squad's sheet
  // and a planned tile).
  const wide = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await wide.goto(ctx.urlFor({ shot: "armory", unit: "test_gun_jeep" }));
  await wide.locator('[data-shot="armory"] .armory-sheet').waitFor();
  await wide.evaluate(() => document.fonts.ready);
  await pointerAway(wide);
  await contained(ctx, wide, "armory 1920");
  await ctx.matchBaseline(wide, "armory-1920");
  await wide.close();

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
  for (const entry of ["skirmish", "armory", "settings", "developer"]) {
    await page.locator(`main.menu:not(.loading):not([inert]) [data-page="${entry}"]`).click();
    ctx.check(
      `menu ${entry}: its tile opens it at its own address`,
      new URL(page.url()).pathname === menuPage(entry).path,
      page.url(),
    );
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
  // The smallest window the game supports (web/README.md), and the first
  // width whose strip unfolds.
  for (const [width, height] of [
    [1280, 720],
    [1500, 900],
  ]) {
    const sized = await ctx.newPage({ viewport: { width, height } });
    await lobbyStrip(ctx, sized, width);
    await sized.close();
  }
}

/** The lobby's strip is one row that its panel holds, from the smallest
 *  window the game supports up: below the fold its longest choices are
 *  pickers, and an open picker stays on the panel too. */
async function lobbyStrip(ctx, page, width) {
  await page.goto(`${new URL(ctx.url).origin}/skirmish`);
  await page
    .locator('[data-testid=lobby-status][data-status="ready"]')
    .waitFor({ timeout: 120_000 });
  await page.getByTestId("loading").waitFor({ state: "hidden", timeout: 120_000 });
  await page.evaluate(() => document.fonts.ready);
  const bottoms = await page
    .locator(".lobby-strip > *")
    .evaluateAll((items) => items.map((item) => Math.round(item.getBoundingClientRect().bottom)));
  ctx.check(
    `lobby ${width}: its strip is one row`,
    new Set(bottoms).size === 1,
    JSON.stringify(bottoms),
  );
  await contained(ctx, page, `lobby ${width}`);
  const picker = page.getByTestId("menu-region");
  if (await picker.count()) {
    await picker.click();
    await contained(ctx, page, `lobby ${width}, region picker open`);
    await page.keyboard.press("Escape");
  }
}
