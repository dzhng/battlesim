// Slice 11: buildings as abstract fighting positions, through real orders.
// The building is prop 0: centre (360, 250), 24 × 24 m, 8 m tall.
import { readFile, writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { lab, obs, advance, until } from "./_lab.mjs";

const village = JSON.parse(
  await readFile(new URL("../../fixtures/village.json", import.meta.url), "utf8"),
);
const CENTRE = [360, 250];
const HALF = 12;
const STANDOFF = village.garrison.slot_standoff_m;
const CAPACITY = village.buildings.capacity_soldiers;

const demo = (page, name) => lab(page, (n) => window.__lab.route.demo(n), name);
const squad = (o, id) => o.own.find((u) => u.id === id);
/** Chebyshev distance from the building's centre: 12 on its walls. */
const ring = ([x, y]) => Math.max(Math.abs(x - CENTRE[0]), Math.abs(y - CENTRE[1]));
const onWall = (p) => Math.abs(ring(p) - HALF) < 0.3 && p[2] > 0.05 && p[2] < 8;

/** Wait until the panel shows the latest tick, then draw one frame. */
async function settle(page) {
  const tick = await lab(page, () => window.__lab.route.tick());
  await page.waitForFunction(
    (t) =>
      document.querySelector("[data-testid=garrison-panel]")?.textContent.includes(`Tick ${t}`),
    tick,
  );
  await page.waitForTimeout(60);
  await page.evaluate(() => window.__lab.frame());
}

/** Frame the building from `distance` metres, target offset west so it sits
 *  right of the panel. */
const NEAR_SIDE = -0.75;
const FAR_SIDE = NEAR_SIDE + Math.PI;
const look = (page, distance, target = [CENTRE[0] - 14, CENTRE[1] + 2], yaw = NEAR_SIDE) =>
  lab(page, (v) => window.__lab.setCamera({ ...window.__lab.camera(), ...v }), {
    target: [...target, 0],
    distance,
    pitch: 0.95,
    yaw,
  });

async function frame(ctx, page, name) {
  await settle(page);
  const shot = await page.screenshot();
  await writeFile(ctx.evidencePath(`frame-${name}-1280x800.png`), shot);
  return decode(shot);
}

/** Screen points of the building's box grown by `margin` metres. */
async function buildingCorners(page, margin) {
  const corners = [];
  for (const sx of [-1, 1])
    for (const sy of [-1, 1])
      for (const z of [0, 8])
        corners.push(
          await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), [
            CENTRE[0] + sx * (HALF + margin),
            CENTRE[1] + sy * (HALF + margin),
            z,
          ]),
        );
  return corners;
}

/** A 2× crop holding the whole building, its perimeter slots and a margin. */
async function cropBuilding(ctx, page, png, name) {
  const corners = await buildingCorners(page, 4);
  const xs = corners.map((q) => q[0]),
    ys = corners.map((q) => q[1]);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  await writeCrop(
    png,
    ctx.evidencePath(name),
    (x0 + x1) / 2,
    (y0 + y1) / 2,
    Math.ceil((x1 - x0) / 2) + 12,
    Math.ceil((y1 - y0) / 2) + 12,
    2,
  );
}

