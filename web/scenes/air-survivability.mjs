// Survivability and flares (slice 08): an F-16 with flares and an Apache
// without them under a Buk, whose missile is decoyable. The Apache is hit
// and lost. When the Buk's missile locks the jet, the jet spends one flare,
// the missile is diverted and the jet is unhurt; the flare's reload runs.
//
// Not shown on the battle view: aspect damage (front, rear, side), a
// missile outrun on afterburner at the end of its motor's lifetime, and
// near-field splash through a flare: `crates/sim/tests/protection.rs` holds
// the last natively.
import { gpuWarnings } from "./_lab.mjs";
import { openAir, play, own, shoot } from "./_air.mjs";
import { hull, unitType } from "./_units.mjs";

const HELI = "ah_64e_guardian";
const JET = "f_16c_block_50";
const FLARES = unitType(JET).capabilities.air.flares.capacity;

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let heliHit = null;
  let heliLost = null;
  let flare = null;
  let afterFlare = null;
  await play(page, 900, (o) => {
    const heli = own(o, HELI)[0];
    const jet = own(o, JET)[0];
    if (heli && heli.hp < hull(HELI).hp && !heliHit)
      heliHit = { tick: o.tick, flares: heli.flares };
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
  ctx.check(
    "the Apache, which carries no flares, is hit and lost",
    !!heliHit && heliHit.flares === null && heliLost !== null,
    JSON.stringify({ heliHit, heliLost }),
  );
  ctx.check(
    "the jet spends one flare when the decoyable missile locks, and its reload runs",
    !!flare && flare.charges === FLARES - 1 && flare.cooldown !== null,
    JSON.stringify(flare),
  );
  ctx.check(
    "the diverted missile leaves the jet unhurt",
    !!afterFlare && afterFlare.hp === flare.hp && flare.hp === hull(JET).hp,
    JSON.stringify(afterFlare),
  );
  await shoot(ctx, page, "survivability");
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
