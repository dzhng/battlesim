// The first helicopter checkpoint. One move order flies the test helicopter
// across the village: it holds cruise height over open ground, climbs to
// clear the roofs taller than that allows, never passes the ceiling, and goes
// round the tower instead of over it. Red's rifle squad plinks at it without
// hurting it; the tank to the south never fires its main gun; the gun
// jeep, freed once the helicopter hovers at its goal, hurts it. Every claim
// is read from blue's observation; the frames are the flight profile for
// review (the whole route side on and from above, with the helicopter pasted
// in along it; a 2× crop at the roof pop and at the tower detour) and the
// fire (a rifle strike, the jeep firing, its hit).
import { writeFile } from "node:fs/promises";
import { PNG } from "pngjs";
import { decode, writeCrop } from "./_png.mjs";
import { lab, obs, advance, presented, openBattle, gpuWarnings, hideHud, aim } from "./_lab.mjs";
import { writeSheet } from "./_sheet.mjs";
import { game, unitType } from "./_units.mjs";
import { loadEncounter, loadMap } from "../src/maps/node.ts";

const map = loadMap("air").definition;
const encounter = loadEncounter("air", "flight");
const air = game.air;
const typeHp = unitType("test_heli").body.hull.hp;
const goal = encounter.scripts.find((s) => s.side === "blue").order.goal;
const jeepAt = encounter.units.find((u) => u.kind === "test_gun_jeep").position;
// The ticks red's gun jeep is told to fire at will, ending the rifle phase,
// and to hold its fire again after its first round, so one hit hurts the
// helicopter and the next does not bring it down (its fall is a later slice's).
const [jeepFree, jeepHeld] = encounter.scripts
  .filter((s) => s.order.kind === "set_engagement")
  .map((s) => s.tick);

/** Every building part as a box (in the world): centre, yaw, half extents and top. */
const parts = map.buildings.flatMap((b) =>
  b.geometry.parts.map((p) => ({
    template: b.geometry.template_id,
    center: p.center,
    yaw: p.yaw,
    half: p.half_extents,
    top: p.base_z + 2 * p.half_extents[2],
  })),
);
/** Whether XY point `q` lies inside part `p`'s footprint. */
function inside(p, q) {
  const [dx, dy] = [q[0] - p.center[0], q[1] - p.center[1]];
  const [c, s] = [Math.cos(p.yaw), Math.sin(p.yaw)];
  const [u, v] = [dx * c + dy * s, -dx * s + dy * c];
  return Math.abs(u) <= p.half[0] && Math.abs(v) <= p.half[1];
}
// The tower walls the route; the roofs the clearance rises over lift it.
const towers = parts.filter((p) => p.top > air.obstacle_m);
const lifting = parts.filter(
  (p) => p.top <= air.obstacle_m && p.top + air.clearance_m > air.cruise_agl_m,
);

/** One observed instant of the flight (no helicopter once it has fallen). */
async function sample(page) {
  const o = await obs(page);
  const heli = o.own.find((u) => u.kind === "test_heli");
  const ground =
    heli && (await lab(page, (p) => window.__lab.route.surfaceZ(p[0], p[1]), heli.position));
  const seen = (kind) => o.identified.find((u) => u.kind === kind);
  return { o, heli, ground, tank: seen("test_tank"), jeep: seen("test_gun_jeep") };
}

/** Enemy rounds of `kind` this tick that ended on a hull within `r` m of `at`. */
const strikes = (o, kind, at, r = 9) =>
  o.projectiles.filter(
    (p) =>
      !p.own &&
      p.kind === kind &&
      p.hit === "hull" &&
      Math.hypot(...p.path.at(-1).map((v, i) => v - at[i])) < r,
  ).length;

