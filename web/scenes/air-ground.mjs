// The large air-to-ground run on the 3 km air-jet field: an F-16 (gun and
// ground missile) and an F-15E (gun and bombs) attack ground points, and an EA-18G attacks the
// Buk's radar contact.
// The simulation's tests behind it:
//   crates/sim/tests/attack_passes.rs::a_bombing_pass_lines_up_dives_under_its_ceiling_releases_and_climbs_back
//   crates/sim/tests/attack_passes.rs::a_missile_pass_fires_the_gun_when_nose_on
//   crates/sim/tests/attack_passes.rs::a_bombing_pass_never_fires_the_gun
//   crates/sim/tests/radar.rs::anti_radiation_weapon_can_fire_at_radar_contact_without_unit_los
//   crates/sim/tests/air.rs::an_attack_on_a_target_the_side_has_never_identified_is_refused
//   crates/sim/tests/air.rs::high_air_has_no_ground_vision_but_can_identify_low_air
// Browser-only evidence: with the roster's real loadouts on the real field,
// the bomber releases its bombs, the missile jet fires its gun on its pass
// (its guided missile has no unit to lock on a ground point, whose tank high
// air never identifies), and the SEAD jet's order on the radar contact is accepted and it
// fires; the height trace, the passes strip and the full-field frame with
// the jet-only fog are for review.
import { gpuWarnings, obs } from "./_lab.mjs";
import { encounter, openAir, play, own, command, shoot, strip, trace, shots } from "./_air.mjs";
import { unitType } from "./_units.mjs";

const [MISSILE_JET, BOMBER, SEAD] = encounter("ground", "air-ground").units.map((u) => u.kind);

/** Keep in `seen` the most rounds `kind`'s mount `id` has launched so far. */
function count(seen, o, kind, id) {
  const mount = unitType(kind).mounts.findIndex((m) => m.id === id);
  const u = own(o, kind)[0];
  const key = `${kind}/${id}`;
  if (u) seen[key] = Math.max(seen[key] ?? 0, shots(u, mount));
}

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let radar = null;
  await play(page, 180, (o) => {
    radar = o.contacts.find((c) => c.source === "radar") ?? null;
    return !!radar;
  });
  const sead = own(await obs(page), SEAD)[0];
  ctx.check(
    "the SEAD jet has a radar contact to attack",
    !!sead && !!radar,
    JSON.stringify({ sead: sead?.id, radar }),
  );
  const ack =
    sead && radar
      ? await command(page, {
          kind: "attack",
          units: [sead.id],
          target: { kind: "contact", id: radar.id },
        })
      : null;
  ctx.check(
    "the attack on the radar contact is accepted",
    ack?.error === null,
    JSON.stringify(ack),
  );

  const seen = {};
  await play(page, 900, (o) => {
    count(seen, o, BOMBER, "bombs");
    count(seen, o, MISSILE_JET, "gun");
    count(seen, o, SEAD, "missiles");
    return false;
  });
  const fired = (kind, id) => seen[`${kind}/${id}`] ?? 0;
  ctx.check("the bomber releases its bombs", fired(BOMBER, "bombs") > 0, JSON.stringify(seen));
  ctx.check(
    "the missile jet fires its gun on its pass",
    fired(MISSILE_JET, "gun") > 0,
    JSON.stringify(seen),
  );
  ctx.check(
    "the SEAD jet fires at the radar contact",
    fired(SEAD, "missiles") > 0,
    JSON.stringify(seen),
  );
  await trace(page, 360, [0, 1, 2], ctx.evidencePath("air-ground-height-trace.png"));
  await strip(page, 180, ctx.evidencePath("air-ground-passes.gif"));
  await shoot(ctx, page, "air-ground");
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
