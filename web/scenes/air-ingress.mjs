// AWF-JETSCENES — paid jet ingress and edge withdrawal on air-jet.
// Simulation contracts named here are pinned by crates/sim/tests/skirmish.rs:
// confirmed_purchases_reserve_credits_and_slots_atomically_during_preparation,
// a_bought_jet_spawns_on_its_back_edge_at_cruise_heading_in_after_its_delay,
// inbound_transit_is_published_to_its_own_side_only, nothing_senses_or_targets_a_jet_before_it_arrives,
// withdrawal_retires_and_refunds_on_crossing_the_nearest_back_edge_point, and fuel_return_uses_the_edge_distance.
// Browser-only evidence: both purchases debit immediately; the delayed aircraft
// has no unit/card before arrival; the approach pose is visible over the margin;
// arrival is a cruise-height aircraft heading in; and the low-fuel aircraft's
// outbound pose leaves at the blue back edge before the refund appears.
import { writeFile } from "node:fs/promises";
import { gpuWarnings, obs, advance } from "./_lab.mjs";
import { openAir, command, JET_CRUISE, shoot } from "./_air.mjs";
const JET = "f_16c_block_50";
const DESTINATIONS = [[1500, 1500], [2500, 2400]];

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let o = await obs(page);
  ctx.check("the checkpoint is an empty skirmish on air-jet", !!o.skirmish && o.own.length === 0, JSON.stringify(o.skirmish));
  await command(page, { kind: "ready" });
  await advance(page, 2);
  const before = (await obs(page)).skirmish.credits;
  for (const destination of DESTINATIONS)
    await command(page, { kind: "confirm_purchase", variant: JET, destination });
  await advance(page, 1);
  o = await obs(page);
  const cost = before - o.skirmish.credits;

  o = await obs(page);
  ctx.check("the delayed aircraft is not a unit", o.own.filter((u) => u.kind === JET).length === 0, JSON.stringify(o.own));

  let inbound = null;
  let entered = null;
  let firstId = null;
  for (let i = 0; i < 900; i++) {
    await advance(page, 1);
    o = await obs(page);
    if (!inbound && o.airTransits.some((t) => t.dir === "in")) {
      inbound = { tick: o.tick, transits: o.airTransits.filter((t) => t.dir === "in") };
      ctx.check("the approach frame has no aircraft unit behind its transit", o.own.filter((u) => u.kind === JET).length === 0, JSON.stringify(inbound));
      await shoot(ctx, page, "air-ingress-approach");
    }
    const jet = o.own.find((u) => u.kind === JET);
    if (jet && !firstId) {
      firstId = jet.id;
      entered = { tick: o.tick, position: jet.position, yaw: jet.yaw, fuel: jet.fuelS };
      break;
    }
  }
  ctx.check("the inbound AirTransit is seen before the unit arrives", !!inbound && !!entered && inbound.tick <= entered.tick, JSON.stringify({ inbound, entered }));
  ctx.check(
    "the jet arrives on the blue back edge at cruise height heading in",
    !!entered && entered.position[0] < 400 && Math.abs(entered.position[2] - JET_CRUISE) < 2 && Math.cos(entered.yaw) > 0,
    JSON.stringify(entered),
  );
  if (!entered) return;

  let withdrawing = null;
  let outbound = null;
  let retired = null;
  let wallet = (await obs(page)).skirmish.credits;
  for (let i = 0; i < 7200 && !retired; i += 30) {
    await advance(page, 30);
    const now = await obs(page);
    const jet = now.own.find((u) => u.id === firstId);
    if (jet?.withdrawing && !withdrawing) withdrawing = { tick: now.tick, fuel: jet.fuelS, position: jet.position };
    if (now.airTransits.some((t) => t.dir === "out")) outbound = { tick: now.tick, transits: now.airTransits.filter((t) => t.dir === "out") };
    if (!jet) retired = { tick: now.tick, refund: now.skirmish.credits - wallet };
    wallet = now.skirmish.credits;
  }
  ctx.check("low fuel starts the return", !!withdrawing && withdrawing.fuelS > 0, JSON.stringify(withdrawing));
  ctx.check("the outbound effect uses the nearest back edge", !!outbound && outbound.transits.some((t) => t.point[0] <= 10), JSON.stringify(outbound));

  await writeFile(ctx.evidencePath("ingress.json"), JSON.stringify({ cost, inbound, entered, withdrawing, outbound, retired }, null, 2));
  await shoot(ctx, page, "air-ingress");
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
