// Layered air defence (slice 03), from red's side. Helicopters (LowAir):
// the medium Buk and the short gun engage them, the long S-300, whose rows
// target only HighAir, never fires. A jet (HighAir): all three tiers fire,
// and the short gun the tower hides from it never does, for want of its
// own line of sight. The simulation's test behind this:
//   crates/sim/tests/air.rs::air_defence_tiers_follow_the_target_layer_and_need_their_own_sight
//
// Not shown here, held by the simulation's tests instead: interception of an
// incoming missile (crates/sim/tests/distances_3d.rs::an_interceptor_engages_an_incoming_missile_at_any_height),
// seeker turn and lifetime misses (guidance.rs::steering_is_bounded_by_the_turn_rate,
// air_missiles.rs::an_air_missile_ends_at_its_lifetime) and the proximity
// burst (air_missiles.rs::a_missile_bursts_within_its_blast_radius_of_a_passing_jet).
import { gpuWarnings } from "./_lab.mjs";
import { encounter, openAir, play, shots, shoot } from "./_air.mjs";

const SHORT = "us_stryker_m_shorad_gun_and_missile_air_defense_vehicle";
const MEDIUM = "eastern_buk_buk_m3_medium_range_air_defense";
const LONG = "eastern_s300_long";

/** Red's AA by where the encounter put it. */
const at = (o, p) => o.own.find((u) => Math.hypot(u.position[0] - p[0], u.position[1] - p[1]) < 1);

async function engagement(ctx, variant, ticks) {
  const units = encounter(`defense-${variant}`).units.filter((u) => u.side === "red");
  const page = await openAir(ctx, variant);
  const warnings = gpuWarnings(page);
  const fired = new Map();
  const reasons = new Map();
  await play(page, ticks, (o) => {
    for (const u of units) {
      const mine = at(o, u.position);
      if (!mine) continue;
      const key = `${u.kind}@${u.position.join(",")}`;
      fired.set(key, Math.max(fired.get(key) ?? 0, shots(mine)));
      reasons.set(key, new Set([...(reasons.get(key) ?? []), mine.mounts[0].reason]));
    }
    return false;
  });
  await shoot(ctx, page, `defense-${variant}`);
  await page.close();
  ctx.check(`${variant}: no GPU validation warnings`, warnings.length === 0, warnings.join("\n"));
  const of = (kind, n = 0) => {
    const u = units.filter((u) => u.kind === kind)[n];
    const key = `${u.kind}@${u.position.join(",")}`;
    return { shots: fired.get(key) ?? 0, reasons: [...(reasons.get(key) ?? [])] };
  };
  return of;
}

export async function run(ctx) {
  const heli = await engagement(ctx, "helicopter", 300);
  ctx.check(
    "helicopters: the Buk fires at them",
    heli(MEDIUM).shots > 0,
    JSON.stringify(heli(MEDIUM)),
  );
  ctx.check(
    "helicopters: the short gun fires at them",
    heli(SHORT).shots > 0,
    JSON.stringify(heli(SHORT)),
  );
  ctx.check(
    "helicopters: the long SAM never fires at LowAir",
    heli(LONG).shots === 0,
    JSON.stringify(heli(LONG)),
  );

  const jet = await engagement(ctx, "jet", 300);
  for (const kind of [SHORT, MEDIUM, LONG])
    ctx.check(`jet: the ${kind} fires at it`, jet(kind).shots > 0, JSON.stringify(jet(kind)));
  const hidden = jet(SHORT, 1);
  ctx.check(
    "jet: the short gun behind the tower never fires, lacking its own sight",
    hidden.shots === 0 && hidden.reasons.includes("no_own_sight"),
    JSON.stringify(hidden),
  );
}
