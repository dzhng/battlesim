// Slices 06, 10 and 11: board and collect through the real transport lab,
// the carriers' cards (their Seats rows, and their riders' compact cards
// stacked under them, compact or opened), and the `controls` variant driven
// by the player's own controls: right-click boarding both ways and its
// cursor, a refusal at the squad, U then a click with its unload mark, and
// U twice.
import { openBattle, advance, lab, obs, until, snapshot } from "./_lab.mjs";
import { decode, writeCrop } from "./_png.mjs";

/** The lab's view at its own framing and pulled out to a far view (where
 *  cards may overprint), saved as `<name>-default.png` and `<name>-far.png`. */
async function views(ctx, page, name) {
  const framing = await lab(page, () => window.__lab.camera());
  await snapshot(ctx, page, `${name}-default.png`);
  await lab(page, (c) => window.__lab.setCamera({ ...c, distance: 1400 }), framing);
  await snapshot(ctx, page, `${name}-far.png`);
  await lab(page, (c) => window.__lab.setCamera(c), framing);
}

/** Each own carrier's card as drawn after a fresh frame: whether it shows,
 *  its Seats row's words and whether that row shows, how many of its own
 *  other rows show, and each rider card under it with its zoom and the rows
 *  it shows. */
const carrierCards = (page) =>
  page.evaluate(async () => {
    await window.__lab.frame();
    const shown = (e) => !!e && e.getClientRects().length > 0;
    return [...document.querySelectorAll("[data-testid=readouts] .ro-callout > .ro-unit")]
      .filter((card) => card.querySelector(':scope > .ro-body [data-state="seats"]'))
      .map((card) => {
        const seats = card.querySelector(':scope > .ro-body [data-state="seats"]');
        const body = card.querySelector(":scope > .ro-body").getBoundingClientRect();
        const firstRider = card.querySelector(":scope > .ro-aboard > [data-passenger]");
        // The leader's rule: the height of its last point, along the card.
        const d = card.parentElement.querySelector(".ro-leader")?.getAttribute("d") ?? "";
        const rule = Number(d.trim().split(/\s+/).at(-1));
        return {
          // The rule underlines the carrier's own panel, above its riders.
          ruleUnderOwn:
            rule >= body.bottom - 0.5 &&
            (!firstRider || rule <= firstRider.getBoundingClientRect().top + 0.5),
          unit: Number(card.dataset.unit),
          shown: shown(card),
          zoom: card.dataset.zoom,
          seats: seats.textContent.trim(),
          seatsShown: shown(seats),
          otherRows: [...card.querySelectorAll(":scope > .ro-body .ro-row")].filter(
            (r) => r !== seats && shown(r),
          ).length,
          riders: [...card.querySelectorAll(":scope > .ro-aboard > [data-passenger]")].map((r) => ({
            unit: Number(r.dataset.unit),
            zoom: r.dataset.zoom,
            rows: [...r.querySelectorAll(".ro-row")].filter(shown).length,
            inside:
              r.getBoundingClientRect().top >=
              card.querySelector(":scope > .ro-body").getBoundingClientRect().bottom - 0.5,
          })),
        };
      });
  });

/** The living soldiers of the squads `o` says ride carrier `id`. */
const seated = (o, id) =>
  o.own
    .filter((u) => u.hold?.host?.kind === "carrier" && u.hold.host.id === id)
    .reduce((n, u) => n + u.memberHp.filter((hp) => hp > 0).length, 0);

// The `board` encounter scripts the carriers' drive at tick 900; every squad
// walks to (or is fetched by) its carrier and boards before then.
const DRIVE_TICK = 900;

export async function run(ctx) {
  await scripted(ctx);
  await controls(ctx);
}

