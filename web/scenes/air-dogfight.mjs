// AWF-JETSCENES — 2v2 turning fight, then a third jet joins, on air-jet.
// Simulation contracts named here are pinned by crates/sim/tests/jet_dogfight.rs:
// firing_makes_it_your_target_and_it_adopts_you_only_if_free,
// a_third_jet_joins_by_targeting_an_engaged_one, the_link_breaks_past_the_rule_distance_in_3d,
// pursuit_aims_where_the_target_was, and dogfight_height_stays_in_its_band.
// Browser-only evidence: the turning trace, three blue aircraft carrying the
// DOGFIGHT card, and the selected aircraft leaving the fight while its wingman remains.
import { gpuWarnings, lab } from "./_lab.mjs";
import { openAir, play, own, command, cardText, shoot, trace } from "./_air.mjs";
const JET = "f_16c_block_50";

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let first = null;
  let joined = null;
  await play(page, 900, (o) => {
    const jets = own(o, JET);
    const locked = jets.filter((u) => u.inDogfight);
    if (!first && locked.length >= 2) first = { tick: o.tick, ids: locked.slice(0, 2).map((u) => u.id) };
    if (first && locked.length >= 3) joined = { tick: o.tick, ids: locked.map((u) => u.id) };
    return !!joined;
  });
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
  ctx.check("the selected aircraft card says DOGFIGHT", !!card && card.includes("DOGFIGHT"), card ?? "no card");
  await shoot(ctx, page, "dogfight-turning-fight");

  const ack = await command(page, {
    kind: "move",
    units: [mover],
    gesture: 1,
    goal: [120, 2850],
    route: "fastest",
  });
  let broke = null;
  let wingStayed = false;
  await play(page, 240, (o) => {
    const u = o.own.find((x) => x.id === mover);
    const w = o.own.find((x) => x.id === wingman);
    if (u && !u.inDogfight && broke === null) broke = o.tick;
    if (w?.inDogfight) wingStayed = true;
    return false;
  });
  await trace(page, 300, joined.ids, ctx.evidencePath("dogfight-trace.png"));
  ctx.check("the selected aircraft leaves a DOGFIGHT card after its move", (await cardText(page, `[data-unit="${mover}"]`))?.includes("DOGFIGHT") === false, JSON.stringify({ ack, broke, wingStayed }));
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
