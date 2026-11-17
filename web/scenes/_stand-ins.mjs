// A growth's stand-ins, judged while its art is held at the network edge:
// the battle draws one physical block per seen body of the new kind, the
// blocks take attack clicks, and the released art replaces them in the same
// session. Rust (crates/presentation/tests/stand_in.rs) owns the block
// rules for every body type; this is the live browser path.
import { writeFile } from "node:fs/promises";
import { lab } from "./_lab.mjs";

const STAND_IN = "__battle_stand_in__";

/** Holds every art response (`bundle.bin`) while `hold` is on. */
export async function holdArt(page) {
  const held = [];
  let hold = false;
  await page.route("**/bundle.bin", async (route) => {
    if (!hold) return route.continue();
    held.push(route);
  });
  return {
    held,
    start: () => {
      hold = true;
    },
    release: async () => {
      hold = false;
      for (const route of held.splice(0)) await route.continue().catch(() => {});
    },
  };
}

const bodies = (unit) => (unit.members.length ? unit.members : [unit.position]);

function samePoint(a, b) {
  return Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]) < 0.01;
}

function bodyPoint(unit) {
  // An outer soldier is clear of the carrier his squad may be beside.
  return (
    [...unit.members].sort(
      (a, b) =>
        Math.hypot(b[0] - unit.position[0], b[1] - unit.position[1]) -
        Math.hypot(a[0] - unit.position[0], a[1] - unit.position[1]),
    )[0] ?? unit.position
  );
}

async function clickUnit(page, unit, button = "left") {
  const point = bodyPoint(unit);
  const css = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1), point);
  if (!css) throw new Error(`unit ${unit.id} is outside the camera`);
  await page.mouse.click(css[0], css[1], { button });
}

/** Frame every body of `unit`, so a check that counts them loses none to the
 *  edge of the picture when a squad stands spread out. */
async function frameAll(page, unit) {
  const all = bodies(unit);
  const centre = [0, 1, 2].map((i) => all.reduce((sum, b) => sum + b[i], 0) / all.length);
  const reach = Math.max(...all.map((b) => Math.hypot(b[0] - centre[0], b[1] - centre[1])));
  await page.evaluate(
    ({ centre, distance }) =>
      window.__lab.setCamera({ ...window.__lab.camera(), target: centre, distance, pitch: 0.55 }),
    { centre, distance: 22 + 3 * reach },
  );
  await page.evaluate(() => window.__lab.frame());
}

/** Frame the point a click lands on, so the army HUD cannot intercept it. */
async function frameOn(page, unit) {
  await page.evaluate(
    (target) =>
      window.__lab.setCamera({ ...window.__lab.camera(), target, distance: 22, pitch: 0.55 }),
    bodyPoint(unit),
  );
  await page.evaluate(() => window.__lab.frame());
}

const drawnStandIns = (page) =>
  lab(
    page,
    (name) => window.__lab.route.drawnModels().filter((m) => m.appearance === name),
    STAND_IN,
  );

/** The identified enemy unit the held growth draws as blocks, or null. The
 *  battle draws only what the camera sees, so each candidate is framed. */
export async function heldUnit(page, observation) {
  for (const unit of observation.identified) {
    await frameAll(page, unit);
    const blocks = await drawnStandIns(page);
    if (bodies(unit).every((body) => blocks.some((m) => samePoint(m.placement, body)))) return unit;
  }
  return null;
}

/** Checks `unit`'s blocks, attacks them with `attacker`, then releases the
 *  held art through `release` and checks the models take the blocks' places.
 *  `committed()` says whether the growth's art has committed. */
export async function judgeStandIns(ctx, page, { unit, attacker, release, committed }) {
  const label = unit.members.length ? "squad" : "vehicle";
  const meshHalf = await page.evaluate(async () => {
    const { BATTLE_STAND_IN_HALF_EXTENTS } = await import("/src/battle/render/proxyModels.ts");
    return BATTLE_STAND_IN_HALF_EXTENTS;
  });
  await frameAll(page, unit);
  const models = await drawnStandIns(page);
  const picks = await lab(page, () => window.__lab.instances());
  ctx.check(
    `the held ${label} draws one stand-in per seen body at its physical size`,
    bodies(unit).every((position) => {
      const matching = models.filter((m) => samePoint(m.placement, position));
      const pick = picks.find((p) => samePoint([p.x, p.y, p.z], position));
      return (
        matching.length === 1 &&
        pick &&
        pick.half.every((half, i) => Math.abs(matching[0].scale[i] * meshHalf[i] - half) < 0.001)
      );
    }),
    JSON.stringify({ unit, models, picks }),
  );
  if (!unit.members.length) {
    const model = models.find((m) => samePoint(m.placement, unit.position));
    ctx.check(
      "the held vehicle's block follows the hull yaw",
      !!model && Math.abs(model.placement[3] - unit.yaw) < 0.001,
    );
  }
  await writeFile(ctx.evidencePath(`${label}-held.png`), await page.screenshot());

  await frameOn(page, attacker);
  await clickUnit(page, attacker);
  await page.waitForFunction((id) => window.__lab.route.selected().includes(id), attacker.id);
  const acks = await lab(page, () => window.__lab.route.acks().length);
  await frameOn(page, unit);
  await clickUnit(page, unit, "right");
  await page.waitForFunction((n) => window.__lab.route.acks().length > n, acks);
  const attack = await lab(page, () => window.__lab.route.acks()[0]);
  ctx.check(
    `a click on the held ${label}'s block is an attack target`,
    attack?.order.kind === "attack" && attack.order.target.id === unit.id && !attack.ack.error,
    JSON.stringify(attack),
  );

  await release();
  const deadline = Date.now() + 120_000;
  while (Date.now() < deadline) {
    await page.evaluate(() => window.__lab.frame());
    if (committed() && (await drawnStandIns(page)).length === 0) break;
    await page.waitForTimeout(250);
  }
  await frameAll(page, unit);
  const after = await lab(page, () => window.__lab.route.drawnModels());
  ctx.check(
    `the ${label}'s models replace its blocks at the same body positions`,
    committed() &&
      bodies(unit).every((position) =>
        after.some((m) => m.appearance !== STAND_IN && samePoint(m.placement, position)),
      ),
    JSON.stringify({ unit, models: after }),
  );
  ctx.check(
    "no instance uses the stand-in appearance after the swap",
    after.every((m) => m.appearance !== STAND_IN),
    JSON.stringify(after.filter((m) => m.appearance === STAND_IN)),
  );
  await writeFile(ctx.evidencePath(`${label}-after.png`), await page.screenshot());
}
