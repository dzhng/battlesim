// Paid jet ingress and edge withdrawal on air-jet.
// The simulation's tests behind it, in crates/sim/tests/skirmish.rs:
// confirmed_purchases_reserve_credits_and_slots_atomically_during_preparation,
// a_waiting_aircraft_leaves_the_queue_and_waits_on_its_own_clock,
// any_aircraft_enters_flying_at_the_edge_point_nearest_its_entry_site,
// inbound_transit_is_published_to_its_own_side_only, nothing_senses_or_targets_a_jet_before_it_arrives,
// withdrawal_retires_and_refunds_on_crossing_the_nearest_back_edge_point, and fuel_return_uses_the_edge_distance;
// the drawn approach and exit tracks by crates/presentation/tests/transit.rs.
// Browser-only evidence: both purchases debit immediately; the delayed aircraft
// has no unit before arrival, only its pending card counting down to the
// published arrival tick (D42); the approach pose is visible over the margin;
// arrival is a cruise-height aircraft at the back-edge point nearest blue's
// entry site, heading for its destination (D39); and the low-fuel aircraft's
// outbound pose leaves at the blue back edge before the refund appears.
import { writeFile } from "node:fs/promises";
import { gpuWarnings, obs, own, advance, aim } from "./_lab.mjs";
import { openAir, command, field, HIGH_AIR_CRUISE, shoot } from "./_air.mjs";
import { rules, unitType } from "./_units.mjs";
const JET = "f_16c_block_50";
const DESTINATIONS = [
  [1500, 1500],
  [2500, 2400],
];
/** Blue's entry site in the ingress skirmish (`apps/battle-lab/src/routes/airWar.tsx`)
 *  sits mid-way along the west edge: that is its back edge, and the point on
 *  it nearest the site is where every bought aircraft arrives. */
