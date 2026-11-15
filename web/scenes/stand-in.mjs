// Slice 10: withheld vehicle and squad art remain visible, selectable and
// targetable as physical blocks, then swap to models after release.
import { openBattle, obs, route, snapshot, lab } from "./_lab.mjs";

const clickUnit = async (page, unit, button = "left") => {
  const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1), unit.position);
  await page.mouse.click(at[0], at[1], { button });
};

export async function run(ctx) {
  const page = await openBattle(ctx, { timeout: 60000 });
  await page.evaluate(() => window.__lab.frame());
  const before = await obs(page);
  const tank = before.own.find((u) => u.kind === "test_tank");
  const squad = before.own.find((u) => u.kind === "test_rifle");
  const enemy = before.identified.find((u) => u.kind === "test_tank");
  ctx.check(
    "the withheld vehicle and squad are observed",
    !!tank && !!squad,
    JSON.stringify(before),
  );
  ctx.check("the withheld enemy is targetable", !!enemy, JSON.stringify(before.identified));
  if (!tank || !squad || !enemy) return;

  await snapshot(ctx, page, "stand-in-before.png");
  await clickUnit(page, tank);
  await page.waitForFunction((id) => window.__lab.route.selected().includes(id), tank.id);
  ctx.check("a withheld vehicle block selects", (await route(page, "selected")).includes(tank.id));
  await clickUnit(page, squad);
  await page.waitForFunction((id) => window.__lab.route.selected().includes(id), squad.id);
  ctx.check("a withheld squad block selects", (await route(page, "selected")).includes(squad.id));
  await clickUnit(page, enemy, "right");
  await page.waitForFunction(() => window.__lab.route.acks().length > 0, undefined, {
    timeout: 5000,
  });
  ctx.check(
    "a right-click on a withheld enemy is an attack target",
    (await route(page, "acks")).length > 0,
  );

  await page.goto(new URL("/labs/stand-in?release=1", page.url()).toString());
  await page.waitForFunction(() => window.__lab?.route?.tick?.() > 3, undefined, {
    timeout: 60000,
  });
  await page.evaluate(() => window.__lab.frame());
  const releasedModels = await route(page, "vehicles");
  ctx.check(
    "released art replaces the stand-in vehicles",
    releasedModels.some((v) => v.appearance === "test_tank"),
    JSON.stringify(releasedModels),
  );
  await snapshot(ctx, page, "stand-in-after.png");
}
