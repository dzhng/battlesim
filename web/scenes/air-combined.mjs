// The combined air battle (slice 11's closeout battle) on the production
// battle view, from blue's side: an EA-18G, an F-16 and a bomber against
// short, medium and long air defence, a J-10, a Hind and a T-72. Within
// its first twenty seconds blue's observation holds every air-war
// mechanism at once: radar contacts, a SEAD launch, a dogfight, a bomb
// released at the tank's ground point, a flare spent, and aircraft lost to
// the air defence.
//
// Not in this battle: paid ingress, fuel withdrawal and refunds (a
// skirmish begins empty, so it cannot hold this fixed battle; the
// air-ingress checkpoint shows them) and the air alarm (air-audio's).
import { gpuWarnings } from "./_lab.mjs";
import { encounter, openAir, play, shots, shoot } from "./_air.mjs";
import { unitType } from "./_units.mjs";

const blue = encounter("combined").units.filter((u) => u.side === "blue");
const BOMBER = encounter("combined").scripts[0].order.units[0];
const bombMount = unitType(blue[BOMBER].kind).mounts.findIndex((m) =>
  m.weapons.includes("air_bomb_guided"),
);

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  const seen = { radar: false, guided: false, dogfight: false, bomb: false, flare: false, lost: 0 };
  let frame = null;
  await play(page, 600, (o) => {
    if (o.contacts.some((c) => c.source === "radar")) seen.radar = true;
    if (o.guided.length) seen.guided = true;
    if (o.own.some((u) => u.inDogfight)) {
      seen.dogfight = true;
      frame ??= o.tick;
    }
    const bomber = o.own.find((u) => u.id === BOMBER);
    if (bomber && shots(bomber, bombMount) > 0) seen.bomb = true;
    if (
      o.own.some(
        (u) => u.flares && u.flares.charges < unitType(u.kind).capabilities.air.flares.capacity,
      )
    )
      seen.flare = true;
    seen.lost = blue.length - o.own.length;
    return false;
  });
  ctx.check("radar contacts are published", seen.radar);
  ctx.check("the SEAD jet launches its missile", seen.guided);
  ctx.check("a dogfight forms", seen.dogfight, `${frame}`);
  ctx.check("the bomber releases a bomb", seen.bomb);
  ctx.check("an aircraft spends a flare", seen.flare);
  ctx.check("the air defence brings aircraft down", seen.lost > 0, `${seen.lost}`);
  await shoot(ctx, page, "combined");
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