async function scripted(ctx) {
  const page = await openBattle(ctx);
  await advance(page, 2);
  const start = await lab(page, () => window.__lab.route.observation());
  ctx.check("transport lab loads", !!start && start.own.length >= 4, JSON.stringify(start));
  await views(ctx, page, "empty");
  const empty = await carrierCards(page);
  ctx.check(
    "each empty carrier's compact card shows its Seats row alone: SEATS 0/N",
    empty.length === 2 &&
      empty.every(
        (c) =>
          c.shown &&
          c.zoom === "compressed" &&
          c.seatsShown &&
          /^SEATS 0\/\d+$/.test(c.seats) &&
          c.otherRows === 0 &&
          c.riders.length === 0,
      ),
    JSON.stringify(empty),
  );
  const squads = start.own.filter((u) => u.memberIds.length > 0).map((u) => u.id);
  const aboard = (o) =>
    squads.every((id) => {
      const hold = o.own.find((u) => u.id === id)?.hold;
      return hold?.host?.kind === "carrier" && hold.phase === "inside";
    });
  const boarded = await until(page, aboard, DRIVE_TICK - start.tick - 30, 30);
  const passengers = (boarded ?? (await lab(page, () => window.__lab.route.observation()))).own
    .filter((u) => u.hold?.host?.kind === "carrier")
    .map((u) => ({ id: u.id, tick: boarded?.tick, hold: u.hold }));
  ctx.check(
    "boarded squads publish a carrier hold",
    !!boarded && passengers.length === squads.length,
    JSON.stringify(passengers),
  );
  await views(ctx, page, "boarded");
  const now = boarded ?? (await lab(page, () => window.__lab.route.observation()));
  const full = await carrierCards(page);
  const held = (c) => passengers.filter((p) => p.hold.host.id === c.unit).map((p) => p.id);
  ctx.check(
    "each carrier's card shows, counts its riders' living soldiers, stacks their compact cards under it, and its leader underlines its own panel",
    full.length === 2 &&
      full.every(
        (c) =>
          c.shown &&
          c.seatsShown &&
          c.seats.startsWith(`SEATS ${seated(now, c.unit)}/`) &&
          seated(now, c.unit) > 0 &&
          JSON.stringify(c.riders.map((r) => r.unit)) === JSON.stringify(held(c)) &&
          c.riders.every((r) => r.zoom === "compressed" && r.rows === 0 && r.inside) &&
          c.ruleUnderOwn,
      ),
    JSON.stringify(full),
  );
  // Space opens every card that fits; a rider's stays compact.
  await page.keyboard.down("Space");
  const opened = await carrierCards(page);
  await snapshot(ctx, page, "boarded-opened-default.png");
  await page.keyboard.up("Space");
  ctx.check(
    "with Space held a carrier's card opens and its riders' cards stay compact",
    opened.some((c) => c.zoom === "default" && c.otherRows > 0) &&
      opened.every((c) => c.riders.every((r) => r.zoom === "compressed" && r.rows === 0)),
    JSON.stringify(opened),
  );
  await page.close();
}

const project = (page, at) => lab(page, (p) => window.__lab.projectToCss(...p), at);
const select = async (page, ids) => {
  await lab(page, (ids) => window.__lab.route.select(ids), ids);
  await page.waitForFunction(
    (ids) => JSON.stringify(window.__lab.route.selected()) === JSON.stringify(ids),
    ids,
  );
};
/** The cursor the system shows becomes `action`, within a few frames; false if not. */
const cursorBecomes = (page, action) =>
  page
    .waitForFunction((a) => document.documentElement.dataset.cursor === a, action, {
      timeout: 5000,
    })
    .then(
      () => true,
      () => false,
    );
/** The page point over own unit `id`: a metre up its hull, or its first
 *  soldier's chest (a squad is picked by its soldiers). */
async function over(page, id) {
  const u = (await obs(page)).own.find((x) => x.id === id);
  const p = u.members[0] ?? u.position;
  return project(page, [p[0], p[1], p[2] + 1]);
}
/** Where `id` rides, or null on the field. */
const host = (o, id) => {
  const hold = o.own.find((u) => u.id === id)?.hold;
  return hold?.host?.kind === "carrier" && hold.phase === "inside" ? hold.host.id : null;
};
/** The newest acknowledged order, once one newer than `seq` has come back. */
const ackAfter = async (page, seq) => {
  await page.waitForFunction((seq) => (window.__lab.route.acks()[0]?.seq ?? 0) > seq, seq);
  return lab(page, () => window.__lab.route.acks()[0]);
};
const newestSeq = (page) => lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);

/** The order line of the APC's order to `goal` with Space held, whole and
 *  as a 3× crop round its destination, at the default and a far view. */
