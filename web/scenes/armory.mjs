// The Armory in the real menu, on the game's roster: the tile opens it, every
// pick (a tile, a group, a faction) rewrites `?unit=` in place, with no new
// history entry and no remount, a refresh reopens the same unit, an unknown
// unit opens the first tile, and Back returns to the menu on the Armory's
// tile. Its stage stands one unit of each group on its spot, seated on the
// ground, with its idle loops heard while the Armory is open; the backdrop
// never restarts and the cover never returns. Its approved pictures are the
// UI gallery's, on fixed content (`ui.mjs`); this scene leaves shots of the
// real page, and crops of each unit on its spot, in its evidence. It also
// measures, once, the frame time with both scenes live, each switch's time
// (S7) and the files each selection's art is read from (S5), into
// throwaway/armory/11/.
import { mkdir, writeFile } from "node:fs/promises";
import { menuPage } from "../../apps/battle-lab/src/menu/pages.ts";
import presentation from "../../fixtures/presentation.json" with { type: "json" };
import { decode, writeCrop } from "./_png.mjs";

const MENU = "main.menu:not(.loading):not([inert])";
/** How near the ground a model's lowest point must stand, metres. */
const GROUNDED_M = 0.02;
/** How far a unit's crop reaches past its bounds. */
const CROP_MARGIN = 0.2;
const MEASURES = new URL("../../throwaway/armory/11/", import.meta.url);
/** Radians a drag of one viewport height turns the Armory's orbit. */
const ORBIT_SPEED = presentation.camera_armory.orbit_speed;
/** How long the camera's clearance takes to come to rest, ms. */
const SETTLE_MS = 700;

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
const armoryProbe = (page, fn, arg) => page.evaluate(fn, arg);

/** Frame intervals over `ms` of animation frames: their median and 95th
 *  percentile, milliseconds. */
function frameTimes(page, ms) {
  return page.evaluate(
    (span) =>
      new Promise((resolve) => {
        const gaps = [];
        let last = performance.now();
        const end = last + span;
        const tick = (now) => {
          gaps.push(now - last);
          last = now;
          if (now < end) return requestAnimationFrame(tick);
          gaps.sort((a, b) => a - b);
          const at = (q) => +gaps[Math.floor(q * (gaps.length - 1))].toFixed(2);
          resolve({ frames: gaps.length, p50: at(0.5), p95: at(0.95) });
        };
        requestAnimationFrame(tick);
      }),
    ms,
  );
}

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

  await stageTour(ctx, page, list);
  await cameraTour(ctx, page, list);
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.screenshot({ path: ctx.evidencePath("armory-1920.png") });
  // Each spot at full HD, for the composition against the yard's shots.
  for (const group of ["MBT", "HELI", "REC"]) {
    await list.getByRole("tab", { name: group, exact: true }).click();
    const kind = unit(page);
    await page.waitForFunction((k) => window.__armory?.shown()?.kind === k, kind, {
      timeout: 120_000,
    });
    await page.waitForTimeout(500);
    await page.screenshot({ path: ctx.evidencePath(`armory-1920-${group.toLowerCase()}.png`) });
  }

  // Escape leaves for the menu, on the Armory's tile, and the stage's loops
  // stop with it.
  await page.keyboard.press("Escape");
  await page.locator(`${MENU} [data-page="armory"]`).waitFor();
  ctx.check(
    "Escape returns to the menu on the Armory's tile",
    new URL(page.url()).pathname === "/" &&
      (await page.evaluate(() => document.activeElement?.getAttribute("data-page"))) === "armory",
    page.url(),
  );
  await page.close();
  await roundTrip(ctx, origin);
}

/** One unit of each group of the open faction on its spot: seated on the
 *  ground, its switch timed and the files its art holds counted, and its crop (its
 *  bounds plus a fifth, the panels left out) kept. */
