// Slice 15: the village battle. Blue plays through the production controls;
// the encounter status, variant, seed, pause/reset and replay export work.
// Battle-look slice 09: the camera tour, from the opening framing out to the
// strategic height and in to the ground, through the real wheel.
import { readFile } from "node:fs/promises";
import { lab, obs, advance, until, snapshot } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";
import { checkOverlayIsolation } from "./_overlays.mjs";

const village = JSON.parse(
  await readFile(new URL("../../fixtures/village.json", import.meta.url), "utf8"),
);
const CAMERA = village.presentation.camera;
/** The tour's fixed tick: every framing shows the same battle state. */
const TOUR_TICK = 90;

const text = (page, id) => page.getByTestId(id).innerText();

async function shot(ctx, page, name) {
  await snapshot(ctx, page, `frame-${name}.png`);
}

/** Battle-look slice 16: the road is drawn where the simulation has it. Top
 *  down over the first road's straight run, ground a metre inside its edge
 *  reads as road and ground a metre and a half outside reads as verge. */
async function checkRoadEdges(ctx, page) {
  const road = village.map.roads[0];
  const [[ax, ay], [bx, by]] = road.points;
  const half = road.width_m / 2;
  const [mx, my] = [(ax + bx) / 2, (ay + by) / 2];
  const len = Math.hypot(bx - ax, by - ay);
  const [nx, ny] = [-(by - ay) / len, (bx - ax) / len];
  const at = (off) => [mx + nx * off, my + ny * off];
  await lab(page, (c) => window.__lab.setCamera({ ...window.__lab.camera(), ...c }), {
    target: [mx, my, 0],
    distance: 60,
    pitch: 1.5,
  });
  const shot = decode(await snapshot(ctx, page, "road-edges-1920x1080.png"));
  const greenness = async (off) => {
    const [x, y] = at(off);
    const css = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], 0), [x, y]);
    const [r, g, b] = pixel(shot, css[0], css[1]);
    return g - (r + b) / 2;
  };
  const centre = await greenness(0);
  const inside = [await greenness(half - 1), await greenness(-(half - 1))];
  const outside = [await greenness(half + 1.5), await greenness(-(half + 1.5))];
  ctx.check(
    "the road is drawn where the simulation has it: road inside its edge, verge outside",
    inside.every((g) => g < centre + 6) && outside.every((g) => g > centre + 12),
    JSON.stringify({ centre, inside, outside }),
  );
}

/** Near, default and far at 1920×1080: the framings the reference crops judge. */
async function tour(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, TOUR_TICK - (await lab(page, () => window.__lab.route.tick())));
  const camera = () => lab(page, () => window.__lab.camera());
  const tourShot = (name) => snapshot(ctx, page, `tour-${name}-1920x1080.png`);
  // Wheel over the battlefield, clear of the panel, until the zoom limit.
  const wheel = async (dy) => {
    await page.mouse.move(1300, 540);
    for (let k = 0; k < 20; k++) await page.mouse.wheel(0, dy);
  };
  const [lowest, highest] = [CAMERA.pitch_curve[0], CAMERA.pitch_curve.at(-1)];

  const opening = await camera();
  ctx.check(
    "the battle opens at the fixture's default framing",
    opening.distance === CAMERA.default.distance &&
      opening.target[0] === CAMERA.default.target[0] &&
      opening.target[1] === CAMERA.default.target[1],
    JSON.stringify(opening),
  );
  await tourShot("default");
  // Rings, zone and orders are overlays: exactly their own colours over the
  // finished, fogged and graded world.
  const isolation = await checkOverlayIsolation(ctx, page, "overlay-default");
  ctx.check(
    "overlays keep their own colours over the finished frame",
    isolation.isolated && isolation.opaque > 0,
    JSON.stringify(isolation),
  );

  await wheel(400);
  const far = await camera();
  ctx.check(
    "wheeling out stops at the strategic height, pitched by the curve",
    far.distance === CAMERA.zoom_max &&
      highest[0] === CAMERA.zoom_max &&
      Math.abs(far.pitch - highest[1]) < 1e-9,
    JSON.stringify(far),
  );
  await tourShot("strategic");
  await checkRoadEdges(ctx, page);

  await lab(page, () => window.__lab.reset());
  await wheel(-400);
  const near = await camera();
  ctx.check(
    "wheeling in stops at ground level, pitched by the curve, the target on the ground",
    near.distance === CAMERA.zoom_min &&
      lowest[0] === CAMERA.zoom_min &&
      Math.abs(near.pitch - lowest[1]) < 1e-9 &&
      near.target[2] ===
        (await lab(page, (t) => window.__lab.route.surfaceZ(t[0], t[1]), near.target)),
    JSON.stringify(near),
  );
  await tourShot("ground");
  await page.close();
}

