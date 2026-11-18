// Emergency airlift: keep a trace of the AI's purchase, landing, service and return.
import { obs, advance, openBattle, snapshot } from "./_lab.mjs";

const xy = (u) => [u.position[0], u.position[1]];
const heli = (o) => o.own.find((u) => u.kind === "test_supply_heli");
const finiteAmmo = (u) =>
  u?.mounts
    .flatMap((m) => m.ammo)
    .filter((n) => n !== null)
    .reduce((sum, n) => sum + n, 0) ?? 0;

export async function run(ctx) {
  const page = await openBattle(ctx);
  const trace = [];
  let purchased = false;
  let landed = false;
  let served = false;
  let returning = false;
  let baselineAmmo = null;
  let baselineHp = null;
  let landedShot = false;
  let returningShot = false;
  for (let waited = 0; waited < 240 * 30; waited += 5) {
    await advance(page, 5);
    const o = await obs(page);
    const h = heli(o);
    const tanks = o.own.filter((u) => u.kind === "test_tank");
    const tank = tanks.sort((a, b) => a.hp - b.hp)[0];
    if (baselineAmmo === null && tank) baselineAmmo = finiteAmmo(tank);
    if (baselineHp === null && tank) baselineHp = tank.hp;
    if (h) {
      purchased = true;
      if (!landed && h.grounded && tank && h.position[0] < tank.position[0]) {
        landed = true;
        landedShot = true;
        await snapshot(ctx, page, "ai-landed.png");
      }
      if (!returning && landed && h.goal !== null && h.goal[0] < 900) {
        returning = true;
        returningShot = true;
        await snapshot(ctx, page, "ai-returning.png");
      }
    }
    served ||=
      (baselineAmmo !== null && tank && finiteAmmo(tank) > baselineAmmo) ||
      (baselineHp !== null && tank && tank.hp > baselineHp);
    trace.push({
      tick: o.tick,
      own: o.own.map((u) => u.kind),
      players: o.skirmish?.players,
      suppliers: o.own
        .filter((u) => u.kind === "test_supply")
        .map((u) => ({ at: xy(u), goal: u.goal, deployment: u.deployment })),
      heli: h && { at: xy(h), goal: h.goal, grounded: h.grounded },
      tanks: tanks.map((u) => ({ at: xy(u), hp: u.hp, ammo: finiteAmmo(u), service: u.service })),
    });
    if (purchased && landed && served && returning) break;
  }
  await ctx.writeEvidence("airlift-trace.json", trace);
  ctx.check("the AI buys a flying supplier", purchased, JSON.stringify(trace.at(-1)));
  ctx.check(
    "the supplier lands on the far side",
    landed && landedShot,
    JSON.stringify(trace.at(-1)),
  );
  ctx.check("the combat group's finite ammo or hp rises", served, JSON.stringify(trace.at(-1)));
  ctx.check(
    "the supplier returns to the support post",
    returning && returningShot,
    JSON.stringify(trace.at(-1)),
  );
  await page.close();
}