async function stageTour(ctx, page, list) {
  const groups = list
    .getByRole("tablist", { name: "Unit groups" })
    .locator('[role="tab"]:not([disabled])');
  const tours = [];
  for (let g = 0; g < (await groups.count()); g++) {
    const tab = groups.nth(g);
    const group = await tab.getAttribute("aria-label");
    await tab.click();
    // The group's first typed tile: a planned one has nothing to stand.
    const tile = list.locator("button[data-family]:not(.unavailable)").first();
    if ((await tile.count()) === 0) continue;
    if ((await tile.getAttribute("aria-pressed")) !== "true") await tile.click();
    const kind = unit(page);
    // Mid-switch, the stage holds the unit before: never the backdrop.
    await page.screenshot({ path: ctx.evidencePath(`armory-switching-${g}.png`) });
    await page.waitForFunction((k) => window.__armory?.shown()?.kind === k, kind, {
      timeout: 120_000,
    });
    await page.waitForTimeout(500);
    const shown = await armoryProbe(page, () => window.__armory.shown());
    const off = shown.gaps.filter((g) => g.gap === null || Math.abs(g.gap) > GROUNDED_M);
    ctx.check(
      `${group}: ${kind} stands seated on its spot, ${shown.spot}`,
      shown.gaps.length > 0 && off.length === 0,
      JSON.stringify(off),
    );
    const png = decode(await page.screenshot());
    const stage = await armoryProbe(page, () => window.__armory.stage());
    const r = shown.rect ?? stage;
    // The crop stays inside the stage, so neither panel is in it.
    const [x0, x1] = [
      Math.max(stage.x, r.x - r.width * CROP_MARGIN),
      Math.min(stage.x + stage.width, r.x + r.width * (1 + CROP_MARGIN)),
    ];
    const [y0, y1] = [
      Math.max(stage.y, r.y - r.height * CROP_MARGIN),
      Math.min(stage.y + stage.height, r.y + r.height * (1 + CROP_MARGIN)),
    ];
    await writeCrop(
      png,
      ctx.evidencePath(`armory-stage-${group.replace(/\W+/g, "-").toLowerCase()}.png`),
      (x0 + x1) / 2,
      (y0 + y1) / 2,
      (x1 - x0) / 2,
      (y1 - y0) / 2,
      1,
    );
    tours.push({ group, kind, spot: shown.spot, rect: shown.rect, held: shown.held });
  }
  ctx.check("the stage shows a unit of every group", tours.length > 0, `${tours.length} groups`);
  const switches = await armoryProbe(page, () => window.__armory.switches);
  // Measured once and recorded: loose tripwires, never targets.
  await mkdir(MEASURES, { recursive: true });
  await writeFile(new URL("switches.json", MEASURES), JSON.stringify({ switches, tours }, null, 2));
  ctx.check("each switch is timed", switches.length >= tours.length, JSON.stringify(switches));
}

/** Menu → Armory → Back: the stage's loops sound while it is open and stop
 *  after, the backdrop's battle never restarts, the cover never returns,
 *  and the frame time with both scenes live is measured. */
