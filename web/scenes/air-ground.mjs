// Air-to-ground orders (slice 07). Attack-ground orders are scripted at
// tick 5: the US Fighter, carrying bombs, bombs the T-72 it cannot see;
// the F-16, carrying none, strafes the other with its gun. The scene orders
// the EA-18G to attack the Buk's radar contact, and it launches its
// anti-radiation missile; an attack on a target blue has never seen is
// refused. Blue's fog never clears and it never identifies the tanks.
//
// Not shown, because the simulation does not do it: a bombing or strafing
// *run*. The aircraft release from where they hover; nothing flies over the
// target. The SEAD missile flies to the radar contact's approximate centre,
// so it can miss the emitter it was fired at.
import { gpuWarnings, obs } from "./_lab.mjs";
import { encounter, openAir, play, own, command, shoot, visibleCells } from "./_air.mjs";
import { unitType } from "./_units.mjs";

const units = encounter("ground").units;
const [BOMBER, STRAFER, SEAD, TANK] = units.map((u) => u.kind);
const bombTarget = encounter("ground").scripts[0].order.target.point;

/** The mount carrying a weapon whose row is a `ground_attack` of `kind`. */
const mountOf = (kind, weapon) =>
  unitType(kind).mounts.findIndex((m) => m.weapons.includes(weapon));

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let radar = null;
  await play(page, 60, (o) => {
    radar = o.contacts.find((c) => c.source === "radar") ?? null;
    return !!radar;
  });
  ctx.check("the SEAD jet has a radar contact to attack", !!radar);
  const sead = own(await obs(page), SEAD)[0];
  const ordered = radar
    ? await command(page, {
        kind: "attack",
        units: [sead.id],
        target: { kind: "contact", id: radar.id },
      })
    : null;
  const refused = await command(page, {
    kind: "attack",
    units: [sead.id],
    target: { kind: "identified", id: 9999 },
  });
  ctx.check(
    "the attack on the radar contact is accepted",
    ordered?.error === null,
    JSON.stringify(ordered),
  );
  ctx.check(
    "an attack on a target blue has never seen is refused",
    refused?.error?.reason === "unknown_target",
    JSON.stringify(refused),
  );

  let fog = 0;
  let tanks = false;
  let guided = false;
  let blastNear = null;
  let peak = {};
  await play(page, 360, (o) => {
    fog = Math.max(fog, visibleCells(o));
    if (o.identified.some((u) => u.kind === TANK)) tanks = true;
    if (o.guided.length) guided = true;
    blastNear ??=
      o.blasts.find(
        (b) => Math.hypot(b.point[0] - bombTarget[0], b.point[1] - bombTarget[1]) < 40,
      ) ?? null;
    for (const u of o.own)
      peak[u.kind] = u.weaponPoses.map((p, i) => Math.max(peak[u.kind]?.[i] ?? 0, p.shots));
    return false;
  });
  const bombs = mountOf(BOMBER, "air_bomb_guided");
  ctx.check(
    "the bomber drops bombs on its ground point",
    bombs >= 0 && (peak[BOMBER]?.[bombs] ?? 0) > 0 && !!blastNear,
    JSON.stringify({ shots: peak[BOMBER], blastNear }),
  );
  const gun = mountOf(STRAFER, "air_gun");
  const missiles = mountOf(STRAFER, "air_missile");
  ctx.check(
    "the F-16 without bombs strafes with its gun, never its air missiles",
    (peak[STRAFER]?.[gun] ?? 0) > 0 && (peak[STRAFER]?.[missiles] ?? 0) === 0,
    JSON.stringify(peak[STRAFER]),
  );
  const arm = mountOf(SEAD, "sead_missile");
  ctx.check(
    "the SEAD jet launches its anti-radiation missile at the radar contact",
    guided && (peak[SEAD]?.[arm] ?? 0) > 0,
    JSON.stringify({ guided, shots: peak[SEAD] }),
  );
  ctx.check("no ground vision: blue's fog never clears", fog === 0, `${fog}`);
  ctx.check("no ground vision: the tanks are never identified", !tanks);
  await shoot(ctx, page, "air-ground");
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