async function orderLine(ctx, page, goal, name) {
  await advance(page, 6);
  await page.mouse.move(1260, 400);
  await page.keyboard.down("Space");
  const framing = await lab(page, () => window.__lab.camera());
  const shot = decode(await snapshot(ctx, page, `${name}-default.png`));
  const at = await project(page, [goal[0], goal[1], 0]);
  await writeCrop(shot, ctx.evidencePath(`${name}-crop-3x.png`), at[0], at[1], 60, 40, 3);
  await lab(page, (c) => window.__lab.setCamera({ ...c, distance: 700 }), framing);
  const far = decode(await snapshot(ctx, page, `${name}-far.png`));
  const atFar = await project(page, [goal[0], goal[1], 0]);
  await writeCrop(far, ctx.evidencePath(`${name}-far-crop-3x.png`), atFar[0], atFar[1], 40, 28, 3);
  await lab(page, (c) => window.__lab.setCamera(c), framing);
  await page.keyboard.up("Space");
}

async function controls(ctx) {
  const url = new URL(ctx.url);
  url.searchParams.set("variant", "controls");
  const page = await openBattle(ctx, { url: url.href });
  await advance(page, 2);
  const start = await obs(page);
  const id = (kind) => start.own.find((u) => u.kind === kind).id;
  const [apc, car, rifle, team] = ["test_apc", "test_scout_car", "test_rifle", "test_at"].map(id);

  // A squad too big for the scout car's seats: refused, said at the squad.
  await select(page, [rifle]);
  let at = await over(page, car);
  await page.mouse.move(...at);
  ctx.check(
    "a selected squad over an own carrier shows the board cursor",
    await cursorBecomes(page, "board"),
    JSON.stringify(await lab(page, () => window.__lab.route.pointerIntent())),
  );
  const shot = decode(await snapshot(ctx, page, "controls-board-cursor.png"));
  await writeCrop(
    shot,
    ctx.evidencePath("controls-board-cursor-3x.png"),
    at[0] + 18,
    at[1] + 18,
    60,
    46,
    3,
  );
  let seq = await newestSeq(page);
  await page.mouse.click(...at, { button: "right" });
  const tooBig = await ackAfter(page, seq);
  await advance(page, 2);
  const message = page.getByTestId("rejected-order");
  const refusal = await message
    .waitFor({ timeout: 5000 })
    .then(() =>
      page.evaluate(() => {
        const m = document.querySelector("[data-testid=rejected-order]");
        const card = document.querySelector(`.ro-unit[data-unit="${m.dataset.unit}"]`);
        const a = m.getBoundingClientRect();
        const b = card?.getBoundingClientRect();
        return {
          text: m.textContent.trim(),
          unit: Number(m.dataset.unit),
          // Beside the card's top, on its right or left.
          besideCard:
            !!b &&
            Math.abs(a.top - b.top) < 2 &&
            (Math.abs(a.left - b.right - 6) < 2 || Math.abs(a.right - b.left + 6) < 2),
        };
      }),
    )
    .catch(() => null);
  await snapshot(ctx, page, "controls-refused.png");
  // Its moment passes (in real time) before the next shots.
  await message.waitFor({ state: "detached", timeout: 10000 });
  ctx.check(
    "boarding a carrier with too few seats is refused at the squad, beside its card",
    tooBig.order.kind === "board" &&
      tooBig.ack.refused.some((r) => r.unit === rifle && r.reason.reason === "capacity_full") &&
      refusal?.unit === rifle &&
      refusal.text === "TOO MANY TO FIT" &&
      refusal.besideCard,
    JSON.stringify({ ack: tooBig.ack, refusal }),
  );

  // Squads selected, right-click an own carrier: they board it.
  at = await over(page, apc);
  await page.mouse.move(...at);
  await cursorBecomes(page, "board");
  seq = await newestSeq(page);
  await page.mouse.click(...at, { button: "right" });
  const board = await ackAfter(page, seq);
  const boarded = await until(page, (o) => host(o, rifle) === apc, 900, 30);
  ctx.check(
    "right-clicking an own carrier with a squad selected boards it, and the squad leaves the selection",
    board.order.kind === "board" &&
      board.order.carrier === apc &&
      !!boarded &&
      (await lab(page, () => window.__lab.route.selected())).length === 0,
    JSON.stringify({ order: board.order, ack: board.ack }),
  );
  // Its deck card stays, dimmed as aboard, and picks its carrier.
  const card = page.locator(`.hud-army-card[data-unit="${rifle}"]`);
  const aboardCard = await card.getAttribute("data-aboard");
  await card.click();
  ctx.check(
    "a riding squad's deck card is marked aboard and selects its carrier",
    aboardCard === "true" &&
      JSON.stringify(await lab(page, () => window.__lab.route.selected())) ===
        JSON.stringify([apc]),
    String(aboardCard),
  );
  await snapshot(ctx, page, "controls-aboard-deck.png");

  // A carrier selected, right-click an own squad: it drives over and collects it.
  await select(page, [car]);
  at = await over(page, team);
  await page.mouse.move(...at);
  const collectCursor = await cursorBecomes(page, "board");
  seq = await newestSeq(page);
  await page.mouse.click(...at, { button: "right" });
  const collect = await ackAfter(page, seq);
  const collected = await until(page, (o) => host(o, team) === car, 1200, 30);
  ctx.check(
    "right-clicking an own squad with a carrier selected collects it, under the board cursor",
    collectCursor &&
      collect.order.kind === "collect" &&
      JSON.stringify(collect.order.squads) === JSON.stringify([team]) &&
      !!collected,
    JSON.stringify({ order: collect.order, ack: collect.ack }),
  );

  // The loaded APC: an ordinary move line, then U and a click to the same
  // point, whose line carries the unload mark.
  const from = (await obs(page)).own.find((u) => u.id === apc).position;
  const goal = [from[0] + 45, from[1] - 20];
  await select(page, [apc]);
  const goalAt = await project(page, [goal[0], goal[1], 0]);
  await page.mouse.click(...goalAt, { button: "right" });
  await orderLine(ctx, page, goal, "move-line");
  await page.keyboard.press("Backspace");
  await advance(page, 4);
  await page.keyboard.press("KeyU");
  const armed = await page.getByRole("button", { name: /^Unload/ }).getAttribute("aria-pressed");
  await page.mouse.move(...goalAt);
  const unloadCursor = await cursorBecomes(page, "unload");
  seq = await newestSeq(page);
  await page.mouse.click(...goalAt);
  const unload = await ackAfter(page, seq);
  await orderLine(ctx, page, goal, "unload-line");
  const out = await until(
    page,
    (o) => host(o, rifle) === null && !o.own.find((u) => u.id === rifle).hold,
    1500,
    30,
  );
  const landed = out?.own.find((u) => u.id === rifle);
  const carrierAt = out?.own.find((u) => u.id === apc).position;
  ctx.check(
    "U then a ground click drives the carrier there and unloads it",
    armed === "true" &&
      unloadCursor &&
      unload.order.kind === "unload" &&
      !!unload.order.goal &&
      !unload.ack.error &&
      !!landed &&
      Math.hypot(carrierAt[0] - goal[0], carrierAt[1] - goal[1]) < 8 &&
      Math.hypot(landed.position[0] - carrierAt[0], landed.position[1] - carrierAt[1]) < 10,
    JSON.stringify({
      armed,
      unloadCursor,
      order: unload.order,
      ack: unload.ack,
      carrierAt,
      landed: landed?.position,
    }),
  );

  // U twice: the scout car stops and unloads where it stands.
  await select(page, [car]);
  seq = await newestSeq(page);
  await page.keyboard.press("KeyU");
  await page.keyboard.press("KeyU");
  const inPlace = await ackAfter(page, seq);
  const stood = (await obs(page)).own.find((u) => u.id === car).position;
  const freed = await until(page, (o) => !o.own.find((u) => u.id === team).hold, 600, 15);
  const there = freed?.own.find((u) => u.id === team).position;
  ctx.check(
    "U twice unloads the carrier in place at once",
    inPlace.order.kind === "unload" &&
      inPlace.order.goal === null &&
      !inPlace.ack.error &&
      !!there &&
      Math.hypot(there[0] - stood[0], there[1] - stood[1]) < 10,
    JSON.stringify({ order: inPlace.order, ack: inPlace.ack, stood, there }),
  );
  await snapshot(ctx, page, "controls-unloaded.png");
  await page.close();
}
