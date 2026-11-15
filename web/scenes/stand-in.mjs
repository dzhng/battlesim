// STAND_IN_LOOK_EVIDENCE=<throwaway directory> also captures matched near/far
// open and native-fog views for the stand-in look review.
// Slice 10: hold the growth admission's runtime responses at the browser
// network edge, inspect the live stand-ins, then release those same responses.
// This growth battle has no own unfielded kind to select; Rust proves identical
// picks before and after art commits. The browser gate checks drawn blocks and
// targets the newly identified vehicle and squad. A nearby known wreck uses
// the static prop stand-in kit while its wreck art is held.
import { writeFile } from "node:fs/promises";
import { lab, presented } from "./_lab.mjs";
import { meetStandIns } from "./_stand-in-battle.mjs";
import { captureLook, vehicleLook } from "./_stand-in-look.mjs";

async function clickUnit(page, unit, button = "left") {
  // An outer soldier is clear of the carrier his squad may be beside.
  const point =
    [...unit.members].sort(
      (a, b) =>
        Math.hypot(b[0] - unit.position[0], b[1] - unit.position[1]) -
        Math.hypot(a[0] - unit.position[0], a[1] - unit.position[1]),
    )[0] ?? unit.position;
  const css = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1), point);
  if (!css) throw new Error(`unit ${unit.id} is outside the camera`);
  await page.mouse.click(css[0], css[1], { button });
}

async function frameOn(page, unit) {
  await page.evaluate(
    ({ position }) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        target: position,
        distance: 22,
        pitch: 0.55,
      }),
    unit,
  );
  await page.evaluate(() => window.__lab.frame());
}

const drawnStandIns = (page) =>
  lab(page, () =>
    window.__lab.route.drawnModels().filter((m) => m.appearance === "__battle_stand_in__"),
  );

async function checkBlocks(ctx, page, unit, meshHalf, label) {
  await frameOn(page, unit);
  const models = await drawnStandIns(page);
  const picks = await lab(page, () => window.__lab.instances());
  const positions = unit.members.length ? unit.members : [unit.position];
  await ctx.writeEvidence(`${label.toLowerCase().replaceAll(" ", "-")}-bodies.json`, {
    unit,
    models,
    picks,
    nearbyModels: (await lab(page, () => window.__lab.route.drawnModels())).filter(
      (model) =>
        Math.hypot(model.placement[0] - unit.position[0], model.placement[1] - unit.position[1]) <
        25,
    ),
    nearbyStructures: (await lab(page, () => window.__lab.route.structures())).filter(
      (model) =>
        Math.hypot(model.position[0] - unit.position[0], model.position[1] - unit.position[1]) < 25,
    ),
  });
  ctx.check(
    `${label} draws one stand-in per seen body at its physical size`,
    positions.every((position) => {
      const matching = models.filter((m) => samePoint(m.placement, position));
      const pick = picks.find((p) => samePoint([p.x, p.y, p.z], position));
      return (
        matching.length === 1 &&
        pick &&
        pick.half.every((half, i) => Math.abs(matching[0].scale[i] * meshHalf[i] - half) < 0.001)
      );
    }),
    JSON.stringify({ positions, models, picks }),
  );
  if (!unit.members.length) {
    const model = models.find((m) => samePoint(m.placement, unit.position));
    ctx.check(
      "the vehicle block follows the hull yaw",
      !!model && Math.abs(model.placement[3] - unit.yaw) < 0.001,
    );
  }
}

function samePoint(a, b) {
  return Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]) < 0.01;
}

