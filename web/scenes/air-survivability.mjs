// A decoyable 290 m/s proximity-fused air missile on air-jet.
// The simulation's tests behind it:
//   crates/sim/tests/air_missiles.rs::a_missile_bursts_within_its_blast_radius_of_a_passing_jet
//   crates/sim/tests/guidance.rs::a_flare_is_ejected_near_the_aircraft_before_the_seeker_arrives
//   crates/sim/tests/air.rs::air_defence_follows_the_target_layer_and_only_guided_fire_needs_its_own_sight
// Browser-only evidence: the flared F-16 spends a charge and survives, the
// Apache without flares is hit, and the shot/flare tracks are visible together.
import { gpuWarnings } from "./_lab.mjs";
import { openAir, play, own, shoot } from "./_air.mjs";
import { hull, unitType } from "./_units.mjs";

const HELI = "ah_64e_guardian";
const JET = "f_16c_block_50";
const FLARES = unitType(JET).capabilities.flares.capacity;

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let heliHit = null;
  let heliLost = null;
  let flare = null;
  let afterFlare = null;
  await play(page, 1500, (o) => {
    const heli = own(o, HELI)[0];
    const jet = own(o, JET)[0];
    if (heli && heli.hp < hull(HELI).hp && !heliHit)
      heliHit = { tick: o.tick, hp: heli.hp, flares: heli.flares };
    if (!heli && !heliLost) heliLost = o.tick;
    if (jet && jet.flares.charges < FLARES && !flare)
      flare = {
        tick: o.tick,
        hp: jet.hp,
        charges: jet.flares.charges,
        cooldown: jet.flares.cooldown,
      };
    if (flare && !afterFlare && o.tick >= flare.tick + 60)
      afterFlare = jet ? { hp: jet.hp, charges: jet.flares.charges } : { lost: true };
    return !!afterFlare && !!heliLost;
  });
  await shoot(ctx, page, "survivability");
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
