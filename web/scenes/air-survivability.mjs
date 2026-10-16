// A decoyable 290 m/s proximity-fused air missile on air-jet: a Buk fires on
// a flared F-16 and an Apache without flares.
// The simulation's tests behind it:
//   crates/sim/tests/air_missiles.rs::a_missile_bursts_within_its_blast_radius_of_a_passing_jet
//   crates/sim/tests/guidance.rs::a_flare_is_ejected_near_the_aircraft_before_the_seeker_arrives
//   crates/sim/tests/guidance.rs::flares_decoy_a_seeker_fired_at_any_unit_that_carries_them
//   crates/sim/tests/protection.rs::missile_burst_at_a_nearby_flare_still_damages_the_airframe
//   crates/sim/tests/air.rs::air_defence_follows_the_target_layer_and_only_guided_fire_needs_its_own_sight
// Whether the flared jet comes out unhurt is theirs to say: a burst at a
// nearby flare still splashes the airframe. Browser-only evidence: the staged
// moment happens (the F-16 spends a flare, the Apache is shot down) and the
// shot shows the missile and flare tracks together.
import { gpuWarnings } from "./_lab.mjs";
import { openAir, play, own, shoot } from "./_air.mjs";
import { unitType } from "./_units.mjs";

const HELI = "ah_64e_guardian";
const JET = "f_16c_block_50";
const FLARES = unitType(JET).capabilities.flares.capacity;

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let flared = null;
  let heliLost = null;
  // Until both have happened, then a second more for the tracks to show.
  await play(page, 1500, (o) => {
    const jet = own(o, JET)[0];
    if (!own(o, HELI)[0] && heliLost === null) heliLost = o.tick;
    if (jet && jet.flares.charges < FLARES && flared === null) flared = o.tick;
    return flared !== null && heliLost !== null && o.tick >= flared + 60;
  });
  ctx.check(
    "the F-16 spends a flare and the Apache is shot down",
    flared !== null && heliLost !== null,
    JSON.stringify({ flared, heliLost }),
  );
  await shoot(ctx, page, "survivability");
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