/** The building's footprint and roof corners all project right of the panel. */
async function framed(page) {
  const panel = await page.getByTestId("garrison-panel").boundingBox();
  const corners = await buildingCorners(page, 2);
  return corners.every(
    (q) => q && q[0] > panel.x + panel.width + 8 && q[0] < 1272 && q[1] > 8 && q[1] < 792,
  );
}

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 20000 });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, 5);
  await look(page, 95);
  const framing = [await framed(page)];

  // Behind the building from where blue stands, red's squad is unseen.
  let o = await obs(page);
  // Battle-look slice 24: the building stands as its appearance while the side
  // knows no ruin (it cannot know of a collapse before it sees one).
  await lab(page, () => window.__lab.frame());
  // Buildings only: the crates are drawn apart too (bodies a vehicle can shove).
  const building = (models) => models.filter((m) => m.state !== "default");
  const standing = building(await lab(page, () => window.__lab.route.structures()));
  ctx.check(
    "a red squad behind the building is not seen from outside it",
    !o.identified.some((e) => e.kind === "rifle"),
    JSON.stringify(o.identified.map((e) => e.kind)),
  );
  // Wide enough to hold blue's squads west of the building as well.
  await look(page, 130, [CENTRE[0] - 50, CENTRE[1] - 8]);
  framing.push(await framed(page));
  await frame(ctx, page, "outside");
  await look(page, 95);

  // Both rifle squads garrison: 16 soldiers fill 16 slots.
  const TANK_OPENS_FIRE = await lab(page, () => window.__lab.route.tankOpensFire);
  // The tick the house was first seen fallen during the entry steps, if it was.
  let fellAt = null;
  const watchHouse = (f) => {
    if (fellAt === null && f.knownProps.some((p) => p.kind === "ruin")) fellAt = f.tick;
  };
  const order = await demo(page, "Garrison both rifle squads");
  let entering = null;
  let firstInside = null;
  o = await until(
    page,
    (f) => [0, 1].every((id) => squad(f, id)?.garrison?.phase === "inside") || fellAt !== null,
    1500,
    5,
    (f) => {
      watchHouse(f);
      const g = squad(f, 0)?.garrison;
      if (g?.phase === "entering" && g.progress > 0.2 && g.progress < 0.8 && !entering)
        entering = { ...g, tick: f.tick };
      if (g?.phase === "inside" && !firstInside) firstInside = f;
    },
  );
  ctx.check(
    "two squads enter after a stationary timer",
    order?.error === null && !!o && !!entering && fellAt === null,
    JSON.stringify({ order, entering, fellAt }),
  );
  if (fellAt !== null) {
    // Fail at the cause: the rest of the scene needs a standing house.
    ctx.check(
      "staging: the house stands through the entry steps",
      false,
      `the house fell at tick ${fellAt}: re-stage the garrison lab (routes/garrison.tsx)`,
    );
    return page.close();
  }
  const occupants = [0, 1].flatMap((id) => squad(o, id).members);
  const distinct = new Set(occupants.map((p) => `${p[0].toFixed(2)},${p[1].toFixed(2)}`));
  ctx.check(
    "every occupant stands at its own perimeter slot just outside the walls",
    // Every living occupant (the red tank may drop one on the walk in).
    occupants.length > CAPACITY / 2 &&
      distinct.size === occupants.length &&
      occupants.every((p) => Math.abs(ring(p) - HALF - STANDOFF) < 0.01),
    `${occupants.length} occupants, ${distinct.size} places`,
  );
  const refused = await demo(page, "Scouts try to join");
  await advance(page, 5);
  o = await obs(page);
  ctx.check(
    "a third squad is refused whole for want of room",
    refused?.error?.reason === "capacity_full" && squad(o, 2).garrison === null,
    JSON.stringify(refused),
  );
  ctx.check(
    "occupants on the far facade see the squad behind the building",
    !!firstInside?.identified.some((e) => e.kind === "rifle"),
    JSON.stringify(firstInside?.identified.map((e) => e.kind)),
  );

  // One squad leaves; the scouts then fit beside the other.
  const leave = await demo(page, "Rifle squad #1 leaves");
  let exiting = false;
  o = await until(
    page,
    (f) => squad(f, 1) && squad(f, 1).garrison === null,
    300,
    5,
    (f) => {
      watchHouse(f);
      exiting ||= squad(f, 1)?.garrison?.phase === "exiting";
    },
  );
  ctx.check(
    "a squad leaves after its timer to free ground outside",
    leave?.error === null &&
      exiting &&
      !!o &&
      squad(o, 1).members.every((p) => ring(p) > HALF + 0.5),
    JSON.stringify(o && squad(o, 1).members.slice(0, 2)),
  );
  // The squad that left falls back out of red's sight (the lab's staging:
  // the tank opens fire on the occupants, not on it).
  await demo(page, "Squad #1 falls back west");
  const joined = await demo(page, "Scouts try to join");
  o = await until(
    page,
    (f) => squad(f, 2)?.garrison?.phase === "inside" || fellAt !== null,
    1500,
    15,
    watchHouse,
  );
  ctx.check(
    "capacity counts soldiers: the scouts fit once a squad has left",
    joined?.error === null && !!o,
    JSON.stringify(joined),
  );
  // The staging's guard: the lab holds red's tank until TANK_OPENS_FIRE so
  // the entry steps above measure the timer, not a collapse. If a rule
  // change brings the house down, or slows the entries past that tick, fix
  // the lab's staging (routes/garrison.tsx), not these checks.
  ctx.check(
    "staging: every entry finished, the house standing, before red's tank opens fire",
    !!o && o.tick < TANK_OPENS_FIRE && fellAt === null,
    JSON.stringify({ entriesDoneAt: o?.tick ?? null, tankOpensFire: TANK_OPENS_FIRE, fellAt }),
  );

  // Red's tank shells the occupants its squad spots; blue answers outward.
  let own = 0,
    ownOnWall = 0,
    enemyOnWall = 0,
    shot = null;
  const hurt = () =>
    [0, 2].some((id) => {
      const u = squad(o, id);
      const full = id === 0 ? 8 : 4;
      return !u || u.members.length < full || u.memberHp.some((hp) => hp < village.health.soldier);
    });
  // The last frame with occupants inside, before the fall: the shelling can
  // bring the building down during the firefight already.
  let before = null;
  const inside = (f) => [0, 2].some((id) => squad(f, id)?.garrison?.phase === "inside");
  const fallen = (f) => f.knownProps.some((p) => p.kind === "ruin");
  for (let t = 0; t < 900 && !(enemyOnWall > 2 && hurt()) && !fallen(o); t += 3) {
    await advance(page, 3);
    o = await obs(page);
    if (inside(o)) before = o;
    // Rounds from the occupants: the squad that left fires from outside
    // past the house's corner, and its rounds grazing that corner are no
    // occupant's.
    const occupants = new Set(
      o.own.filter((u) => u.garrison?.phase === "inside").flatMap((u) => u.memberIds),
    );
    for (const p of o.projectiles) {
      if (p.own) {
        if (!occupants.has(p.shooterMember)) continue;
        own += 1;
        if (p.hit !== "none" && onWall(p.path.at(-1))) ownOnWall += 1;
      } else if (p.hit !== "none" && onWall(p.path.at(-1))) {
        enemyOnWall += 1;
        if (!shot) shot = await frame(ctx, page, "firefight");
      }
    }
  }
  ctx.check(
    "outgoing rounds leave by facades facing the target and never strike their own walls",
    own > 20 && ownOnWall === 0,
    `${own} blue segments, ${ownOnWall} on the building`,
  );
  ctx.check(
    "red's misses stop in the walls and its fire hurts occupants",
    enemyOnWall > 0 && hurt(),
    `${enemyOnWall} red impacts on the walls`,
  );
  const firefight = shot ?? (await frame(ctx, page, "firefight"));
  // The facades facing away from the first camera, from the other side.
  await look(page, 95, [CENTRE[0] + 14, CENTRE[1] - 2], FAR_SIDE);
  framing.push(await framed(page));
  const farSide = await frame(ctx, page, "firefight-far-side");
  await cropBuilding(ctx, page, farSide, "crop-perimeter-slots-far-side-2x.png");
  await look(page, 95);
  await cropBuilding(ctx, page, firefight, "crop-perimeter-slots-2x.png");

  // The tank keeps shelling until the building falls.
  o = fallen(o)
    ? o
    : await until(page, fallen, 4500, 3, (f) => {
        if (inside(f)) before = f;
      });
  const ruin = o?.knownProps.find((p) => p.kind === "ruin");
  ctx.check(
    "the building collapses into a lower ruin on its footprint",
    !!ruin &&
      ruin.replaces === 0 &&
      ruin.center.join() === CENTRE.join() &&
      ruin.half[0] === HALF &&
      Math.abs(ruin.half[2] * 2 - village.props.building.destroyed.into.height_m) < 1e-6,
    JSON.stringify(ruin),
  );
  const occupantsBefore = before
    ? [0, 2].flatMap((id) =>
        squad(before, id)?.garrison?.phase === "inside" ? squad(before, id).members : [],
      )
    : [];
  const survivors = o ? [0, 2].map((id) => squad(o, id)).filter(Boolean) : [];
  ctx.check(
    "survivors come out on foot outside the ruin, heavily suppressed",
    occupantsBefore.length > 0 &&
      survivors.length > 0 &&
      survivors.every(
        (u) =>
          u.garrison === null &&
          u.suppression >= village.suppression.collapse_level &&
          u.members.every((p) => ring(p) > HALF),
      ),
    JSON.stringify(
      survivors.map((u) => ({ id: u.id, n: u.members.length, s: u.suppression.toFixed(2) })),
    ),
  );
  ctx.check(
    "every occupant is a survivor or a fallen soldier",
    !!before &&
      occupantsBefore.length ===
        survivors.reduce((n, u) => n + u.members.length, 0) +
          (o.corpses.filter((c) => c.own).length - before.corpses.filter((c) => c.own).length),
    `${occupantsBefore.length} inside before`,
  );
  await look(page, 95);
  framing.push(await framed(page));
  const collapsed = await frame(ctx, page, "collapse");
  // The ruin at 1920x1080 from the ground-ish framing the slice 24 verdict reads.
  await page.setViewportSize({ width: 1920, height: 1080 });
  await lab(page, (v) => window.__lab.setCamera({ ...window.__lab.camera(), ...v }), {
    target: [CENTRE[0], CENTRE[1], 0],
    distance: 55,
    pitch: 0.6,
    yaw: NEAR_SIDE,
  });
  await settle(page);
  await writeFile(ctx.evidencePath("ruin-1920x1080.png"), await page.screenshot());
  await page.setViewportSize({ width: 1280, height: 800 });
  // The ruin replaces the building in one list: the same appearance, ruined,
  // on the same spot, and no intact building left beside it.
  const ruined = building(await lab(page, () => window.__lab.route.structures()));
  const at = (m) => m.position.slice(0, 2).join();
  ctx.check(
    "the known ruin swaps in for its building atomically, as that building's appearance",
    standing.length === 1 &&
      standing[0].state === "intact" &&
      at(standing[0]) === CENTRE.join() &&
      ruined.length === 1 &&
      ruined[0].state === "ruin" &&
      at(ruined[0]) === CENTRE.join() &&
      ruined[0].appearance === standing[0].appearance,
    JSON.stringify({ standing, ruined }),
  );
  await cropBuilding(ctx, page, collapsed, "crop-ruin-2x.png");

  // The ruin blocks ground movement: squad #1's route goes round it.
  const cross = await demo(page, "Squad #1 crosses the ruin");
  await advance(page, 4);
  o = await obs(page);
  const mover = squad(o, 1);
  const crosses = (() => {
    if (!mover) return true;
    let from = mover.position;
    for (const p of mover.route) {
      for (let k = 0; k <= 40; k++) {
        const q = [from[0] + ((p[0] - from[0]) * k) / 40, from[1] + ((p[1] - from[1]) * k) / 40];
        if (ring(q) < HALF) return true;
      }
      from = p;
    }
    return false;
  })();
  ctx.check(
    "a squad sent across the ruin routes round it",
    cross?.error === null && mover.route.length > 0 && !crosses,
    JSON.stringify(mover?.route),
  );
  await advance(page, 240);
  await frame(ctx, page, "ruin-detour");
  ctx.check(
    "every capture frames the building clear of the panel",
    framing.every(Boolean),
    JSON.stringify(framing),
  );
  await writeFile(
    ctx.evidencePath("metadata.json"),
    JSON.stringify(
      {
        fixture: "garrison",
        seed: 11,
        viewport: [1280, 800],
        dpr: 1,
        browser: ctx.browser,
        entering,
        firefight: { blueSegments: own, redWallImpacts: enemyOnWall },
        collapseTick: o?.tick,
      },
      null,
      2,
    ),
  );
}
