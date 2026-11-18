// Decoyable proximity-fused air missiles on air-jet: dedicated launchers
// engage a flared F-16 and an Apache without flares, so target priority cannot
// leave either engagement unexercised.
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
import { advance, gpuWarnings, lab, obs, openBattle } from "./_lab.mjs";
import { own, shoot, visibleCells } from "./_air.mjs";
import { rules, unitType } from "./_units.mjs";

const HELI = "ah_64e_guardian";
const JET = "f_16c_block_50";
const FLARES = unitType(JET).capabilities.flares.capacity;

export async function run(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 } });
  const warnings = gpuWarnings(page);
  let flared = null;
  let heliLost = null;
  const moments = [];
  // Fixed ticks keep before/after flight pictures comparable even when the
  // changed guidance makes a different shot bring down the helicopter.
  for (let t = 0; t < 1500; t++) {
    await advance(page, 1);
    const o = await obs(page);
    const jet = own(o, JET)[0];
    if (!own(o, HELI)[0] && heliLost === null) heliLost = o.tick;
    if (jet && jet.flares.charges < FLARES && flared === null) flared = o.tick;
    if ([90, 300, 600].includes(o.tick)) {
      moments.push({
        tick: o.tick,
        projectiles: o.projectiles,
        blasts: o.blasts,
        guided: o.guided,
      });
      await shoot(ctx, page, `flight-${o.tick}-wide`);
      const camera = await lab(page, () => window.__lab.camera());
      await lab(
        page,
        (camera) =>
          window.__lab.setCamera({
            ...camera,
            target: [1700, 1500, 30],
            distance: 1300,
            pitch: 0.9076,
            yaw: 3.752,
          }),
        camera,
      );
      await shoot(ctx, page, `flight-${o.tick}-reference-zoom`);
      await lab(page, (camera) => window.__lab.setCamera(camera), camera);
    }
    if (o.tick >= 600 && flared !== null && heliLost !== null && o.tick >= flared + 60) break;
  }
  await ctx.writeEvidence("projectile-moments.json", moments);
  ctx.check(
    "the F-16 spends a flare and the Apache is shot down",
    flared !== null && heliLost !== null,
    JSON.stringify({ flared, heliLost }),
  );
  await shoot(ctx, page, "survivability");
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
  await page.close();
  await unseenGroundBurst(ctx);
}

async function unseenGroundBurst(ctx) {
  const url = new URL(ctx.url);
  url.searchParams.set("variant", "visibility");
  const page = await openBattle(ctx, { url: url.href, viewport: { width: 1920, height: 1080 } });
  const warnings = gpuWarnings(page);
  const bombs = new Set(
    Object.entries(rules.weapons)
      .filter(([, w]) => w.delivery === "unpowered")
      .map(([id]) => id),
  );
  let flightSeen = false;
  let fog = 0;
  let impact = null;
  for (let t = 0; t < 2400; t++) {
    await advance(page, 1);
    const o = await obs(page);
    fog = Math.max(fog, visibleCells(o));
    flightSeen ||= o.projectiles.some((p) => !p.own && bombs.has(p.kind));
    const burst = o.blasts.find((b) => bombs.has(b.kind));
    if (burst) {
      impact = { tick: o.tick, burst };
      break;
    }
  }
  ctx.check(
    "air sight sees an enemy bomb and its ground burst",
    flightSeen && impact !== null,
    JSON.stringify({ flightSeen, impact }),
  );
  ctx.check("seeing the bomb and burst grants no ground vision", fog === 0, `${fog} cells`);
  await ctx.writeEvidence("air-ground-burst.json", { flightSeen, impact, fog });
  if (impact) {
    await advance(page, 4);
    await shoot(ctx, page, "air-ground-burst-wide");
    await lab(
      page,
      (point) =>
        window.__lab.setCamera({
          ...window.__lab.camera(),
          target: point,
          distance: 1300,
          pitch: 0.9076,
          yaw: 3.752,
        }),
      impact.burst.point,
    );
    await shoot(ctx, page, "air-ground-burst-reference-zoom");
    await lab(page, () => window.__lab.setCamera({ ...window.__lab.camera(), distance: 350 }));
    await shoot(ctx, page, "air-ground-burst-close");
  }
  ctx.check(
    "air-ground burst has no GPU validation warnings",
    warnings.length === 0,
    warnings.join("\n"),
  );
  await page.close();
}