const ARRIVAL_POINT = [0, field("air-ingress")[1] / 2];

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let o = await obs(page);
  ctx.check(
    "the checkpoint is an empty skirmish on air-jet",
    !!o.skirmish && o.own.length === 0,
    JSON.stringify(o.skirmish),
  );
  // The whole field from the south, as the checkpoint opens: its objectives
  // spread across it.
  await shoot(ctx, page, "air-ingress-field");
  await command(page, { kind: "ready" });
  await advance(page, 2);
  // Income lands every active tick, so each credit reading below is off by
  // one tick of it: measure that tick while nothing is bought.
  const idle = own(await obs(page)).credits;
  await advance(page, 1);
  const before = own(await obs(page)).credits;
  const tickIncome = before - idle;
  for (const destination of DESTINATIONS)
    await command(page, { kind: "confirm_purchase", variant: JET, destination });
  await advance(page, 1);
  o = await obs(page);
  const cost = before - own(o).credits;
  const price = unitType(JET).cost;
  ctx.check(
    "both purchases debit immediately",
    Math.abs(cost - DESTINATIONS.length * price) <= 2 * tickIncome + 1e-6,
    JSON.stringify({ cost, price, tickIncome }),
  );

  ctx.check(
    "the delayed aircraft is not a unit",
    o.own.filter((u) => u.kind === JET).length === 0,
    JSON.stringify(o.own),
  );

  // While the jets wait off the map, each pending card counts down to the
  // arrival tick its pending record publishes.
  const waiting = own(o).pending.filter((p) => p.kind === JET);
  // The battle is paused: wait for the deck to draw this observation.
  await page
    .waitForFunction(
      (n) => document.querySelectorAll(".hud-army-arriving").length === n,
      waiting.length,
      { timeout: 10_000 },
    )
    .catch(() => {});
  const cards = await page.locator(".hud-army-arriving").allTextContents();
  const due = waiting.map(
    (p) => `Arriving in ${Math.ceil((p.arrivalTick - o.tick) / rules.tick_hz)} s`,
  );
  ctx.check(
    "each waiting jet's pending card counts down to its published arrival",
    waiting.length === 2 &&
      waiting.every((p) => p.arrivalTick > o.tick) &&
      cards.map((c) => c.replace(/\s+/g, " ").trim()).join("|") === due.join("|"),
    JSON.stringify({ cards, due, pending: own(o).pending, tick: o.tick }),
  );
  const deck = page.locator(".hud-army-deck");
  await writeFile(ctx.evidencePath("air-ingress-pending.png"), await deck.screenshot());

  let inbound = null;
  let approachShot = false;
  let entered = null;
  let firstId = null;
  for (let i = 0; i < 900; i++) {
    await advance(page, 1);
    o = await obs(page);
    if (!inbound && o.airTransits.some((t) => t.dir === "in")) {
      inbound = { tick: o.tick, transits: o.airTransits.filter((t) => t.dir === "in") };
      ctx.check(
        "the approach frame has no aircraft unit behind its transit",
        o.own.filter((u) => u.kind === JET).length === 0,
        JSON.stringify(inbound),
      );
    }
    // The approach is drawn in the last seconds before arrival.
    const coming = o.airTransits.find((t) => t.dir === "in");
    if (coming && !approachShot && coming.edgeTick - o.tick <= 4 * rules.tick_hz) {
      approachShot = true;
      // Close on where the approach draws the jet now: back along its heading
      // from the edge point by the distance it has still to fly.
      const ahead = (coming.speedMps * (coming.edgeTick - o.tick)) / rules.tick_hz;
      const at = [
        coming.point[0] - Math.cos(coming.heading) * ahead,
        coming.point[1] - Math.sin(coming.heading) * ahead,
        coming.point[2],
      ];
      await aim(
        page,
        at,
        { distance: 140, pitch: 0.35, yaw: coming.heading + Math.PI * 0.8 },
        { onGround: false },
      );
      await shoot(ctx, page, "air-ingress-approach");
    }
    const jet = o.own.find((u) => u.kind === JET);
    if (jet && !firstId) {
      firstId = jet.id;
      entered = {
        tick: o.tick,
        position: jet.position,
        yaw: jet.yaw,
        fuel: jet.fuelS,
        goal: jet.goal,
      };
      break;
    }
  }
  ctx.check(
    "the inbound AirTransit is seen before the unit arrives",
    !!inbound && !!entered && inbound.tick < entered.tick,
    JSON.stringify({ inbound, entered }),
  );
  // One tick of flight past the edge point, at cruise speed.
  const tickRun = unitType(JET).mobility.air.cruise_kmh / 3.6 / rules.tick_hz;
  const goal = entered?.goal ?? DESTINATIONS[0];
  const bearing = Math.atan2(goal[1] - ARRIVAL_POINT[1], goal[0] - ARRIVAL_POINT[0]);
  ctx.check(
    "the jet arrives at the back-edge point nearest blue's entry, at cruise height, heading for its destination",
    !!entered &&
      Math.hypot(entered.position[0] - ARRIVAL_POINT[0], entered.position[1] - ARRIVAL_POINT[1]) <=
        2 * tickRun + 1 &&
      Math.abs(entered.position[2] - HIGH_AIR_CRUISE) < 2 &&
      Math.cos(entered.yaw - bearing) > Math.cos(0.1),
    JSON.stringify({ entered, at: ARRIVAL_POINT, bearing }),
  );
  if (!entered) return;

  let withdrawing = null;
  let outbound = null;
  let retired = null;
  let wallet = own(await obs(page)).credits;
  // Coarse steps until the return starts, then tick by tick: the departure is
  // published on the single tick the jet crosses its edge.
  for (let i = 0; i < 7200 && !retired; i += withdrawing ? 1 : 30) {
    await advance(page, withdrawing ? 1 : 30);
    const now = await obs(page);
    const jet = now.own.find((u) => u.id === firstId);
    if (jet?.withdrawing && !withdrawing)
      withdrawing = { tick: now.tick, fuel: jet.fuelS, position: jet.position };
    if (now.airTransits.some((t) => t.dir === "out"))
      outbound = { tick: now.tick, transits: now.airTransits.filter((t) => t.dir === "out") };
    if (!jet) retired = { tick: now.tick, refund: own(now).credits - wallet };
    wallet = own(now).credits;
  }
  ctx.check(
    "low fuel starts the return",
    !!withdrawing && withdrawing.fuel > 0,
    JSON.stringify(withdrawing),
  );
  ctx.check(
    "the outbound effect uses the nearest back edge",
    !!outbound && outbound.transits.some((t) => t.point[0] <= 10),
    JSON.stringify(outbound),
  );
  ctx.check(
    "the jet retires after its outbound transit and is refunded",
    !!retired && !!outbound && outbound.tick <= retired.tick && retired.refund > 2 * tickIncome,
    JSON.stringify({ outbound: outbound?.tick, retired, tickIncome }),
  );

  await writeFile(
    ctx.evidencePath("ingress.json"),
    JSON.stringify({ cost, inbound, entered, withdrawing, outbound, retired }, null, 2),
  );
  await shoot(ctx, page, "air-ingress");
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
