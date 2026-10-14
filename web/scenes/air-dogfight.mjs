// Dogfights (slice 05): two F-16s against two J-10s. The J-10 that may
// fire opens on a blue jet, and the attacked jet and its attacker lock into
// a dogfight; the second blue jet joins against the same enemy, so the
// dogfight is any number of aircraft, not pairs. The selected jet's card
// says DOGFIGHT. An explicit move breaks the mover's dogfight at once, and
// it stays out while it flies the order, though its wingman fights on.
//
// Not shown, because the field is too small: the simulation breaks a
// dogfight past 1,200 m, more than the air map's diagonal. Not shown,
// because the simulation does not do it: Stop leaves a dogfight in place
// (only a move or an attack order replaces it).
import { gpuWarnings, lab } from "./_lab.mjs";
import { openAir, play, own, command, cardText, shoot } from "./_air.mjs";

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let first = null;
  let both = null;
  await play(page, 240, (o) => {
    const jets = own(o, "f_16c_block_50");
    const locked = jets.filter((u) => u.inDogfight);
    if (!first && locked.length) first = { tick: o.tick, id: locked[0].id };
    if (locked.length === 2) both = { tick: o.tick, ids: locked.map((u) => u.id) };
    return !!both;
  });
  ctx.check("an attacked jet locks into a dogfight", !!first, JSON.stringify(first));
  ctx.check(
    "the second blue jet joins the same fight: both in it at once",
    !!both,
    JSON.stringify(both),
  );
  if (!both) return;
  const mover = both.ids.find((id) => id !== first.id) ?? both.ids[0];
  const wing = both.ids.find((id) => id !== mover);
  await lab(page, (id) => window.__lab.route.select([id]), mover);
  const card = await cardText(page, `[data-unit="${mover}"]`);
  ctx.check("its card says DOGFIGHT", !!card && card.includes("DOGFIGHT"), card ?? "no card");
  await shoot(ctx, page, "dogfight");

  const ack = await command(page, {
    kind: "move",
    units: [mover],
    gesture: 1,
    goal: [60, 40],
    route: "fastest",
  });
  let broke = null;
  let relinked = false;
  let wingIn = false;
  await play(page, 30, (o) => {
    const u = o.own.find((u) => u.id === mover);
    if (u && !u.inDogfight && !broke) broke = o.tick;
    if (broke && u?.inDogfight) relinked = true;
    if (o.own.find((u) => u.id === wing)?.inDogfight) wingIn = true;
    return false;
  });
  ctx.check(
    "an explicit move breaks the mover's dogfight within a tick of its order",
    ack?.error === null && broke !== null && broke <= ack.applied_tick + 1,
    JSON.stringify({ ack, broke }),
  );
  ctx.check("it stays out of the fight while it flies the order", !relinked);
  ctx.check("its wingman fights on: the move broke only the mover's link", wingIn, `wing ${wing}`);
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
