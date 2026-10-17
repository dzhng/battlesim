// Slices 06 and 10: board and collect through the real transport lab, and
// the carriers' cards: their Seats rows, and their riders' compact cards
// stacked under them, compact or opened.
import { openBattle, advance, lab, until, snapshot } from "./_lab.mjs";

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
