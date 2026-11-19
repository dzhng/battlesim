// Muzzle flashes on drawn muzzles, in the street battle: each kind of shot's
// flash and tracer start where the model draws its muzzle (street.mjs's
// `muzzle` tour).
import { stageStreetMoves } from "./_streetStaging.mjs";
import { lab, obs, advance, until, snapshot, presented, openBattle, aim } from "./_lab.mjs";
import { decode, pixel, writeCrop } from "./_png.mjs";
import { streetAttack, streetMap, visuals } from "./_units.mjs";

const CAMERA = visuals.camera;

/** A flash's core projects this close to the drawn
 *  muzzle, in CSS px at the default camera. */
const MUZZLE_PX = 3;
/** The flash's own light, added over the frame without effects, within
 *  `FLASH_BOX_PX` of the drawn muzzle: at least this bright (0..255). */
const FLASH_BOX_PX = 6;
const FLASH_LIGHT_MIN = 60;
/** A hull's round starts this close to its
 *  flash, in metres. The drawn muzzle leads or trails the simulation's by
 *  the gun's pitch and recoil and a tick's drive; the phantom muzzle this
 *  replaced was metres off for an HMG turned from the cannon. */
const TRACER_M = 1;

/** Every flash sits on the muzzle as the model draws
 *  it, not where the simulation starts the round: a tank's cannon (recoiling)
 *  driving and standing, its cupola HMG while driving, the jeep's HMG and a
 *  rifleman's rifle. The cannon and jeep share a road approach; a fresh southern
 *  approach supplies the moving tank HMG. One tank holds throughout; a jeep
 *  approaches the street on a separately admitted move. The rifle fights in
 *  a third battle: the street's houses see the open west and the tank shells
 *  what they see, so a squad crossing it dies short of rifle range. It rounds
 *  the west wood's south holding fire, as the woods tour's walker does, and
 *  goes into the trees at the red AT team on the wood's north edge. At each
 *  kind's first shot the default camera frames the shooter, and the flash's
 *  core and the drawn muzzle socket (posed from the model instance, apart
 *  from the flashes' own muzzles) must project within `MUZZLE_PX`, with the
 *  flash's light on screen there. */
