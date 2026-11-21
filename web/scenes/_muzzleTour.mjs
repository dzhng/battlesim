// Muzzle flashes on drawn muzzles, in the street battle: each kind of shot's
// flash and tracer start where the model draws its muzzle (street.mjs's
// `muzzle` tour).
import { stageStreetMoves } from "./_streetStaging.mjs";
import { lab, obs, advance, snapshot, presented, openBattle, aim } from "./_lab.mjs";
import { decode, pixel, writeCrop } from "./_png.mjs";
import { visuals } from "./_units.mjs";

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

/** A shot as the passes name it: what fired it and how, or null. */
function shotName(unit, mount, driving) {
  if (unit.kind === "test_tank")
    return `tank ${mount === 0 ? "cannon" : "HMG"}, ${driving ? "driving" : "standing"}`;
  if (unit.kind === "test_jeep") return "jeep HMG";
  return unit.kind === "test_rifle" && mount === 0 ? "rifle" : null;
}

/** The first own shot between `prev` and `o` of a kind still `pending`:
 *  its name, shooter and mount, or null. */
function newShot(prev, o, pending) {
  for (const u of o.own) {
    const was = prev.own.find((q) => q.id === u.id);
    if (!was) continue;
    const driving =
      Math.hypot(u.position[0] - was.position[0], u.position[1] - was.position[1]) > 0.01;
    for (const w of u.weaponPoses) {
      const before = was.weaponPoses.find((x) => x.mount === w.mount)?.shots ?? w.shots;
      const name = w.shots > before ? shotName(u, w.mount, driving) : null;
      if (pending.has(name)) return { name, unit: u, mount: w.mount };
    }
  }
  return null;
}

/** Every flash sits on the muzzle as the model draws
 *  it, not where the simulation starts the round: a tank's cannon (recoiling)
 *  driving and standing, its cupola HMG while driving, the jeep's HMG and a
 *  rifleman's rifle. On the road approach one tank drives east down the road,
 *  its cannon firing on the move, while the jeep attack-moves into the lane
 *  south of it and shoots the red AT team at the wood's edge, which gives
 *  itself away firing at the tank. A second battle stages the same approach
 *  and sends the other tank into that lane: it halts there and fires its
 *  cannon standing, then drives on down the lane, its HMG firing on the move
 *  at the AT team the lane sees past the wood's edge. The rifle fights in
 *  a third battle, the street map's `lean` encounter: a blue and a red rifle
 *  squad close in the west wood. Staged in the attack battle, red's AT team
 *  at the wood's edge spots any squad crossing the open west for the tank,
 *  which shells it before it reaches the trees. At each
 *  kind's first shot the default camera frames the shooter, and the flash's
 *  core and the drawn muzzle socket (posed from the model instance, apart
 *  from the flashes' own muzzles) must project within `MUZZLE_PX`, with the
 *  flash's light on screen there. */
export async function muzzleTour(ctx) {
  let page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick: 30 });
  // The jeep's lane: where the second tank halts, and how far it drives on.
  const [laneHalt, laneOn] = [
    [330, 905],
    [520, 905],
  ];
  /** The road approach, and with `lane` the second tank sent to halt in the
   *  jeep's lane: that tank, or null. */
  const approach = async (lane) => {
    const o = await obs(page);
    const [driver, second] = o.own.filter((u) => u.kind === "test_tank");
    const jeep = o.own.find((u) => u.kind === "test_jeep");
    await stageStreetMoves(
      ctx,
      page,
      [
        {
          kind: "move",
          units: [driver.id],
          gesture: 1,
          goal: [driver.position[0] + 400, driver.position[1]],
          route: "shortest",
        },
        { kind: "attack_move", units: [jeep.id], gesture: 2, goal: [690, 910] },
        ...(lane
          ? [{ kind: "move", units: [second.id], gesture: 3, goal: laneHalt, route: "shortest" }]
          : []),
      ],
      lane ? "muzzle lane approach" : "muzzle road approach",
    );
    return lane ? second : null;
  };
  const passes = [
    { name: "road", subjects: ["tank cannon, driving", "jeep HMG"] },
    { name: "lane", subjects: ["tank cannon, standing", "tank HMG, driving"] },
    { name: "wood", subjects: ["rifle"] },
  ];
  const wanted = new Set(passes.flatMap((p) => p.subjects));
  const found = {};
  let laneTank = null;
  for (const [k, pass] of passes.entries()) {
    // Separate controlled fights: the second tank in the jeep's lane, and
    // the rifle's fight in the wood.
    if (k > 0) {
      await page.close();
      const url = new URL(ctx.url);
      if (pass.name === "wood") url.searchParams.set("encounter", "lean");
      page = await openBattle(ctx, {
        viewport: { width: 1920, height: 1080 },
        tick: 30,
        url: url.href,
      });
    }
    if (pass.name === "wood") {
      // The wood's canopy roofs the rifle's fight from the capture camera:
      // that battle is drawn with the trees lifted off, as the ground
      // stations do, from before the fight so no capture rebuilds the scene.
      await lab(page, () => window.__lab.suppressTrees(true));
    } else laneTank = await approach(pass.name === "lane");
    const pending = new Set(pass.subjects);
    let prev = await obs(page);
    for (let t = 0; t < 30 * 180 && pending.size > 0; t++) {
      await advance(page, 1);
      let o = await obs(page);
      const shot = newShot(prev, o, pending);
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
      if (name !== "rifle" && muzzle) {
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
          ? starts
              .map((s) => s.path[0])
              .reduce((a, b) => (dist(b, muzzle) < dist(a, muzzle) ? b : a))
          : null;
        const apart = tracer && dist(tracer, muzzle);
        ctx.check(
          `a ${name} tracer starts at its flash`,
          apart !== null && apart <= TRACER_M,
          JSON.stringify({ muzzle, tracer, apart }),
        );
      }
      // Its standing shot fired, the lane's tank drives on down the lane.
      if (name === "tank cannon, standing" && laneTank) {
        await stageStreetMoves(
          ctx,
          page,
          [{ kind: "move", units: [laneTank.id], gesture: 4, goal: laneOn, route: "shortest" }],
          "muzzle lane drive on",
        );
        prev = await obs(page);
      }
    }
  }
  ctx.check(
    "every kind of shot fired in the staged street fights: tank cannon driving and standing, tank HMG driving, jeep HMG, rifle",
    wanted.size === 0,
    JSON.stringify({ missing: [...wanted], found: Object.keys(found) }),
  );
  await page.close();
}