export async function run(ctx) {
  const battle = await meetStandIns(ctx);
  const { page, held, timeline, heldObservation, vehicle, squad } = battle;
  const look = process.env.STAND_IN_LOOK_EVIDENCE;
  try {
    ctx.check(
      "the held network responses leave a seen vehicle and squad",
      held.length > 0 && !!vehicle && !!squad,
      JSON.stringify({ held: held.length, identified: heldObservation.identified }),
    );
    await ctx.writeEvidence("meeting.json", heldObservation);
    if (!vehicle || !squad) throw new Error("the scout did not meet both body types");
    await presented(page);
    const meshHalf = await page.evaluate(async () => {
      const { BATTLE_STAND_IN_HALF_EXTENTS } = await import("/src/battle/render/proxyModels.ts");
      return BATTLE_STAND_IN_HALF_EXTENTS;
    });
    await checkBlocks(ctx, page, vehicle, meshHalf, "The vehicle");
    await writeFile(ctx.evidencePath("vehicle-before.png"), await page.screenshot());
    if (look) await captureLook(page, vehicle, "vehicle", "held", look, [["open", false]]);
    await checkBlocks(ctx, page, squad, meshHalf, "The squad");
    await writeFile(ctx.evidencePath("squad-before.png"), await page.screenshot());

    if (look) await captureLook(page, squad, "squad", "held", look);

    const ownVehicle = heldObservation.own.find((unit) => unit.members.length === 0);
    if (!ownVehicle) throw new Error("the scout was lost before targeting");
    await frameOn(page, ownVehicle);
    await clickUnit(page, ownVehicle);
    await page.waitForFunction((id) => window.__lab.route.selected().includes(id), ownVehicle.id);

    const beforeAcks = await lab(page, () => window.__lab.route.acks().length);
    await frameOn(page, vehicle);
    await clickUnit(page, vehicle, "right");
    await page.waitForFunction((n) => window.__lab.route.acks().length > n, beforeAcks);
    const vehicleAttack = await lab(page, () => window.__lab.route.acks()[0]);
    ctx.check(
      "a click on the withheld vehicle is an attack target",
      vehicleAttack?.order.kind === "attack" &&
        vehicleAttack.order.target.id === vehicle.id &&
        !vehicleAttack.ack.error,
      JSON.stringify(vehicleAttack),
    );

    const squadAcks = await lab(page, () => window.__lab.route.acks().length);
    await frameOn(page, squad);
    await clickUnit(page, squad, "right");
    await page.waitForFunction((n) => window.__lab.route.acks().length > n, squadAcks);
    const squadAttack = await lab(page, () => window.__lab.route.acks()[0]);
    ctx.check(
      "a click on the withheld squad is an attack target",
      squadAttack?.order.kind === "attack" &&
        squadAttack.order.target.id === squad.id &&
        !squadAttack.ack.error,
      JSON.stringify(squadAttack),
    );

    await battle.release();
    const committedAt = Date.now();
    while (Date.now() - committedAt < 120_000) {
      await page.evaluate(() => window.__lab.frame());
      if (timeline.includes("art committed") && (await drawnStandIns(page)).length === 0) break;
      await page.waitForTimeout(250);
    }
    ctx.check(
      "releasing the responses commits art in the same session",
      timeline.includes("art committed"),
      JSON.stringify(timeline),
    );
    const remaining = [];
    for (const [label, unit] of [
      ["vehicle", vehicle],
      ["squad", squad],
    ]) {
      await frameOn(page, unit);
      remaining.push(...(await drawnStandIns(page)));
      const models = await lab(page, () => window.__lab.route.drawnModels());
      const positions = unit.members.length ? unit.members : [unit.position];
      ctx.check(
        `the ${label}'s models replace its blocks at the same body positions`,
        positions.every((position) =>
          models.some(
            (model) =>
              model.appearance !== "__battle_stand_in__" && samePoint(model.placement, position),
          ),
        ),
        JSON.stringify({ positions, models }),
      );
      await ctx.writeEvidence(`${label}-after-models.json`, {
        unit,
        nearbyModels: models.filter(
          (model) =>
            Math.hypot(
              model.placement[0] - unit.position[0],
              model.placement[1] - unit.position[1],
            ) < 25,
        ),
        nearbyStructures: (await lab(page, () => window.__lab.route.structures())).filter(
          (model) =>
            Math.hypot(model.position[0] - unit.position[0], model.position[1] - unit.position[1]) <
            25,
        ),
      });
      await writeFile(ctx.evidencePath(`${label}-after.png`), await page.screenshot());
    }
    ctx.check(
      "no instance uses the stand-in appearance after the swap",
      remaining.length === 0,
      JSON.stringify(remaining),
    );
    if (look) {
      await captureLook(page, vehicle, "vehicle", "model", look, [["open", false]]);
      await captureLook(page, squad, "squad", "model", look);
    }
  } finally {
    await battle.close();
  }
  if (look) await vehicleLook(ctx, look);
}