async function roundTrip(ctx, origin) {
  const page = await ctx.newPage({ viewport: { width: 1280, height: 800 } });
  await page.addInitScript(seedGraphics);
  // Whether the loading cover is ever drawn after the menu first shows.
  await page.addInitScript(() => {
    window.__coverReturned = false;
    new MutationObserver(() => {
      if (window.__menuShown && document.querySelector('[data-testid="loading"]'))
        window.__coverReturned = true;
    }).observe(document, { childList: true, subtree: true });
  });
  await openMenu(page, `${origin}/`);
  await page.evaluate(() => (window.__menuShown = true));
  const backdrop = page.workers();
  const alone = await frameTimes(page, 3000);
  await page.locator(`${MENU} [data-page="armory"]`).click();
  // A hull with an engine: the first typed tile of the first group whose
  // first unit idles with loops.
  const list = page.getByRole("region", { name: "Units" });
  await list.waitFor();
  await page.waitForFunction(() => window.__armory?.shown(), undefined, { timeout: 120_000 });
  const groups = list
    .getByRole("tablist", { name: "Unit groups" })
    .locator('[role="tab"]:not([disabled])');
  let voices = 0;
  for (let g = 0; g < (await groups.count()) && voices === 0; g++) {
    await groups.nth(g).click();
    const tile = list.locator("button[data-family]:not(.unavailable)").first();
    if ((await tile.getAttribute("aria-pressed")) !== "true") await tile.click();
    const kind = unit(page);
    await page.waitForFunction((k) => window.__armory?.shown()?.kind === k, kind, {
      timeout: 120_000,
    });
    voices = await page
      .waitForFunction(() => window.__armory.voices() > 0 && window.__armory.voices(), undefined, {
        timeout: 15_000,
      })
      .then((h) => h.jsonValue())
      .catch(() => 0);
  }
  ctx.check("the stage's idle loops sound on /armory", voices > 0, `${voices} voices`);
  const both = await frameTimes(page, 3000);
  await page.screenshot({ path: ctx.evidencePath("armory-stage-1280.png") });
  await page.getByRole("button", { name: "Back" }).click();
  await page.locator(`${MENU} [data-page="armory"]`).waitFor();
  const after = await page
    .waitForFunction(() => window.__armory.voices() === 0, undefined, { timeout: 5_000 })
    .then(() => 0)
    .catch(() => page.evaluate(() => window.__armory.voices()));
  ctx.check("its loops stop after Back", after === 0, `${after} voices`);
  const workers = page.workers();
  ctx.check(
    "the backdrop's battle never restarts and the cover never returns",
    backdrop.length === 1 &&
      workers.length === 1 &&
      workers[0] === backdrop[0] &&
      !(await page.evaluate(() => window.__coverReturned)),
    JSON.stringify({ before: backdrop.length, after: workers.length }),
  );
  await mkdir(MEASURES, { recursive: true });
  await writeFile(new URL("frame-time.json", MEASURES), JSON.stringify({ alone, both }, null, 2));
  ctx.check(
    "frame time is measured with both scenes live",
    both.frames > 0,
    JSON.stringify({ alone, both }),
  );
  await page.close();
}

/** Whether `p` stands inside `box` (an obstacle box: centre, yaw, half
 *  extents, base height). */
function insideBox(p, box) {
  const [dx, dy] = [p[0] - box.center[0], p[1] - box.center[1]];
  const [c, s] = [Math.cos(box.yaw), Math.sin(box.yaw)];
  const [lx, ly] = [dx * c + dy * s, dy * c - dx * s];
  return (
    Math.abs(lx) <= box.half[0] &&
    Math.abs(ly) <= box.half[1] &&
    p[2] >= box.baseZ &&
    p[2] <= box.baseZ + 2 * box.half[2]
  );
}

/** For one unit on each spot, drag and wheel to every extreme (8 yaws, 3
 *  pitches, the nearest and farthest zoom): the eye is never inside the
 *  unit's bounds or a prop's box and never below the ground. Keys, the
 *  screen's edges and drags begun on the list or the sheet never move it.
 *  The four extremes of the helicopter are kept as crops of the cell. */
