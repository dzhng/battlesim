// Player commands, the pause menu and captions on the street battle.
import {
  lab,
  obs,
  advance,
  until,
  snapshot,
  openBattle,
  openMenu,
  closeMenu,
  restart,
  pauseMenuText,
  pointerOffCanvas,
} from "./_lab.mjs";
import { unitType } from "./_units.mjs";
const text = (page, id) => page.getByTestId(id).innerText();
const shot = (ctx, page, name) => snapshot(ctx, page, `frame-${name}.png`);

/** Emoji, pictographs and the symbol blocks a placeholder icon is drawn
 *  from (arrows, maths, technical, shapes, dingbats): the HUD's icons are
 *  the generated set, never a font's glyph. */
const GLYPH_ICON =
  /[\p{Extended_Pictographic}\u2190-\u21ff\u2200-\u23ff\u25a0-\u27bf\u2900-\u2bff]/gu;

/** The HUD's text as the player sees it holds no glyph icon. */
async function checkNoGlyphIcons(ctx, page, where) {
  const found = (await page.evaluate(() => document.body.innerText)).match(GLYPH_ICON) ?? [];
  ctx.check(
    `the HUD draws no emoji or symbol glyph as an icon (${where})`,
    found.length === 0,
    [...new Set(found)].join(" "),
  );
}

