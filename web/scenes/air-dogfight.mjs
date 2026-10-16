// A 2v2 turning fight on air-jet, then a third jet joins.
// The simulation's tests behind it, in crates/sim/tests/dogfight.rs:
// firing_makes_it_your_target_and_it_adopts_you_only_if_free,
// a_third_jet_joins_by_targeting_an_engaged_one, the_link_breaks_past_the_rule_distance_in_3d,
// pursuit_aims_where_the_target_was, and dogfight_height_stays_in_its_band.
// The battle is directed: two blue jets start inside break range of two
// J-10s and are each sent at one; the third, held back, is flown at the fight
// and sent at the first J-10 once inside break range, since a jet sent at
// another from beyond it circles where it is.
// Browser-only evidence: on the real field the fight forms and grows to three
// blue aircraft, the selected one carries the DOGFIGHT card, and an explicit
// move takes it out of the fight while its wingman is still in it; the
// turning trace and shot are for review.
import { gpuWarnings, lab, obs } from "./_lab.mjs";
import { encounter, openAir, play, own, command, cardText, shoot, trace } from "./_air.mjs";
const JET = "f_16c_block_50";
const ENEMIES = encounter("dogfight", "air-dogfight").units.filter((u) => u.side === "red");

/** Blue's identified aircraft nearest where the encounter put `enemy`. */
const sighting = (o, enemy) =>
  o.identified
    .filter((u) => u.kind === enemy.kind)
    .sort(
      (a, b) =>
        Math.hypot(a.position[0] - enemy.position[0], a.position[1] - enemy.position[1]) -
        Math.hypot(b.position[0] - enemy.position[0], b.position[1] - enemy.position[1]),
    )[0];
const attack = (page, unit, target) =>
  command(page, { kind: "attack", units: [unit], target: { kind: "identified", id: target.id } });

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  const start = await obs(page);
  const blue = own(start, JET).map((u) => u.id);
  const [lead, second] = ENEMIES.map((e) => sighting(start, e));
  const locked = (o) =>
    own(o, JET)
      .filter((u) => u.inDogfight)
      .map((u) => u.id);
  let first = null;
  let joined = null;
  if (lead && second) {
    await attack(page, blue[0], lead);
    await attack(page, blue[1], second);
  }
  await play(page, 900, (o) => {
    if (locked(o).length >= 2) first = { tick: o.tick, ids: locked(o) };
    return !!first;
  });
  ctx.check("two blue jets lock into a dogfight", !!first, JSON.stringify({ first }));
  const reserve = blue.find((id) => !first?.ids.includes(id));
  if (first && lead && reserve !== undefined) {
    await command(page, {
      kind: "move",
      units: [reserve],
      gesture: 1,
      goal: lead.position.slice(0, 2),
      route: "fastest",
    });
    // Until the reserve is inside break range of the J-10 it is sent at.
    let near = null;
    await play(page, 1200, (o) => {
      const me = o.own.find((u) => u.id === reserve);
      near = o.identified.find((u) => u.id === lead.id) ?? null;
      return !!me && !!near && Math.hypot(...me.position.map((v, i) => v - near.position[i])) < 900;
    });
    if (near) await attack(page, reserve, near);
    await play(page, 600, (o) => {
      if (locked(o).length >= 3) joined = { tick: o.tick, ids: locked(o) };
      return !!joined;
    });
  }
  ctx.check("a third blue jet joins the fight", !!joined, JSON.stringify({ first, joined }));
  if (!joined) {
    await trace(page, 300, [0, 1, 2], ctx.evidencePath("dogfight-trace.png"));
    await shoot(ctx, page, "dogfight-turning-fight");
    ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
    return;
  }

  const mover = joined.ids[0];
  const wingman = joined.ids.find((id) => id !== mover);
  await lab(page, (id) => window.__lab.route.select([id]), mover);
  const card = await cardText(page, `[data-unit="${mover}"]`);
  ctx.check(
    "the selected aircraft card says DOGFIGHT",
    !!card && card.includes("DOGFIGHT"),
    card ?? "no card",
  );
  await shoot(ctx, page, "dogfight-turning-fight");

  const ack = await command(page, {
    kind: "move",
    units: [mover],
    gesture: 1,
    goal: [120, 2850],
    route: "fastest",
  });
  // The tick the mover leaves the fight, and whether its wingman is still in
  // it on that tick.
  let broke = null;
  let wingAtBreak = null;
  await play(page, 240, (o) => {
    const u = o.own.find((x) => x.id === mover);
    const w = o.own.find((x) => x.id === wingman);
    if (u && !u.inDogfight && broke === null) {
      broke = o.tick;
      wingAtBreak = !!w?.inDogfight;
    }
    return false;
  });
  ctx.check(
    "the move takes the selected aircraft out of the fight while its wingman stays in it",
    broke !== null && wingAtBreak === true,
    JSON.stringify({ ack, broke, wingman, wingAtBreak }),
  );
  await trace(page, 300, joined.ids, ctx.evidencePath("dogfight-trace.png"));
  ctx.check(
    "the selected aircraft leaves a DOGFIGHT card after its move",
    (await cardText(page, `[data-unit="${mover}"]`))?.includes("DOGFIGHT") === false,
    JSON.stringify({ ack, broke }),
  );
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