async function cameraTour(ctx, page, list) {
  const viewport = page.viewportSize();
  const stage = await armoryProbe(page, () => window.__armory.stage());
  const centre = [stage.x + stage.width / 2, stage.y + stage.height / 2];
  const camera = () => armoryProbe(page, () => window.__armory.camera());
  const drag = async (dx, dy) => {
    await page.mouse.move(centre[0], centre[1]);
    await page.mouse.down();
    await page.mouse.move(centre[0] + dx, centre[1] + dy, { steps: 8 });
    await page.mouse.up();
  };
  const wheel = async (dy) => {
    await page.mouse.move(centre[0], centre[1]);
    for (let k = 0; k < 6; k++) await page.mouse.wheel(0, dy);
  };
  const settled = () => page.waitForTimeout(SETTLE_MS);
  /** One viewport height of drag, in pixels. */
  const height = viewport.height;
  const crop = async (name) => {
    const png = decode(await page.screenshot());
    await writeCrop(
      png,
      ctx.evidencePath(`armory-camera-${name}.png`),
      centre[0],
      centre[1],
      stage.width / 2,
      stage.height / 2,
      1,
    );
  };
  for (const group of ["REC", "MBT", "HELI"]) {
    await list.getByRole("tab", { name: group, exact: true }).click();
    const kind = unit(page);
    await page.waitForFunction((k) => window.__armory?.shown()?.kind === k, kind, {
      timeout: 120_000,
    });
    await settled();
    const misses = [];
    let poses = 0;
    for (let yaw = 0; yaw < 8; yaw++) {
      await drag((Math.PI / 4 / ORBIT_SPEED) * height, 0);
      for (const tilt of [-1.5, 0.3, 1.5]) {
        await drag(0, tilt * height);
        for (const zoom of [-2000, 2000]) {
          await wheel(zoom);
          await settled();
          const c = await camera();
          poses++;
          const bad =
            c.eye[2] <= 0 ||
            (c.bounds &&
              insideBox(c.eye, {
                center: [
                  (c.bounds.min[0] + c.bounds.max[0]) / 2,
                  (c.bounds.min[1] + c.bounds.max[1]) / 2,
                ],
                yaw: 0,
                half: [0, 1, 2].map((i) => (c.bounds.max[i] - c.bounds.min[i]) / 2),
                baseZ: c.bounds.min[2],
              })) ||
            c.boxes.some((b) => insideBox(c.eye, b));
          if (bad) misses.push({ yaw, tilt, zoom, eye: c.eye });
        }
      }
    }
    ctx.check(
      `${group}: at every extreme the eye is clear of ${kind}, the props and the ground`,
      poses === 48 && misses.length === 0,
      JSON.stringify({ poses, misses: misses.slice(0, 4) }),
    );
  }

  // The helicopter's four extremes, for the eye.
  await wheel(-2000);
  await drag(0, -1.5 * height);
  await settled();
  await crop("closest-lowest");
  await drag(0, 1.5 * height);
  await settled();
  await crop("closest-highest");
  await wheel(2000);
  await settled();
  await crop("farthest-highest");
  await drag(0, -1.5 * height);
  await settled();
  await crop("farthest-lowest");
  const low = await camera();
  ctx.check(
    "the lowest pitch dips below the horizon",
    low.orbit.pitch < 0,
    JSON.stringify({ pitch: low.orbit.pitch, distance: low.orbit.distance, bounds: low.bounds }),
  );

  // Nothing but a drag or the wheel on the stage moves it.
  const before = JSON.stringify((await camera()).orbit);
  for (const key of [
    "KeyW",
    "KeyA",
    "KeyS",
    "KeyD",
    "ArrowUp",
    "ArrowLeft",
    "KeyQ",
    "KeyE",
    "KeyZ",
    "KeyX",
  ])
    await page.keyboard.press(key);
  for (const [x, y] of [
    [1, viewport.height / 2],
    [viewport.width - 2, viewport.height / 2],
    [viewport.width / 2, 1],
    [viewport.width / 2, viewport.height - 2],
  ]) {
    await page.mouse.move(x, y);
    await page.waitForTimeout(150);
  }
  for (const panel of [list, page.locator(".armory-sheet")]) {
    const box = await panel.boundingBox();
    const [x, y] = [box.x + box.width / 2, box.y + box.height - 20];
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x + 200, y - 150, { steps: 8 });
    await page.mouse.up();
    await page.mouse.move(x, y);
    await page.mouse.wheel(0, 600);
  }
  await settled();
  const after = JSON.stringify((await camera()).orbit);
  ctx.check(
    "keys, the screen's edges and drags on the list or the sheet never move the camera",
    after === before,
    `${before} → ${after}`,
  );
}
