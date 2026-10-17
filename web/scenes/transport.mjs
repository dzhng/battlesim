// Slice 06: board and collect through the real transport lab.
import { openBattle, advance, lab, until, snapshot } from "./_lab.mjs";

// The `board` encounter scripts the carriers' drive at tick 900; every squad
// walks to (or is fetched by) its carrier and boards before then.
const DRIVE_TICK = 900;

export async function run(ctx) {
  const page = await openBattle(ctx);
  await advance(page, 2);
  const start = await lab(page, () => window.__lab.route.observation());
  ctx.check("transport lab loads", !!start && start.own.length >= 4, JSON.stringify(start));
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
  await snapshot(ctx, page, "boarded.png");
  await page.close();
}
