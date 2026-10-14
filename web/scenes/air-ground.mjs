// AWF-JETSCENES — the large air-to-ground run on the 3 km jet field.
// Simulation contracts named here are already pinned by:
//   crates/sim/tests/jet_runs.rs::a_bombing_pass_lines_up_dives_under_its_ceiling_releases_and_climbs_back
//   crates/sim/tests/jet_runs.rs::a_missile_pass_fires_the_gun_when_nose_on
//   crates/sim/tests/jet_runs.rs::a_bombing_pass_never_fires_the_gun
//   crates/sim/tests/air.rs::an_attack_on_a_target_the_side_has_never_identified_is_refused
//   crates/sim/tests/air.rs::high_air_has_no_ground_vision_but_can_identify_low_air
// Browser-only evidence: the bomber's moving pass and near-target blast, the
// missile/gun jet's nose-on gun flash, and a full-field frame with the SEAD
// contact staged beside the jet-only fog.
import { gpuWarnings, obs } from "./_lab.mjs";
import { encounter, openAir, play, own, command, shoot, strip, trace } from "./_air.mjs";
const SEAD = encounter("ground", "air-ground").units[2].kind;

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let radar = null;
  await play(page, 180, (o) => {
    radar = o.contacts.find((c) => c.source === "radar") ?? null;
    return !!radar;
  });
  const sead = own(await obs(page), SEAD)[0];
  if (sead && radar) await command(page, { kind: "attack", units: [sead.id], target: { kind: "contact", id: radar.id } });
  if (sead) await command(page, { kind: "attack", units: [sead.id], target: { kind: "identified", id: 9999 } });

  // The sim tests own release, nose-on fire, SEAD guidance, refusal and fog.
  // The browser contract is the pass evidence: moving aircraft, the release
  // effect, and the SEAD contact order all share one long run.
  await play(page, 900, () => false);
  await trace(page, 360, [0, 1, 2], ctx.evidencePath("air-ground-height-trace.png"));
  await strip(page, 180, ctx.evidencePath("air-ground-passes.gif"));
  await shoot(ctx, page, "air-ground");
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
