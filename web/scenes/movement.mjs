// Slice 04: predictable routes, group intent, gestures, traffic and blockage.
import { movementCursor } from "./_cursorOrders.mjs";
import { lab, groundCss, presented } from "./_lab.mjs";
import { paintOnly } from "./_overlays.mjs";
import { hull, visuals } from "./_units.mjs";

const unit = (page, id) =>
  lab(page, (i) => window.__lab.route.observation().own.find((u) => u.id === i), id);
/** Advance until `id` has its route, or knows it has none: a unit holds,
 *  planning, for as many ticks as its route takes to work out. */
async function planned(page, id) {
  for (let i = 0; i < 200; i++) {
    await lab(page, () => window.__lab.route.advance(5));
    const u = await unit(page, id);
    if (u.state !== "planning") return u;
  }
  throw new Error(`unit ${id} never finished planning`);
}
const until = async (page, predicate, arg, timeout = 20000) =>
  page.waitForFunction(predicate, arg, { timeout, polling: 100 });

export async function run(ctx) {
  await movementCursor(ctx);
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await until(page, () => window.__lab.route?.tick() > 3);

  // Real pointer gestures: box select, right-click, double right-click, Shift queue, S stop.
  await lab(page, () => window.__lab.route.resume());
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [60, 205, 0], distance: 120 }),
  );
  await page.evaluate(() => window.__lab.frame());
  const recon = await lab(page, () =>
    window.__lab.route.observation().own.find((u) => u.kind === "test_recon"),
  );
  const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), recon.position);
  await page.mouse.move(at[0] - 30, at[1] - 30);
  await page.mouse.down();
  await page.mouse.move(at[0] + 30, at[1] + 30, { steps: 5 });
  await page.mouse.up();
  const selected = await lab(page, () => window.__lab.route.selected());
  ctx.check(
    "drag box selects the unit under it",
    selected.includes(recon.id),
    JSON.stringify(selected),
  );
  await lab(page, (id) => window.__lab.route.select([id]), recon.id);
  const goal = await lab(page, () => window.__lab.projectToCss(95, 195, 0));
  await page.mouse.click(goal[0], goal[1], { button: "right" });
  await page.mouse.click(goal[0] + 2, goal[1] + 1, { button: "right" });
  await until(
    page,
    (id) => window.__lab.route.observation().own.find((u) => u.id === id).policy === "fastest",
    recon.id,
    5000,
  );
  const log = await page.getByTestId("ack-log").textContent();
  ctx.check(
    "double right-click upgrades the same gesture to the fast route",
    /upgrade gesture \d+ to fast route/.test(log) &&
      (await unit(page, recon.id)).queue.length === 0,
    log,
  );
  const later = await lab(page, () => window.__lab.projectToCss(95, 230, 0));
  await page.keyboard.down("Shift");
  await page.mouse.click(later[0], later[1], { button: "right" });
  await page.keyboard.up("Shift");
  await until(
    page,
    (id) => window.__lab.route.observation().own.find((u) => u.id === id).queue.length === 1,
    recon.id,
    5000,
  );
  ctx.check("Shift right-click queues a waypoint", true);
  await page.keyboard.press("Backspace");
  await until(
    page,
    (id) => window.__lab.route.observation().own.find((u) => u.id === id).state === "idle",
    recon.id,
    5000,
  );
  const stopped = await unit(page, recon.id);
  ctx.check(
    "Backspace stops and clears the queue",
    stopped.queue.length === 0 && stopped.goal === null,
    JSON.stringify(stopped.state),
  );
  await page.close();
  await paintOnDeckAndWater(ctx);
}

/** Ground paint shows on every surface movers stand on or cross (post-close
 *  review, 5): a selected truck has its queued intention line painted
 *  across the river, mid-stream, and a selected
 *  tank parked on the bridge deck has its marker ring painted on the deck.
 *  Both once vanished: the water and the deck didn't read the paint. Read
 *  as `paintOnly`, along the line over deep water and round the ring. */
