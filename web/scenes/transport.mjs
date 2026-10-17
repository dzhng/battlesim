// Slice 06: board and collect through the real transport lab.
import { openBattle, advance, lab } from "./_lab.mjs";

export async function run(ctx) {
  const page = await openBattle(ctx);
  await advance(page, 2);
  const start = await lab(page, () => window.__lab.route.observation());
  ctx.check("transport lab loads", !!start && start.own.length >= 4, JSON.stringify(start));
  await advance(page, 90);
  const boarded = await lab(page, () => window.__lab.route.observation());
  const passengers = boarded.own.filter((u) => u.hold?.host?.kind === "carrier");
  ctx.check(
    "boarded squads publish a carrier hold",
    passengers.length >= 1,
    JSON.stringify(passengers),
  );
  await page.close();
}