export async function run(ctx) {
  const page = await openBattle(ctx);
  const warnings = gpuWarnings(page);
  await hideHud(page);
  const css = (p) => lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), p);

  /** Draw the presented tick and return the frame, saved as `file`. */
  const shoot = async (file) => {
    await presented(page);
    await page.evaluate(() => window.__lab.frame());
    const shot = await page.screenshot();
    if (file) await writeFile(ctx.evidencePath(file), shot);
    return shot;
  };
  // The whole route from start to goal, side on from the south with the eye
  // under its height (the helicopter against the sky over the roofs) and from
  // straight above (the way round the tower).
  const views = {
    side: [315, 470, 12, { distance: 420, pitch: 0, yaw: -Math.PI / 2 }],
    plan: [315, 470, 0, { distance: 540, pitch: 1.55, yaw: -Math.PI / 2 }],
  };
  const view = (name) => {
    const [x, y, z, v] = views[name];
    return aim(page, [x, y, z], v, { onGround: false });
  };
  const wide = () => view("side");
  /** The route's frame from each view, with where it draws the helicopter and
   *  how far its rotor reaches on screen. */
  const strobeFrames = async (heli) => {
    const out = {};
    const p = heli.position;
    for (const name of Object.keys(views)) {
      await view(name);
      const at = await css(p);
      let reach = 0;
      for (const d of [
        [8, 0, 0],
        [0, 8, 0],
        [0, 0, 8],
      ]) {
        const q = await css([p[0] + d[0], p[1] + d[1], p[2] + d[2]]);
        reach = Math.max(reach, Math.hypot(q[0] - at[0], q[1] - at[1]));
      }
      out[name] = { shot: decode(await shoot()), at, r: reach + 3 };
    }
    await wide();
    return out;
  };
  /** A close look on `centre`, its frame saved and a 2× crop round the
   *  helicopter. */
  const close = async (name, view, heli, centre, halfW = 220, halfH = 130) => {
    const at = [heli.position[0], heli.position[1], heli.position[2]];
    await aim(page, centre ?? at, view, { onGround: false });
    const shot = await shoot(`frame-${name}-1280x800.png`);
    const c = await css(at);
    await writeCrop(
      decode(shot),
      ctx.evidencePath(`crop-${name}-2x.png`),
      c[0],
      c[1],
      halfW,
      halfH,
      2,
    );
    await wide();
    return shot;
  };

  let s = await sample(page);
  ctx.check(
    "the helicopter waits at cruise height over open ground before its order",
    Math.abs(s.heli.position[2] - s.ground - air.cruise_agl_m) < 0.5,
    JSON.stringify({ at: s.heli.position, ground: s.ground }),
  );

  // Fly the route a tick at a time, until it hovers at its goal.
  await wide();
  const track = [];
  const strobe = [];
  let rifleHits = 0;
  let cannonShots = 0;
  let tankSeen = 0;
  let popShot = null;
  let detourShot = null;
  let sparkShot = null;
  let nearestTower = Infinity;
  for (let t = s.o.tick; t < jeepFree - 1; t++) {
    await advance(page, 1);
    s = await sample(page);
    const { heli, ground, o, tank } = s;
    track.push({ tick: o.tick, at: heli.position, ground, hp: heli.hp, state: heli.state });
    const struck = strikes(o, "rifle", heli.position);
    rifleHits += struck;
    if (tank) {
      tankSeen++;
      cannonShots = Math.max(cannonShots, tank.weaponPoses[0].shots);
    }
    if (o.tick % 20 === 0 && heli.position[0] < goal[0] - 1) strobe.push(await strobeFrames(heli));
    // Rifle rounds striking it on the way, from the squad's side.
    if (!sparkShot && struck > 1 && heli.state === "moving")
      sparkShot = await close(
        "rifle-sparks",
        { distance: 32, pitch: 0.12, yaw: -1.2 },
        heli,
        null,
        200,
        120,
      );
    // From under the roof's edge: the roof and the helicopter over it both
    // against the sky, the gap between them at one distance.
    const roof = lifting.find((p) => inside(p, heli.position));
    if (!popShot && roof && roof.top + air.clearance_m > air.cruise_agl_m + 1) {
      const [x, y, z] = heli.position;
      popShot = await close("roof-pop", { distance: 70, pitch: -0.12, yaw: -1.3 }, heli, [
        x,
        y,
        (roof.top + z) / 2,
      ]);
    }
    const tower = towers.find((p) => Math.abs(heli.position[0] - p.center[0]) < 1.5);
    if (!detourShot && tower) {
      nearestTower = Math.abs(heli.position[0] - tower.center[0]);
      // From straight above the tower: the helicopter beside its roof.
      const above = [tower.center[0], tower.center[1], tower.top];
      detourShot = await close(
        "tower-detour",
        { distance: 160, pitch: 1.5, yaw: -Math.PI / 2 },
        heli,
        above,
        200,
        150,
      );
    }
  }
  const arrived = s;

  // The track, against the rules' heights and the map's bodies.
  const over = (q) => parts.filter((p) => inside(p, q.at));
  const rows = track.map((q) => ({ ...q, agl: q.at[2] - q.ground, roofs: over(q) }));
  const ceiling = Math.max(...rows.map((r) => r.agl));
  ctx.check(
    "it never climbs past the ceiling",
    ceiling <= air.ceiling_agl_m + 0.5,
    JSON.stringify({ highest: ceiling, ceiling: air.ceiling_agl_m }),
  );
  const overRoofs = rows.filter((r) => r.roofs.some((p) => lifting.includes(p)));
  const lowest = overRoofs.map((r) => r.at[2] - Math.max(...r.roofs.map((p) => p.top)));
  ctx.check(
    "over each roof taller than its clearance allows it flies a clearance above the roof",
    overRoofs.length > 0 && Math.min(...lowest) >= air.clearance_m - 1,
    JSON.stringify({ samples: overRoofs.length, lowestOverRoof: Math.min(...lowest) }),
  );
  ctx.check(
    "over those roofs it flies higher than cruise",
    overRoofs.every((r) => r.agl > air.cruise_agl_m + 1),
    JSON.stringify(overRoofs.map((r) => +r.agl.toFixed(1))),
  );
  const inTower = rows.filter((r) => towers.some((p) => inside(p, r.at)));
  ctx.check(
    "its track never enters the tower's footprint",
    towers.length > 0 && inTower.length === 0,
    JSON.stringify({ towers: towers.map((p) => p.template), inside: inTower.map((r) => r.at) }),
  );
  ctx.check(
    "it passes the tower beside it, not short of it",
    detourShot !== null && nearestTower < 1.5,
    JSON.stringify({ nearestTower }),
  );
  // It flies on round the tower: never back for a corner of its route.
  let furthest = -Infinity;
  let fellBack = 0;
  for (const r of rows) {
    furthest = Math.max(furthest, r.at[0]);
    fellBack = Math.max(fellBack, furthest - r.at[0]);
  }
  ctx.check("it never turns back on its route", fellBack < 2, JSON.stringify({ fellBack }));
  ctx.check(
    "it arrives at its goal and hovers at cruise height over open ground",
    Math.hypot(arrived.heli.position[0] - goal[0], arrived.heli.position[1] - goal[1]) < 1 &&
      Math.abs(arrived.heli.position[2] - arrived.ground - air.cruise_agl_m) < 0.5,
    JSON.stringify({ at: arrived.heli.position, ground: arrived.ground }),
  );
  ctx.check("rifle rounds strike the helicopter", rifleHits >= 5, JSON.stringify({ rifleHits }));
  ctx.check(
    "the rifle squad's fire leaves its health full",
    rows.every((r) => r.hp === typeHp),
    JSON.stringify({ hp: [...new Set(rows.map((r) => r.hp))] }),
  );
  const ends = {};
  for (const name of Object.keys(views)) {
    await view(name);
    ends[name] = await shoot(`frame-route-end-${name}-1280x800.png`);
  }
  await wide();

  // The jeep is freed: its autocannon hurts the helicopter.
  let jeepHits = 0;
  let jeepShots = 0;
  let hurt = null;
  let hitShot = null;
  let fireShot = null;
  const jeepPhase = [];
  /** Step one tick of the jeep's phase, tallying its fire and the tank's. */
  const step = async () => {
    await advance(page, 1);
    s = await sample(page);
    jeepHits += strikes(s.o, "autocannon", s.heli?.position ?? goal);
    const fired = s.jeep && s.jeep.weaponPoses[0].shots > jeepShots;
    if (s.jeep) jeepShots = Math.max(jeepShots, s.jeep.weaponPoses[0].shots);
    // Its first round, from behind the jeep toward the helicopter.
    if (fired && !fireShot && s.heli) {
      const [hx, hy] = s.heli.position;
      const away = Math.atan2(jeepAt[1] - hy, jeepAt[0] - hx);
      fireShot = await close(
        "jeep-fires",
        { distance: 24, pitch: 0.1, yaw: away + 0.25 },
        { position: s.jeep.position },
        [...s.jeep.position.slice(0, 2), s.jeep.position[2] + 1.5],
        200,
        120,
      );
    }
    jeepPhase.push({ tick: s.o.tick, hp: s.heli?.hp ?? 0, jeepShots });
    if (s.tank) {
      tankSeen++;
      cannonShots = Math.max(cannonShots, s.tank.weaponPoses[0].shots);
    }
  };
  // Its rounds take a second to fly there, past the hold order.
  while (s.o.tick < jeepHeld + 3 * game.tick_hz && !hurt) {
    await step();
    if (s.heli && s.heli.hp < typeHp) hurt = s;
  }
  ctx.check(
    "the gun jeep's autocannon damages it",
    hurt !== null && jeepHits > 0 && jeepShots > 0,
    JSON.stringify({ hp: hurt?.heli.hp, jeepHits, jeepShots, tick: hurt?.o.tick }),
  );
  // The moment it is hit, looking along the jeep's line of fire.
  if (hurt) {
    const [hx, hy] = hurt.heli.position;
    const towardJeep = Math.atan2(jeepAt[1] - hy, jeepAt[0] - hx);
    hitShot = await close(
      "jeep-hit",
      { distance: 60, pitch: -0.08, yaw: towardJeep + 0.5 },
      hurt.heli,
      null,
      160,
      100,
    );
  }
  // Told to hold its fire, the jeep leaves the helicopter hurt but flying.
  while (s.o.tick < jeepHeld + 10 * game.tick_hz) await step();
  await ctx.writeEvidence("jeep-phase.json", jeepPhase);
  ctx.check(
    "held after one hit, the jeep leaves it hovering, hurt",
    !!s.heli && s.heli.hp === hurt?.heli.hp,
    JSON.stringify({ hp: s.heli?.hp, tick: s.o.tick }),
  );
  ctx.check(
    "the tank, seen throughout, never fires its main gun",
    tankSeen > 0 && cannonShots === 0,
    JSON.stringify({ tankSeen, cannonShots }),
  );

  // The flight profile: each view's frame at the route's end with the
  // helicopter pasted in where each earlier frame drew it: the pixels within
  // its rotor's reach that the next frame, the helicopter gone on and the
  // side's sight much the same, draws differently.
  const profiles = {};
  for (const name of Object.keys(views)) {
    const base = decode(ends[name]);
    strobe.forEach(({ [name]: { shot, at, r } }, k) => {
      const next = strobe[k + 1]?.[name].shot ?? decode(ends[name]);
      for (let y = Math.max(0, Math.round(at[1] - r)); y < Math.min(base.height, at[1] + r); y++)
        for (let x = Math.max(0, Math.round(at[0] - r)); x < Math.min(base.width, at[0] + r); x++) {
          if (Math.hypot(x - at[0], y - at[1]) > r) continue;
          const i = (y * base.width + x) * 4;
          const d =
            Math.abs(shot.data[i] - next.data[i]) +
            Math.abs(shot.data[i + 1] - next.data[i + 1]) +
            Math.abs(shot.data[i + 2] - next.data[i + 2]);
          if (d > 24) shot.data.copy(base.data, i, i, i + 4);
        }
    });
    profiles[name] = PNG.sync.write(base);
    await writeFile(ctx.evidencePath(`frame-route-strobe-${name}-1280x800.png`), profiles[name]);
  }
  await ctx.writeEvidence(
    "track.json",
    rows.map(({ roofs, ...r }) => ({ ...r, roofs: roofs.map((p) => p.template) })),
  );
  const cells = [
    { png: profiles.side, caption: "the route side on: the helicopter every 20 ticks" },
    { png: profiles.plan, caption: "the route from above: the helicopter every 20 ticks" },
    popShot && { png: popShot, caption: "over the 7-floor block" },
    detourShot && { png: detourShot, caption: "round the tower, from above" },
    sparkShot && { png: sparkShot, caption: "a rifle round strikes it" },
    fireShot && { png: fireShot, caption: "the gun jeep fires" },
    hitShot && { png: hitShot, caption: "the jeep's autocannon hits it" },
  ].filter(Boolean);
  await writeSheet(ctx, "sheet-flight-profile.png", {
    title: "Air: the flight profile",
    columns: 1,
    cellWidth: 960,
    cells,
  });
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