export async function run(ctx) {
  await tour(ctx);
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await shot(ctx, page, "start-1280x800");
  ctx.check(
    "the status names the variant and seed",
    /Ordinary ambush · seed \d+/.test(await text(page, "status")),
    await text(page, "status"),
  );
  ctx.check(
    "the encounter status is published and running",
    /^Hold the village: 0\/\d+ s held · in progress$/.test(await text(page, "encounter")),
    await text(page, "encounter"),
  );

  // Select the tanks and right-click the ground: an accepted move.
  const o = await obs(page);
  const tanks = o.own.filter((u) => u.kind === "tank").map((u) => u.id);
  await lab(page, (ids) => window.__lab.route.select(ids), tanks);
  await page.waitForFunction((n) => window.__lab.route.selected().length === n, tanks.length);
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

  // Each selected tank and its destination ring carry the panel's name.
  const tags = await lab(page, () => ({
    names: [...document.querySelectorAll(".ro-unit.ro-selected .ro-name")].map((n) => ({
      unit: Number(n.parentElement.dataset.unit),
      text: n.textContent,
    })),
    goals: [...document.querySelectorAll(".ro-goal")].map((n) => {
      const r = n.getBoundingClientRect();
      return {
        unit: Number(n.dataset.goal),
        text: n.textContent,
        shown: n.style.display !== "none",
        at: [r.x + r.width / 2, r.bottom],
      };
    }),
    panel: [...document.querySelectorAll("[data-testid=selection-panel] [data-unit] strong")].map(
      (n) => n.textContent,
    ),
  }));
  const now = await obs(page);
  const goalPx = await Promise.all(
    tanks.map((id) => {
      const g = now.own.find((u) => u.id === id).goal;
      return g ? lab(page, (p) => window.__lab.projectToCss(p[0], p[1], 0), g) : null;
    }),
  );
  ctx.check(
    "each selected tank and its destination ring show the panel's name",
    tanks.every((id, k) => {
      const name = tags.names.find((n) => n.unit === id)?.text;
      const goal = tags.goals.find((g) => g.unit === id);
      // A tank still under way has a tag just above its destination ring.
      const atRing =
        !goalPx[k] ||
        (goal?.text === name &&
          goal.shown &&
          Math.abs(goal.at[0] - goalPx[k][0]) < 40 &&
          goal.at[1] < goalPx[k][1] &&
          goalPx[k][1] - goal.at[1] < 40);
      return !!name && tags.panel.includes(name) && atRing;
    }),
    JSON.stringify({ tags, goalPx }),
  );

  // Zoomed out, where the two tanks' clusters and destination names would
  // pile up, none sits under the panel and none overprints another.
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [300, 800, 0], distance: 1150 }),
  );
  await shot(ctx, page, "tanks-far-1280x800");
  const placed = await lab(page, () => {
    const box = (e) => {
      const r = e.getBoundingClientRect();
      return { x0: r.left, x1: r.right, y0: r.top, y1: r.bottom };
    };
    const shown = (sel) =>
      [...document.querySelectorAll(sel)].filter((e) => e.style.display !== "none").map(box);
    return {
      panel: box(document.querySelector("[data-occludes-readouts]")),
      readouts: shown(".ro-unit"),
      goals: shown(".ro-goal"),
    };
  });
  const overlap = (a, b) => a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1;
  ctx.check(
    "no readout or name sits under the panel or overprints another",
    placed.readouts.length > 0 &&
      placed.goals.length === tanks.length &&
      [...placed.readouts, ...placed.goals].every(
        (b, k, all) => !overlap(b, placed.panel) && all.every((c, j) => j === k || !overlap(b, c)),
      ),
    JSON.stringify(placed),
  );

  // Sound: at a tick where blue hears something, the newest caption says
  // what, how far and from where, for the unit that heard it.
  const heard = await until(page, (o) => o.audible.length > 0, 30 * 120, 1);
  const cue = heard?.audible.at(-1);
  const listener = cue && heard.own.find((u) => u.id === cue.listener);
  const caption = heard && (await text(page, "captions")).split("\n")[0];
  ctx.check(
    "when blue hears something, a caption names the sound, its range and the listener",
    !!listener &&
      caption.startsWith("Heard ") &&
      caption.includes(`, ${cue.band}, `) &&
      caption.includes(` of ${listener.kind} #${listener.id}`),
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

  // Export: the file names its variant and carries the accepted commands.
  const file = await lab(page, () => window.__lab.route.exportReplay());
  ctx.check(
    "the replay export records its variant and commands",
    file?.variant === "ordinary" && JSON.parse(file.replay).accepted?.length >= 1,
    file && file.replay.slice(0, 120),
  );

  // Pause holds the tick; resume continues.
  const held = await lab(page, () => window.__lab.route.tick());
  await page.waitForTimeout(400);
  ctx.check(
    "paused, the battle does not advance",
    (await lab(page, () => window.__lab.route.tick())) === held,
  );

  // Reset starts again from the seed with an empty log.
  await page.getByRole("button", { name: "Reset" }).click();
  await page.waitForFunction(
    () => window.__lab.route.acks().length === 0 && window.__lab.route.tick() < 60,
  );
  ctx.check("reset rebuilds from the seed", true);

  // The seed and the variant are the player's to change.
  await page.getByLabel("Seed").fill("7");
  await page.waitForFunction(
    () => /seed 7 /.test(document.querySelector("[data-testid=status]")?.textContent ?? ""),
    undefined,
    { timeout: 30000 },
  );
  ctx.check("a new seed restarts the battle on that seed", true);
  await page.getByLabel("Variant").selectOption("prepared_crossfire");
  await page.waitForFunction(
    () =>
      /Prepared crossfire/.test(document.querySelector("[data-testid=status]")?.textContent ?? ""),
    undefined,
    { timeout: 30000 },
  );
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  ctx.check("the variant select loads the crossfire battle", true);
}