export async function playTour(ctx) {
  const page = await openBattle(ctx);
  await shot(ctx, page, "start-1280x800");
  const opened = await pauseMenuText(page);
  ctx.check("the pause menu shows no seed", !/seed|\d{3,}/i.test(opened), opened);
  ctx.check(
    "the top bar holds the clock alone",
    /^\d+:\d\d$/.test(await text(page, "clock")) &&
      (await page.locator("header.hud-top").innerText()).split("\n").length === 1,
    await page.locator("header.hud-top").innerText(),
  );
  // Esc opens the pause menu over a running battle and pauses it; Esc again
  // closes it and the battle runs on. An armed command takes Esc first.
  const armedUnit = (await obs(page)).own.find((u) => u.kind === "test_tank").id;
  await lab(page, (id) => window.__lab.route.select([id]), armedUnit);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  await lab(page, () => window.__lab.route.resume());
  await page.waitForFunction(() => window.__lab.route.status().status === "running");
  await page.keyboard.press("c");
  await page.keyboard.press("Escape");
  const disarmed = (await page.getByRole("dialog", { name: "Paused" }).count()) === 0;
  await page.keyboard.press("Escape");
  await page.getByRole("dialog", { name: "Paused" }).waitFor();
  await page.waitForFunction(() => window.__lab.route.status().status === "paused");
  const pausedTick = await lab(page, () => window.__lab.route.tick());
  await page.waitForTimeout(300);
  const heldInMenu = (await lab(page, () => window.__lab.route.tick())) === pausedTick;
  await page.keyboard.press("Escape");
  await page.waitForFunction((t) => window.__lab.route.tick() > t + 2, pausedTick, {
    timeout: 5000,
  });
  ctx.check(
    "Esc disarms an armed command first, then opens the pause menu, which holds the battle until closed",
    disarmed && heldInMenu,
    JSON.stringify({ disarmed, heldInMenu }),
  );
  await lab(page, () => window.__lab.route.pause());

  // Select the tanks and right-click the ground: an accepted move.
  const o = await obs(page);
  const tanks = o.own.filter((u) => u.kind === "test_tank").map((u) => u.id);
  await lab(page, (ids) => window.__lab.route.select(ids), tanks);
  await page.waitForFunction((n) => window.__lab.route.selected().length === n, tanks.length);
  await checkNoGlyphIcons(ctx, page, "tanks selected");
  await openMenu(page);
  await checkNoGlyphIcons(ctx, page, "pause menu");
  await closeMenu(page);
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [360, 800, 0], distance: 600 }),
  );
  const spot = await lab(page, () => window.__lab.projectToCss(420, 800, 0));
  await page.mouse.click(spot[0], spot[1], { button: "right" });
  await page.waitForFunction(() => window.__lab.route.acks().length > 0, undefined, {
    timeout: 5000,
  });
  const ack = (await lab(page, () => window.__lab.route.acks()))[0];
  ctx.check(
    "right-clicking the ground moves the selected tanks",
    /move/.test(ack.label) && ack.ack.error === null,
    JSON.stringify(ack),
  );
  await advance(page, 30 * 20);
  const moved = await obs(page);
  await shot(ctx, page, "advance-1280x800");
  ctx.check(
    "the tanks drove toward the order",
    moved.own
      .filter((u) => tanks.includes(u.id))
      .every((u) => Math.hypot(u.position[0] - 420, u.position[1] - 800) < 150),
    JSON.stringify(moved.own.filter((u) => tanks.includes(u.id)).map((u) => u.position)),
  );
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [420, 800, 0], distance: 350 }),
  );
  await shot(ctx, page, "tanks-close-1280x800");

  // Each selected tank's callout carries the unit card's name; a
  // destination carries no text (its marker and route say
  // whose it is).
  const tags = await lab(page, () => ({
    names: [...document.querySelectorAll(".ro-unit.ro-selected .ro-name-word")].map((n) => ({
      unit: Number(n.closest(".ro-unit").dataset.unit),
      text: n.textContent.trim(),
    })),
    panel: [...document.querySelectorAll('.hud-army-card[aria-pressed="true"]')].map((n) =>
      n
        .getAttribute("aria-label")
        .replace(/ #\d+$/, "")
        .toUpperCase(),
    ),
    extraText: (() => {
      const layer = document.querySelector("[data-testid=readouts]").cloneNode(true);
      layer.querySelectorAll(".ro-unit").forEach((panel) => panel.remove());
      return layer.textContent.trim();
    })(),
  }));
  ctx.check(
    "each selected tank's callout shows the unit card's name, and no destination shows text",
    tags.extraText === "" &&
      tanks.every((id) => {
        const name = tags.names.find((n) => n.unit === id)?.text;
        return !!name && tags.panel.includes(name);
      }),
    JSON.stringify(tags),
  );

  // Far views keep compact cards and allow depth-ordered overlap, while
  // keeping the HUD clear.
  await pointerOffCanvas(page);
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [300, 800, 0], distance: 1150 }),
  );
  await shot(ctx, page, "tanks-far-1280x800");
  const placed = await lab(page, () => {
    const box = (e) => {
      const r = e.getBoundingClientRect();
      return { x0: r.left, x1: r.right, y0: r.top, y1: r.bottom };
    };
    return {
      panels: [...document.querySelectorAll("[data-occludes-readouts]")].map(box),
      readouts: [...document.querySelectorAll(".ro-unit")]
        .filter((e) => e.style.display !== "none")
        .map((e) => ({ ...box(e), zoom: e.dataset.zoom })),
    };
  });
  const overlap = (a, b) => a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1;
  ctx.check(
    "far readouts remain compact and clear of the HUD's bars",
    placed.readouts.length > 0 &&
      placed.readouts.every(
        (b) => b.zoom === "compressed" && placed.panels.every((p) => !overlap(b, p)),
      ),
    JSON.stringify(placed),
  );

  // Sound: at a tick where blue hears something, the newest caption says
  // what, how far and from where, for the unit that heard it.
  // Nobody fires until blue closes to weapon range, so send it into contact.
  await lab(page, () =>
    window.__lab.route.command({
      kind: "attack_move",
      units: window.__lab.route.observation().own.map((u) => u.id),
      gesture: 9,
      goal: [700, 800],
    }),
  );
  await advance(page, 1);
  await page.waitForFunction(
    () => window.__lab.route.acks().some((entry) => entry.order.gesture === 9),
    undefined,
    { timeout: 5000 },
  );
  const approach = (await lab(page, () => window.__lab.route.acks())).find(
    (entry) => entry.order.gesture === 9,
  ).ack;
  const admitted =
    approach && !approach.error && approach.placement?.destinations.some((d) => d.placed);
  ctx.check("the sound approach admits observers", !!admitted, JSON.stringify(approach));
  await ctx.writeEvidence("play-contact-approach.json", approach ?? null);
  if (!admitted) return page.close();
  const heard = await until(page, (o) => o.audible.length > 0, 30 * 240, 1);
  const cue = heard?.audible.at(-1);
  const listener = cue && heard.own.find((u) => u.id === cue.listener);
  const caption = heard && (await text(page, "captions")).split("\n")[0];
  ctx.check(
    "when blue hears something, a caption names the sound, its range and the listener",
    !!listener &&
      caption.startsWith("Heard ") &&
      caption.includes(`, ${cue.band}, `) &&
      caption.includes(` of ${unitType(listener.kind).name}`),
    heard ? `tick ${heard.tick}: ${JSON.stringify(cue)} → ${caption}` : "never heard",
  );
  // Twenty seconds of fire later, repeats have collapsed into a few rows.
  await advance(page, 30 * 20);
  await page.evaluate(() => window.__lab.frame());
  const rows = await lab(page, () =>
    [...document.querySelectorAll("[data-testid=captions] li[data-count]")].map((li) => ({
      text: li.textContent.replace(/ ×\d+$/, ""),
      count: Number(li.dataset.count),
    })),
  );
  ctx.check(
    "repeated sounds collapse into at most three counted rows",
    rows.length <= 3 && new Set(rows.map((r) => r.text)).size === rows.length,
    JSON.stringify(rows),
  );

  // Pause holds the tick; resume continues.
  const held = await lab(page, () => window.__lab.route.tick());
  await page.waitForTimeout(400);
  ctx.check(
    "paused, the battle does not advance",
    (await lab(page, () => window.__lab.route.tick())) === held,
  );

  // Restart restarts the clock with an empty command log.
  await restart(page);
  await page.waitForFunction(() => {
    const route = window.__lab?.route;
    return !!route && route.acks().length === 0 && route.tick() < 60;
  });
  ctx.check("restart empties the command log and restarts the clock", true);
}