export async function muzzleTour(ctx) {
  let page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick: 30 });
  const approach = async (southern) => {
    const o = await obs(page);
    const driver = o.own.find((u) => u.kind === "test_tank");
    const jeep = o.own.find((u) => u.kind === "test_jeep");
    await stageStreetMoves(
      ctx,
      page,
      [
        {
          kind: "move",
          units: [driver.id],
          gesture: 1,
          goal: southern ? [720, 910] : [driver.position[0] + 400, driver.position[1]],
          route: "shortest",
        },
        { kind: "attack_move", units: [jeep.id], gesture: 2, goal: [690, 910] },
      ],
      southern ? "muzzle southern approach" : "muzzle road approach",
    );
  };
  const flank = async () => {
    const rifle = (await obs(page)).own.find((u) => u.kind === "test_rifle");
    const [[x0], , [, y1]] = streetMap.forests[0].shape.ring;
    const corner = [x0 - 60, y1 + 40];
    // A fire policy, acknowledged before the next staging counts its own.
    const policy = async (p) => {
      const before = await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);
      await lab(
        page,
        (c) => window.__lab.route.command({ kind: "set_engagement", units: [c.id], policy: c.p }),
        { id: rifle.id, p },
      );
      await page.waitForFunction((n) => (window.__lab.route.acks()[0]?.seq ?? 0) > n, before);
    };
    await policy("return_fire_only");
    await stageStreetMoves(
      ctx,
      page,
      [
        {
          kind: "move",
          units: [rifle.id],
          gesture: 3,
          goal: [x0 - 300, y1 + 40],
          route: "fastest",
        },
        {
          order: { kind: "move", units: [rifle.id], gesture: 4, goal: corner, route: "fastest" },
          queued: true,
        },
      ],
      "muzzle rifle round the wood",
    );
    const atCorner = await until(
      page,
      (o) => {
        const u = o.own.find((q) => q.id === rifle.id);
        return (
          u &&
          !u.goal &&
          u.steps.length === 0 &&
          Math.hypot(u.position[0] - corner[0], u.position[1] - corner[1]) < 1
        );
      },
      15000,
      30,
    );
    ctx.check(
      "the rifle squad reaches the wood's south-west corner",
      !!atCorner,
      JSON.stringify(atCorner?.own.find((q) => q.id === rifle.id)?.position ?? null),
    );
    if (!atCorner) throw new Error("muzzle rifle never reached the wood");
    const at = streetAttack.units.find((u) => u.side === "red" && u.kind === "test_at");
    await policy("fire_at_will");
    await stageStreetMoves(
      ctx,
      page,
      [{ kind: "attack_move", units: [rifle.id], gesture: 5, goal: at.position }],
      "muzzle rifle into the wood",
    );
  };
  const passes = [
    { name: "road", subjects: ["tank cannon, driving", "tank cannon, standing", "jeep HMG"] },
    { name: "southern", subjects: ["tank HMG, driving"] },
    { name: "wood", subjects: ["rifle"] },
  ];
  const wanted = new Set(passes.flatMap((p) => p.subjects));
  const found = {};
  for (const [k, pass] of passes.entries()) {
    // Separate controlled fights: the moving tank HMG in physical reach on
    // the flank, and the rifle's fight in the wood.
    if (k > 0) {
      await page.close();
      page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick: 30 });
    }
    if (pass.name === "wood") {
      await flank();
      // The wood's canopy roofs the rifle's fight from the capture camera:
      // that battle is drawn with the trees lifted off, as the ground
      // stations do, from before the fight so no capture rebuilds the scene.
      await lab(page, () => window.__lab.suppressTrees(true));
    } else await approach(pass.name === "southern");
    const pending = new Set(pass.subjects);
    let prev = await obs(page);
    for (let t = 0; t < 30 * 180 && pending.size > 0; t++) {
      await advance(page, 1);
      let o = await obs(page);
      let shot = null;
      for (const u of o.own) {
        const was = prev.own.find((q) => q.id === u.id);
        if (!was || shot) continue;
        const driving = Math.hypot(
          u.position[0] - was.position[0],
          u.position[1] - was.position[1],
        );
        for (const w of u.weaponPoses) {
          const before = was.weaponPoses.find((x) => x.mount === w.mount)?.shots ?? w.shots;
          if (w.shots <= before) continue;
          const name =
            u.kind === "test_tank"
              ? `tank ${w.mount === 0 ? "cannon" : "HMG"}, ${driving > 0.01 ? "driving" : "standing"}`
              : u.kind === "test_jeep"
                ? "jeep HMG"
                : u.kind === "test_rifle" && w.mount === 0
                  ? "rifle"
                  : null;
          if (pending.has(name)) shot = { name, unit: u, mount: w.mount };
        }
      }
      prev = o;
      if (!shot) continue;
      const { name, unit } = shot;
      // The shooter's drawn muzzle: a hull's mount's node, or the rifleman
      // whose new round starts next tick (the simulation flies a round from
      // the tick after it fires).
      let near = unit.position;
      let shooterAt = unit.position;
      let socket = shot.mount === 0 && unit.kind === "test_tank" ? "muzzle" : "hmg_muzzle";
      if (name === "rifle") {
        await advance(page, 1);
        o = prev = await obs(page);
        const squad = o.own.find((u) => u.id === unit.id);
        const round = o.projectiles.find(
          (s) =>
            s.kind === "rifle" &&
            squad.memberIds.includes(s.shooterMember) &&
            Math.hypot(
              squad.members[squad.memberIds.indexOf(s.shooterMember)][0] - s.path[0][0],
              squad.members[squad.memberIds.indexOf(s.shooterMember)][1] - s.path[0][1],
            ) < 2,
        );
        if (!round) continue;
        near = round.path[0];
        shooterAt = squad.members[squad.memberIds.indexOf(round.shooterMember)];
        socket = "muzzle";
      }
      wanted.delete(name);
      pending.delete(name);
      await aim(page, near, {
        distance: CAMERA.default.distance,
        pitch: 0.85,
        yaw: CAMERA.default.yaw,
      });
      const slug = name.replace(/[^a-z]+/gi, "-").toLowerCase();
      await presented(page);
      const lit = decode(await snapshot(ctx, page, `muzzle-${slug}-1920x1080.png`));
      const drawn = await lab(page, () => ({
        sockets: window.__lab.route.muzzleSockets(),
        glows: window.__lab.route.effectInstances().filter((e) => e.shape === 1 && e.rays === 0),
      }));
      await lab(page, () => window.__lab.suppressEffects(true));
      const bare = decode(await snapshot(ctx, page, `muzzle-${slug}-noeffects-1920x1080.png`));
      await lab(page, () => window.__lab.suppressEffects(false));
      const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1], (a[2] ?? 0) - (b[2] ?? 0));
      const nearest = (list, p) => list.reduce((a, b) => (dist(b.at, p) < dist(a.at, p) ? b : a));
      const candidates = drawn.sockets.filter((s) => s.name === socket);
      // Select the firing body first: a neighboring soldier's barrel can lie
      // closer to the published launch point than this soldier's own muzzle.
      const muzzle = candidates.length
        ? candidates.reduce((a, b) =>
            dist(b.origin, shooterAt) < dist(a.origin, shooterAt) ? b : a,
          ).at
        : null;
      const glow = muzzle && drawn.glows.length ? nearest(drawn.glows, muzzle).at : null;
      const px = (p) => p && lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), p);
      const [muzzlePx, glowPx] = [await px(muzzle), await px(glow)];
      let light = 0;
      if (muzzlePx)
        for (let dy = -FLASH_BOX_PX; dy <= FLASH_BOX_PX; dy++)
          for (let dx = -FLASH_BOX_PX; dx <= FLASH_BOX_PX; dx++) {
            const [x, y] = [muzzlePx[0] + dx, muzzlePx[1] + dy];
            if (x < 0 || y < 0 || x >= lit.width || y >= lit.height) continue;
            const [a, b] = [pixel(lit, x, y), pixel(bare, x, y)];
            light = Math.max(light, (a[0] + a[1] + a[2] - b[0] - b[1] - b[2]) / 3);
          }
      const off =
        muzzlePx && glowPx ? Math.hypot(muzzlePx[0] - glowPx[0], muzzlePx[1] - glowPx[1]) : null;
      if (muzzlePx)
        await writeCrop(
          lit,
          ctx.evidencePath(`muzzle-${slug}-crop.png`),
          muzzlePx[0],
          muzzlePx[1],
          60,
          30,
          6,
        );
      found[name] = { tick: o.tick, off, light: Math.round(light), muzzle, glow };
      ctx.check(
        `a ${name} flash sits on the drawn muzzle`,
        off !== null && off <= MUZZLE_PX && light >= FLASH_LIGHT_MIN,
        JSON.stringify(found[name]),
      );
      // A hull's round flies first on the next tick, from the simulation's
      // muzzle for that mount: its tracer starts where the flash is.
      if (name === "rifle" || !muzzle) continue;
      const endKey = (p) => p.join(",");
      const flying = new Set(
        o.projectiles.filter((s) => s.hit === "none").map((s) => endKey(s.path.at(-1))),
      );
      await advance(page, 1);
      prev = await obs(page);
      const hmg = name.includes("HMG");
      const starts = prev.projectiles.filter(
        (s) =>
          s.own &&
          s.shooterMember === null &&
          !flying.has(endKey(s.path[0])) &&
          (hmg ? s.kind === "hmg" : s.kind.startsWith("tank_")),
      );
      const tracer = starts.length
        ? starts.map((s) => s.path[0]).reduce((a, b) => (dist(b, muzzle) < dist(a, muzzle) ? b : a))
        : null;
      const apart = tracer && dist(tracer, muzzle);
      ctx.check(
        `a ${name} tracer starts at its flash`,
        apart !== null && apart <= TRACER_M,
        JSON.stringify({ muzzle, tracer, apart }),
      );
    }
  }
  ctx.check(
    "every kind of shot fired in the staged street fights: tank cannon driving and standing, tank HMG driving, jeep HMG, rifle",
    wanted.size === 0,
    JSON.stringify({ missing: [...wanted], found: Object.keys(found) }),
  );
  await page.close();
}
