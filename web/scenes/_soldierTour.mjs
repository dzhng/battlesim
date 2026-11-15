import { lab, obs, advance, snapshot, openBattle, aim } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";
import { rules } from "./_units.mjs";

/** A squad's framings: the camera's closest zoom, the Defilade default, and
 *  far enough that soldiers are impostor cards. */
const SQUAD_FRAMINGS = {
  ground: { distance: 25, pitch: 0.22 },
  default: { distance: 65, pitch: 0.85 },
  far: { distance: 420, pitch: 0.85 },
};

/** Soldiers as posed models at every framing: tiers and cards, the fog mask,
 *  picking, and infantry beside armour in an advance. `tick` is the battle
 *  state every framing shows. */
export async function soldierTour(ctx, tick) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick });
  const o = await obs(page);
  const squads = o.own.filter((u) => u.members.length > 0);
  const soldiers = squads.reduce((n, u) => n + u.members.length, 0);
  // The squad nearest the opening framing's target.
  const [tx, ty] = [170, 800];
  const squad = squads.reduce((a, b) =>
    Math.hypot(a.position[0] - tx, a.position[1] - ty) <=
    Math.hypot(b.position[0] - tx, b.position[1] - ty)
      ? a
      : b,
  );
  const seen = {};
  for (const [name, framing] of Object.entries(SQUAD_FRAMINGS)) {
    await aim(page, squad.position, { ...framing, yaw: -Math.PI / 2 });
    await snapshot(ctx, page, `soldiers-${name}-1920x1080.png`);
    seen[name] = await lab(page, () => window.__lab.stats().models);
  }
  const meshes = (s) => s.tiers.reduce((a, b) => a + b, 0);
  // Soldiers' own tiers (vehicles and props share the layer).
  const finest = (s) => s.bodyTiers.findIndex((n) => n > 0);
  ctx.check(
    "every drawn soldier is a posed model: finer tiers near, impostor cards far, only meshes posed",
    // Vehicles and props are models too, so soldiers are counted
    // as the posed bodies, their cards and the bodies culled.
    Object.values(seen).every(
      (s) =>
        s.soldierMeshes <= meshes(s) &&
        s.atlasLayers >= 6 &&
        s.soldierMeshes + s.cards + s.culledBodies === soldiers,
    ) &&
      finest(seen.ground) >= 0 &&
      finest(seen.default) > finest(seen.ground) &&
      seen.far.cards === soldiers,
    JSON.stringify({ soldiers, seen }),
  );

  // Units are drawn by identification: never fogged. In the fog mask a
  // soldier is white wherever the camera sees him, his back to every eye included.
  await aim(page, squad.position, { ...SQUAD_FRAMINGS.ground, yaw: -Math.PI / 2 });
  await lab(page, () => window.__lab.setFrameView("fog-mask"));
  const mask = decode(await snapshot(ctx, page, "soldiers-fog-mask-1920x1080.png"));
  await lab(page, () => window.__lab.setFrameView("final"));
  let dark = 0;
  let sampled = 0;
  for (const m of squad.members) {
    const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1.1), m);
    if (!at || at[0] < 0 || at[1] < 0 || at[0] >= 1920 || at[1] >= 1080) continue;
    for (let dy = -4; dy <= 4; dy += 2)
      for (let dx = -3; dx <= 3; dx++) {
        sampled++;
        if (pixel(mask, at[0] + dx, at[1] + dy)[0] < 128) dark++;
      }
  }
  ctx.check(
    "soldiers are never fogged: the fog mask is white across every soldier in view",
    sampled > 50 && dark === 0,
    JSON.stringify({ sampled, dark }),
  );

  // Picking keeps the simulation's boxes: a soldier's torso picks his body's.
  // The frame's pick records are f32, so his half width is the radius's f32.
  await aim(page, squad.position, { ...SQUAD_FRAMINGS.default, yaw: -Math.PI / 2 });
  await page.evaluate(() => window.__lab.frame());
  const m = squad.members[0];
  const torso = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1), m);
  const picked = await lab(page, (p) => window.__lab.pickAt(p[0], p[1]), torso);
  const box = await lab(page, (k) => window.__lab.instances()[k], picked);
  ctx.check(
    "clicking a soldier picks his box",
    picked >= 0 &&
      box.half[0] === Math.fround(rules.physics.soldier_radius_m) &&
      Math.hypot(box.x - m[0], box.y - m[1]) < 1.5,
    JSON.stringify({ picked, box, member: m }),
  );

  // Infantry beside armour during a real advance. The deterministic death
  // lifecycle is exercised by the consequences scene.
  await lab(
    page,
    (o) =>
      window.__lab.route.command({
        kind: "attack_move",
        units: o.own.map((u) => u.id),
        gesture: 1,
        goal: [1000, 800],
      }),
    o,
  );
  // The Defilade framing on the squad nearest a tank (the reference crop's
  // infantry beside armour), with and without the HUD's marks.
  const besideTank = async (name) => {
    const o = await obs(page);
    const tanks = o.own.filter((u) => u.kind === "test_tank");
    const near = (u) =>
      Math.min(
        ...tanks.map((t) =>
          Math.hypot(t.position[0] - u.position[0], t.position[1] - u.position[1]),
        ),
      );
    const squad = o.own.filter((u) => u.members.length > 0).sort((a, b) => near(a) - near(b))[0];
    if (!squad) return;
    await aim(page, squad.position, { ...SQUAD_FRAMINGS.default, yaw: -Math.PI / 2 });
    await snapshot(ctx, page, `soldiers-${name}-1920x1080.png`);
    await lab(page, () => window.__lab.setFrameView("world"));
    await snapshot(ctx, page, `soldiers-${name}-world-1920x1080.png`);
    await lab(page, () => window.__lab.setFrameView("final"));
  };
  // Eight seconds into the advance.
  await advance(page, 240);
  await besideTank("advance");
  await page.close();
}