async function paintOnDeckAndWater(ctx) {
  const DECK = [390, 220];
  const BEFORE_RIVER = [360, 240];
  const AFTER_RIVER = [430, 240];
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await until(page, () => window.__lab.route?.tick() > 3);
  await lab(page, () => window.__lab.route.pause());
  const own = await lab(page, () => window.__lab.route.observation().own);
  const tank = own.find((u) => u.kind === "test_tank");
  const truck = own.find((u) => u.kind === "test_supply");
  await lab(page, (ids) => window.__lab.route.select(ids), [tank.id, truck.id]);
  await page.waitForFunction(() => window.__lab.route.selected().length === 2);
  const move = (id, goal, gesture, queued = false) =>
    lab(
      page,
      (o) =>
        window.__lab.route.command(
          {
            kind: "move",
            units: [o.id],
            gesture: o.gesture,
            goal: o.goal,
            route: "fastest",
          },
          o.queued,
        ),
      { id, goal, gesture, queued },
    );
  // The paint alone, framed on `at`.
  const paintAt = async (at, name) => {
    await lab(
      page,
      (t) =>
        window.__lab.setCamera({ ...window.__lab.camera(), target: t, distance: 60, pitch: 0.9 }),
      [at[0], at[1], 0],
    );
    await page.evaluate(() => window.__lab.frame());
    await page.evaluate(() => window.__lab.frame());
    return paintOnly(ctx, page, name);
  };
  // How much of `points` the paint shows (the brightest rise within a pixel
  // of each), and how bright it is where it shows (the mean of those rises).
  const inkOf = async (paint, points) => {
    let inked = 0;
    let sum = 0;
    for (const q of points) {
      const p = await groundCss(page, q);
      let best = 0;
      for (let dy = -1; dy <= 1; dy++)
        for (let dx = -1; dx <= 1; dx++) {
          const i = ((Math.round(p[1]) + dy) * paint.width + Math.round(p[0]) + dx) * 4;
          best = Math.max(best, paint.data[i] + paint.data[i + 1] + paint.data[i + 2]);
        }
      if (best > 60) {
        inked++;
        sum += best;
      }
    }
    return { share: inked / points.length, rise: inked ? sum / inked : 0 };
  };
  const shown = async (paint, points) => (await inkOf(paint, points)).share;
  const ring = (c, radius) =>
    Array.from({ length: 48 }, (_, k) => [
      c[0] + Math.cos((k / 48) * 2 * Math.PI) * radius,
      c[1] + Math.sin((k / 48) * 2 * Math.PI) * radius,
    ]);

  // Two reachable destinations, joined by the queued intention's dashed
  // line. The actual drive crosses the bridge; the queue line crosses water.
  const firstAck = await move(truck.id, BEFORE_RIVER, 81);
  await planned(page, truck.id);
  const queuedAck = await move(truck.id, AFTER_RIVER, 82, true);
  ctx.check(
    "both river-side destinations are admitted before drawing their queue line",
    !firstAck.error &&
      !queuedAck.error &&
      firstAck.placement.destinations.every((mark) => mark.placed) &&
      queuedAck.placement.destinations.every((mark) => mark.placed),
    JSON.stringify({ firstAck, queuedAck }),
  );
  await lab(
    page,
    (tick) => window.__lab.route.advance(Math.max(0, tick - window.__lab.route.tick())),
    queuedAck.applied_tick,
  );
  await presented(page);
  const queuedTruck = await unit(page, truck.id);
  const onLine = (x0, x1) => {
    const out = [];
    for (let x = x0; x <= x1; x += 0.5) out.push([x, BEFORE_RIVER[1]]);
    return out;
  };
  const across = onLine(384, 396);
  const paint = await paintAt(across[across.length >> 1], "water");
  const [over, before] = [await inkOf(paint, across), await inkOf(paint, onLine(368, 378))];
  const water = { queue: queuedTruck.queue, over, before };
  // The tank, parked on the deck.
  await move(tank.id, DECK, 80);
  for (let i = 0; i < 80; i++) {
    await lab(page, () => window.__lab.route.advance(60));
    if ((await unit(page, tank.id)).state === "idle") break;
  }
  const parked = await unit(page, tank.id);
  const onDeck =
    Math.abs(parked.position[0] - DECK[0]) < 12 && Math.abs(parked.position[1] - DECK[1]) < 3;
  const marker =
    hull("test_tank").half_extents_m[0] + visuals.overlay.orders.vehicle_marker_margin_m;
  const deck = await shown(await paintAt(parked.position, "deck"), ring(parked.position, marker));
  ctx.check(
    "ground paint shows on the water and on the bridge deck: queued dashes mid-river, a parked tank's ring",
    water.queue.some((q) => Math.hypot(q[0] - AFTER_RIVER[0], q[1] - AFTER_RIVER[1]) < 2) &&
      over.share >= 0.3 &&
      over.rise >= 0.8 * before.rise &&
      onDeck &&
      deck >= 0.7,
    JSON.stringify({ water, tank: parked.position, deck }),
  );
  await page.close();
}
