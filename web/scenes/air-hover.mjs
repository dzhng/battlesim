// Test helicopters hovering at cruise height: drawn up there, rotors turning
// from frame to frame, shadow on the ground below and off along the sun, and
// where each is over the ground read from its ground marker and drop line
// (D18): over open ground, over a roof, an identified enemy's, selected and
// not, and from the map's zoom. The pose driver's and model detail's tests
// own the rotor arithmetic and the shadow's culling, the overlay's tests the
// marks' geometry; this judges the drawn frame.
import { writeFile } from "node:fs/promises";
import { decode, mostChanged, pixel, around, anyNear, writeCrop } from "./_png.mjs";
import {
  lab,
  obs,
  advance,
  presented,
  openBattle,
  gpuWarnings,
  hideHud,
  aim,
  snapshot,
} from "./_lab.mjs";
import { game, hull, unitType } from "./_units.mjs";

/** The test helicopter's ground marker's radius: its hull's half length and
 *  the vehicle marker's margin. */
const MARKER_R =
  hull("test_heli").half_extents_m[0] + game.presentation.overlay.orders.vehicle_marker_margin_m;

/** Mean luminance of the pixels within `r` px of `p`. */
function luminance(png, p, r) {
  let sum = 0;
  let n = 0;
  for (const [x, y] of around(png, p, r)) {
    const [R, G, B] = pixel(png, x, y);
    sum += 0.2126 * R + 0.7152 * G + 0.0722 * B;
    n++;
  }
  return sum / n;
}

/** The brightest pixel within `r` px of `p`, as [R, G, B]. */
function brightest(png, p, r) {
  let best = [0, 0, 0];
  for (const [x, y] of around(png, p, r)) {
    const c = pixel(png, x, y);
    if (c[0] + c[1] + c[2] > best[0] + best[1] + best[2]) best = c;
  }
  return best;
}
const sum = (c) => c[0] + c[1] + c[2];

/** The brightest pixel within `r` of the screen segment `a`–`b`, sampled
 *  every pixel along it: a dotted line is found wherever a dot falls. */
function brightestAlong(png, a, b, r) {
  const steps = Math.max(1, Math.ceil(Math.hypot(b[0] - a[0], b[1] - a[1])));
  let best = [0, 0, 0];
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const c = brightest(png, [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t], r);
    if (sum(c) > sum(best)) best = c;
  }
  return best;
}
/** How far either side of a sampled height a drop line is looked for, metres:
 *  past the pitch of its dots at every zoom the scene views it from. */
const DROP_SPAN_M = 3;
/** The share of an airframe's height its drop line's dots fade out by. */
const DROP_REACH = game.presentation.overlay.orders.drop_line_reach;
/** Where along the height a drop line is looked for: low, where its dots
 *  are strongest. */
const DROP_LOW = 0.2;

/** The overlays alone over black, the DOM hidden: what the overlay pass
 *  lays down (the drop lines; the ground markers are paint in the world). */
async function overlaysOnly(page) {
  const hide = await page.addStyleTag({
    content: "* { visibility: hidden !important } canvas { visibility: visible !important }",
  });
  await page.evaluate(() => window.__lab.setFrameView("overlays-on-black"));
  await page.evaluate(() => window.__lab.frame());
  const shot = decode(await page.screenshot());
  await page.evaluate(() => window.__lab.setFrameView("final"));
  await hide.evaluate((el) => el.remove());
  return shot;
}

export async function run(ctx) {
  const page = await openBattle(ctx);
  const warnings = gpuWarnings(page);
  await hideHud(page);
  await advance(page, 4);
  await presented(page);
  const o = await obs(page);
  const heli = o.own.find((u) => u.kind === "test_heli");
  const jeep = o.own.find((u) => u.kind === "test_jeep");
  const surface = (p) => lab(page, (q) => window.__lab.route.surfaceZ(q[0], q[1]), p);
  const ground = await surface(heli.position);
  const cruise = game.air.cruise_agl_m;
  ctx.check(
    "the helicopter hovers at cruise height over the ground",
    Math.abs(heli.position[2] - ground - cruise) < 0.5,
    JSON.stringify({ at: heli.position, ground, cruise }),
  );

  // Where its shadow falls: its foot, carried along the sun's fall by its height.
  const { sun_azimuth: azimuth, sun_elevation: elevation } = game.presentation.light;
  const lift = heli.position[2] - ground;
  const throwM = lift / Math.tan(elevation);
  const shadow = [
    heli.position[0] - Math.cos(azimuth) * throwM,
    heli.position[1] - Math.sin(azimuth) * throwM,
    ground,
  ];
  // Open ground in the sun beside it, past the reach of its rotor's shadow.
  const aside = 12;
  const sunlit = [
    shadow[0] + Math.sin(azimuth) * aside,
    shadow[1] - Math.cos(azimuth) * aside,
    ground,
  ];
  const css = (p) => lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), p);
  const centre = [heli.position[0], heli.position[1], heli.position[2] + 1.6];
  const underfoot = [heli.position[0], heli.position[1], ground];

  /** Draw the presented tick, save the full frame, the 2× crop holding the
   *  helicopter and its shadow and the 2× crop of airframe, drop line and
   *  marker, and return the decoded frame. */
  const capture = async (name, focus = heli) => {
    await presented(page);
    await page.evaluate(() => window.__lab.frame());
    const shot = await page.screenshot();
    await writeFile(ctx.evidencePath(`frame-${name}-1280x800.png`), shot);
    const png = decode(shot);
    const top = await css([focus.position[0], focus.position[1], focus.position[2] + 1.6]);
    const foot = await css([focus.position[0], focus.position[1], await surface(focus.position)]);
    // The airframe alone, its rotor disc and all.
    await writeCrop(png, ctx.evidencePath(`crop-heli-${name}-2x.png`), top[0], top[1], 170, 90, 2);
    // Airframe, drop line and the whole marker on the ground under it.
    const g = await surface(focus.position);
    const edge = await css([focus.position[0] + MARKER_R, focus.position[1], g]);
    const pad = 1.4 * Math.hypot(edge[0] - foot[0], edge[1] - foot[1]) + 40;
    await writeCrop(
      png,
      ctx.evidencePath(`crop-drop-${name}-2x.png`),
      (top[0] + foot[0]) / 2,
      (top[1] + foot[1]) / 2,
      Math.abs(top[0] - foot[0]) / 2 + pad,
      Math.abs(top[1] - foot[1]) / 2 + pad,
      2,
    );
    if (focus !== heli) return png;
    const [a, b] = [await css(centre), await css(shadow)];
    const [x0, x1] = [Math.min(a[0], b[0]) - 120, Math.max(a[0], b[0]) + 120];
    const [y0, y1] = [Math.min(a[1], b[1]) - 120, Math.max(a[1], b[1]) + 120];
    await writeCrop(
      png,
      ctx.evidencePath(`crop-heli-shadow-${name}-2x.png`),
      (x0 + x1) / 2,
      (y0 + y1) / 2,
      (x1 - x0) / 2,
      (y1 - y0) / 2,
      2,
    );
    return png;
  };

  const first = await capture("t0");
  const at = await css(centre);
  const inside = ([x, y]) => x > 0 && x < 1280 && y > 0 && y < 800;
  ctx.check(
    "the helicopter and its shadow are framed",
    inside(at) && inside(await css(shadow)),
    JSON.stringify({ at, shadow: await css(shadow) }),
  );

  // A tick on, only its rotors have moved: the frame changes round the mast.
  await advance(page, 1);
  const second = await capture("t1");
  await advance(page, 3);
  await capture("t4");
  const turned = mostChanged(first, second, at, 60);
  ctx.check("its rotors turn from one tick to the next", turned > 60, JSON.stringify({ turned }));
  const jeepAt = await css([jeep.position[0], jeep.position[1], jeep.position[2] + 1]);
  const still = mostChanged(first, second, jeepAt, 12);
  ctx.check("the jeep below it does not move", still < 30, JSON.stringify({ still }));

  // Its shadow darkens the ground where the sun casts it, not where it doesn't.
  const [dark, lit] = [
    luminance(first, await css(shadow), 4),
    luminance(first, await css(sunlit), 4),
  ];
  ctx.check(
    "its shadow lies on the ground, off along the sun",
    dark < lit * 0.8,
    JSON.stringify({ dark, lit, shadow, sunlit }),
  );

  // Its drop line stands in the overlay from the ground under it up to the
  // airframe: drawn halfway up, and nowhere a few metres to its side.
  const lineAt = (u, k, g, dz = 0) => [
    u.position[0],
    u.position[1],
    g + (u.position[2] - g) * k + dz,
  ];
  /** The brightest of `u`'s drop line round height share `k` over ground `g`. */
  const lineNear = async (png, u, k, g, r) =>
    brightestAlong(
      png,
      await css(lineAt(u, k, g, -DROP_SPAN_M)),
      await css(lineAt(u, k, g, DROP_SPAN_M)),
      r,
    );
  const overlays = await overlaysOnly(page);
  const [low, side, high] = [
    await lineNear(overlays, heli, DROP_LOW, ground, 3),
    brightest(overlays, await css([heli.position[0] + 4, heli.position[1], ground + lift / 2]), 3),
    await lineNear(overlays, heli, (DROP_REACH + 1) / 2 + 0.1, ground, 3),
  ];
  ctx.check(
    "a dotted drop line rises from the ground under the airframe, and fades out short of it",
    sum(low) > 120 && sum(side) < 20 && sum(high) < 20,
    JSON.stringify({ low, side, high }),
  );

  // Selected, its marker on the ground takes the selection's colour: its
  // rim, a marker's radius from the point under the airframe, changes.
  const camera = await lab(page, () => window.__lab.camera());
  const toward = camera.yaw + Math.PI;
  const rim = await css([
    underfoot[0] + Math.cos(toward) * MARKER_R,
    underfoot[1] + Math.sin(toward) * MARKER_R,
    ground,
  ]);
  await lab(page, (id) => window.__lab.route.select([id]), heli.id);
  await advance(page, 1);
  const selected = await capture("selected");
  await lab(page, () => window.__lab.route.select([]));
  ctx.check(
    "its ground marker is drawn round the point under it, in the selection's colour when selected",
    mostChanged(second, selected, rim, 6) > 60,
    JSON.stringify({ rim, changed: mostChanged(second, selected, rim, 6) }),
  );

  // Ordered away with Space held, its ghost flies at cruise height over its
  // destination, not landed there: drawn up there, and not on the ground
  // beside its drop line, which joins it to the destination's marker.
  const goal = [100, 125];
  await lab(
    page,
    ([id, goal]) =>
      window.__lab.route.command({
        kind: "move",
        units: [id],
        gesture: 1201,
        goal,
        route: "shortest",
      }),
    [heli.id, goal],
  );
  await advance(page, 1);
  await aim(page, goal, { distance: 70, pitch: 0.72, yaw: -1.25 });
  const unheld = decode(await snapshot(ctx, page, "frame-ghost-unheld-1280x800.png"));
  await page.keyboard.down("Space");
  await advance(page, 1);
  const held = await capture("ghost", {
    position: [goal[0], goal[1], (await surface(goal)) + cruise],
  });
  const ghostLines = await overlaysOnly(page);
  await page.keyboard.up("Space");
  const goalGround = await surface(goal);
  const [aloft, landed] = [
    mostChanged(unheld, held, await css([goal[0], goal[1], goalGround + cruise + 1]), 8),
    mostChanged(unheld, held, await css([goal[0] + 4, goal[1], goalGround + 1]), 3),
  ];
  ctx.check(
    "with Space held, its ghost flies at cruise height over its destination",
    aloft > 60 && landed < aloft / 2,
    JSON.stringify({ aloft, landed }),
  );
  const ghostLine = brightestAlong(
    ghostLines,
    await css([goal[0], goal[1], goalGround + cruise * DROP_LOW - DROP_SPAN_M]),
    await css([goal[0], goal[1], goalGround + cruise * DROP_LOW + DROP_SPAN_M]),
    3,
  );
  ctx.check(
    "a drop line joins the ghost to its destination's marker",
    sum(ghostLine) > 120,
    JSON.stringify({ ghostLine }),
  );

  // Over a roof, an identified enemy beyond the road.
  const r = await begin(page, "roof");
  const roofHeli = r.own.find((u) => u.kind === "test_heli");
  const enemy = r.identified.find((u) => u.kind === "test_heli");
  // The house it hovers over: the building whose parts centre nearest it.
  const houses = await lab(page, () => window.__lab.route.buildings());
  const near = (b) =>
    Math.min(
      ...b.authored.map((p) =>
        Math.hypot(p.center[0] - roofHeli.position[0], p.center[1] - roofHeli.position[1]),
      ),
    );
  const house = houses.reduce((a, b) => (near(b) < near(a) ? b : a));
  const roofTop = Math.max(...house.authored.map((p) => p.baseZ + 2 * p.half[2]));
  await aim(page, [257, 462, 12], { distance: 110, pitch: 0.72, yaw: -1.25 }, { onGround: false });
  await advance(page, 1);
  await capture("roof", roofHeli);
  await capture("enemy", enemy);
  const roofOverlays = await overlaysOnly(page);
  const roofGround = await surface(roofHeli.position);
  const onLine = (u, z) => css([u.position[0], u.position[1], z]);
  const [above, below] = [
    brightestAlong(
      roofOverlays,
      // Its dots between the roof and where they fade out.
      await onLine(roofHeli, roofTop + 0.2),
      await onLine(roofHeli, roofGround + (roofHeli.position[2] - roofGround) * DROP_REACH),
      2,
    ),
    brightest(roofOverlays, await onLine(roofHeli, (roofGround + roofTop) / 2), 2),
  ];
  ctx.check(
    "over a roof, the drop line stands down to the roof and the roof hides it below",
    sum(above) > 120 && sum(below) < 20,
    JSON.stringify({ above, below, roofTop }),
  );
  const red = (c) => c[0] > 100 && c[0] > 1.6 * c[1];
  const enemyMid = await lineNear(roofOverlays, enemy, DROP_LOW, await surface(enemy.position), 3);
  ctx.check(
    "an identified enemy helicopter's drop line is drawn, red",
    red(enemyMid),
    JSON.stringify({ enemyMid }),
  );

  // From the map's zoom: the airframes are small and the marks still say
  // where each is over the ground.
  await aim(page, [258, 469], { distance: 350, pitch: 0.85, yaw: -1.25 });
  await advance(page, 1);
  await capture("map-roof", roofHeli);
  const mapOverlays = await overlaysOnly(page);
  // Its dots that show, over the roof: from the roof up to where they fade out.
  const mapMid = brightestAlong(
    mapOverlays,
    await onLine(roofHeli, roofTop + 0.2),
    await onLine(roofHeli, roofGround + (roofHeli.position[2] - roofGround) * DROP_REACH),
    2,
  );
  ctx.check(
    "from the map's zoom the drop line is still drawn",
    sum(mapMid) > 120,
    JSON.stringify({ mapMid }),
  );

  await apache(ctx, page, css, surface);
  await lostSign(ctx, page, surface, css);

  // An enemy over ground blue cannot see: a low house hides the ground behind
  // it from blue's jeep, not the airframe above it (V02). Its marker lies on
  // the fogged ground and its drop line stands over it.
  const f = await begin(page, "fog");
  const hidden = f.identified.find((u) => u.kind === "test_heli");
  ctx.check(
    "the enemy helicopter over the fog is identified",
    !!hidden,
    JSON.stringify(f.identified),
  );
  if (!hidden) return;
  const hiddenGround = await surface(hidden.position);
  for (const [name, view] of [
    ["fog", { distance: 75, pitch: 0.72, yaw: -1.25 }],
    ["map-fog", { distance: 350, pitch: 0.85, yaw: -1.25 }],
  ]) {
    await aim(page, [hidden.position[0] - 20, hidden.position[1]], view);
    await advance(page, 1);
    await capture(name, hidden);
    const underCss = await css([hidden.position[0], hidden.position[1], hiddenGround]);
    await page.evaluate(() => window.__lab.setFrameView("fog-mask"));
    await page.evaluate(() => window.__lab.frame());
    const mask = decode(await page.screenshot());
    await page.evaluate(() => window.__lab.setFrameView("final"));
    const fogged = sum(pixel(mask, underCss[0], underCss[1])) < 60;
    const fogOverlays = await overlaysOnly(page);
    const mid = await lineNear(fogOverlays, hidden, DROP_LOW, hiddenGround, 2);
    ctx.check(
      `${name}: over unseen ground, the enemy's drop line is drawn, red`,
      fogged && red(mid),
      JSON.stringify({ fogged, mid }),
    );
  }

  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}

/** The AH-64E on its real art, hovering at cruise height over the verge, its
 *  chin gun firing down at enemy tanks on the road 45 to 67 m off: the gun
 *  drawn down along the published elevation, and its muzzle, where the flash
 *  is drawn, at the barrel's tip where the round leaves. */
async function apache(ctx, page, css, surface) {
  await lab(page, (v) => window.__lab.route.variant(v), "apache");
  await page.waitForFunction(() => window.__lab.route?.reset);
  await lab(page, () => window.__lab.route.reset());
  await page.waitForFunction(() => window.__lab.route?.tick() > 0);
  await lab(page, () => window.__lab.route.pause());
  // Tick by tick to the chin gun's second round, at the tick its flash is
  // drawn: a drawn gun is level until its first round and eases down to it
  // after, so by the second it is drawn down where it fires (the next tank
  // is a few degrees off the first).
  const CHIN = 0;
  const chinRound = (o) => {
    const shots = o.own.find((u) => u.kind === "ah_64e_guardian")?.weaponPoses[CHIN].shots ?? 0;
    return shots > 1
      ? o.projectiles.find((p) => p.own && p.kind === "autocannon" && p.shooterMember === null)
      : undefined;
  };
  let o = await obs(page);
  for (let t = 0; t < 30 * game.tick_hz && !chinRound(o); t++) {
    await advance(page, 1);
    o = await obs(page);
  }
  const heli = o.own.find((u) => u.kind === "ah_64e_guardian");
  const round = chinRound(o);
  ctx.check(
    "the Apache's chin gun fires within 30 s",
    !!round,
    JSON.stringify({ heli, identified: o.identified, tick: o.tick }),
  );
  if (!round) return;
  const ground = await surface(heli.position);
  ctx.check(
    "the Apache hovers at cruise height",
    Math.abs(heli.position[2] - ground - game.air.cruise_agl_m) < 1,
    JSON.stringify({ at: heli.position, ground }),
  );
  const pose = heli.weaponPoses.find((w) => w.mount === CHIN);
  const deg = (r) => (r * 180) / Math.PI;
  ctx.check(
    "its chin gun fires down at the tank, past a tank gun's -10°",
    deg(pose.elevation) < -15,
    JSON.stringify({ elevation_deg: deg(pose.elevation) }),
  );

  // From low off its right bow, under the chin: the gun and the barrel's tip
  // clear of the nose and the stores.
  const chinAt = [
    heli.position[0] + 4.6 * Math.cos(heli.yaw),
    heli.position[1] + 4.6 * Math.sin(heli.yaw),
    heli.position[2] + 0.8,
  ];
  await aim(page, chinAt, { distance: 16, pitch: 0.34, yaw: heli.yaw - 0.9 }, { onGround: false });
  await presented(page);
  await page.evaluate(() => window.__lab.frame());
  const muzzle = await lab(page, (id) => window.__lab.route.drawnMuzzle(id, 0), heli.id);
  ctx.check("the chin gun's muzzle is drawn", !!muzzle, JSON.stringify({ muzzle }));
  if (!muzzle) return;
  // The simulation launches the round level with the bore at the muzzle's
  // reach along its bearing. The drawn barrel pivots on the same bore, pitched
  // down along the shot, so its tip is that reach along the shot's direction,
  // run back along it by at most the drawn recoil.
  const reach = unitType("ah_64e_guardian").mounts[CHIN].muzzle_m[0];
  const launch = round.path[0];
  const [b, e] = [pose.bearing, pose.elevation];
  const level = [Math.cos(b), Math.sin(b), 0];
  const bore = [Math.cos(e) * Math.cos(b), Math.cos(e) * Math.sin(b), Math.sin(e)];
  const tip = [0, 1, 2].map((k) => launch[k] - reach * level[k] + reach * bore[k]);
  const back = Math.min(
    game.presentation.pose.mount.recoil_m,
    Math.max(
      0,
      [0, 1, 2].reduce((s, k) => s + (tip[k] - muzzle[k]) * bore[k], 0),
    ),
  );
  const off = Math.hypot(...[0, 1, 2].map((k) => tip[k] - back * bore[k] - muzzle[k]));
  ctx.check(
    "the drawn muzzle is at the barrel's tip, pitched down along the shot from where the round leaves",
    off < 0.1,
    JSON.stringify({ muzzle, launch, tip, recoiled_m: back, off }),
  );
  const shot = await page.screenshot();
  await writeFile(ctx.evidencePath("frame-apache-firing-1280x800.png"), shot);
  const png = decode(shot);
  const onScreen = await css(muzzle);
  await writeCrop(
    png,
    ctx.evidencePath("crop-apache-chin-gun-2x.png"),
    onScreen[0],
    onScreen[1],
    110,
    70,
    2,
  );
  const nose = await css([heli.position[0], heli.position[1], heli.position[2] + 1.5]);
  await writeCrop(
    png,
    ctx.evidencePath("crop-apache-airframe-2x.png"),
    nose[0],
    nose[1],
    230,
    130,
    2,
  );
  // The flash's bright core: within half a metre of the barrel's tip, out
  // along the bore where the round leaves.
  const out = await css([0, 1, 2].map((k) => muzzle[k] + 0.5 * bore[k]));
  const halfMetre = Math.hypot(out[0] - onScreen[0], out[1] - onScreen[1]);
  const fire = ([R, , B]) => R > 200 && R - B > 60;
  ctx.check(
    "the muzzle flash is drawn at the barrel's tip",
    anyNear(png, out, Math.max(4, halfMetre), fire),
    JSON.stringify({ onScreen, out, halfMetre }),
  );
  // The whole scene, from the battle's camera: the airframe over the verge,
  // its drop line, the tank under fire on the road.
  const tank = o.identified.find((u) => u.kind === "test_tank") ?? null;
  const mid = tank
    ? [(heli.position[0] + tank.position[0]) / 2, (heli.position[1] + tank.position[1]) / 2, 8]
    : heli.position;
  await aim(page, mid, { distance: 70, pitch: 0.55, yaw: -1.25 }, { onGround: false });
  await presented(page);
  await page.evaluate(() => window.__lab.frame());
  await writeFile(ctx.evidencePath("frame-apache-wide-1280x800.png"), await page.screenshot());
}

/** How far the overlay's marks reach from `p` along (dx, dy) on the
 *  overlays-on-black shot: until 8 dark pixels in a row (the sign's dark
 *  keyline is narrower than that). */
function extent(png, p, dx, dy) {
  let dark = 0;
  let last = 0;
  for (let k = 1; k < 600; k++) {
    const [x, y] = [p[0] + dx * k, p[1] + dy * k];
    if (x < 0 || y < 0 || x >= png.width || y >= png.height) break;
    if (sum(pixel(png, x, y)) > 60) {
      dark = 0;
      last = k;
    } else if (++dark >= 8) break;
  }
  return last;
}

/** An enemy helicopter blue's jeep saw across the field and lost behind the
 *  house (D11, slice 13): its last sighting hangs in the air where it was
 *  last seen, a red disc facing the camera at the airframe's height, over
 *  fogged ground and, seen from beyond, over the house's roof. */
async function lostSign(ctx, page, surface, css) {
  const o = await begin(page, "lost", "aloft");
  const sign = o.contacts.find((c) => c.aloft);
  ctx.check(
    "the lost helicopter leaves an aloft contact at its height, out of sight",
    !!sign &&
      sign.layer === "low_air" &&
      Math.abs(sign.center[2] - (await surface(sign.center)) - game.air.cruise_agl_m) < 1 &&
      !o.identified.some((u) => u.kind === "test_heli"),
    JSON.stringify({ contacts: o.contacts, identified: o.identified.map((u) => u.kind) }),
  );
  if (!sign) return;
  const at = sign.center;
  // The building it was lost behind: the one whose parts centre nearest the sign.
  const buildings = await lab(page, () => window.__lab.route.buildings());
  const near = (b) =>
    Math.min(...b.authored.map((p) => Math.hypot(p.center[0] - at[0], p.center[1] - at[1])));
  const house = buildings.reduce((a, b) => (near(b) < near(a) ? b : a));
  const roofTop = Math.max(...house.authored.map((p) => p.baseZ + 2 * p.half[2]));
  const footprint = house.authored[0];
  const { min_radius_px: smallest, max_radius_px: largest } = game.presentation.contacts.air;
  // Strokes thin to this share of their width as the camera pulls out.
  const thin = game.presentation.overlay.stroke.thin_scale;
  // From the south-east the sign hangs over the fogged field beyond the
  // house; from across the sign, low, the shop the jeep stands against lies behind it. Close
  // in and from the map's zoom.
  const southEast = { pitch: 0.72, yaw: -1.25 };
  // Facing the building across the sign, low enough that the eye's line
  // through the sign comes down onto its roof.
  const away = [at[0] - footprint.center[0], at[1] - footprint.center[1]];
  const facing = {
    pitch: Math.atan2(at[2] - roofTop, Math.hypot(away[0], away[1])),
    yaw: Math.atan2(away[1], away[0]),
  };
  for (const [name, view] of [
    ["lost-fog", { distance: 90, ...southEast }],
    ["lost-roof", { distance: 90, ...facing }],
    ["lost-map-fog", { distance: 350, ...southEast }],
    ["lost-map-roof", { distance: 350, ...facing }],
  ]) {
    await aim(page, at, view, { onGround: false });
    await advance(page, 1);
    await presented(page);
    const shot = decode(await snapshot(ctx, page, `frame-${name}-1280x800.png`));
    const c = await css(at);
    const overlays = await overlaysOnly(page);
    // Across, and twice upward: below it the stem carries on down.
    const [w, h] = [
      extent(overlays, c, 1, 0) + extent(overlays, c, -1, 0),
      2 * extent(overlays, c, 0, -1),
    ];
    // The sign and three times its size round it, enlarged 3×.
    const r = Math.min(w, h) / 2;
    await writeCrop(shot, ctx.evidencePath(`crop-sign-${name}-3x.png`), c[0], c[1], 3 * r, 3 * r);
    const middle = brightest(overlays, c, 2);
    ctx.check(
      `${name}: the sign is a red disc round the contact's centre, a circle on screen, held between its smallest and largest`,
      middle[0] > 100 &&
        middle[0] > 1.6 * middle[1] &&
        Math.abs(w / h - 1) < 0.12 &&
        w > 2 * smallest * thin &&
        w < 2 * largest + 24,
      JSON.stringify({ middle, w, h }),
    );
    const under = await css([at[0], at[1], await surface(at)]);
    if (under[1] - c[1] > r + 20) {
      const stem = brightest(overlays, [c[0], (c[1] + r + under[1]) / 2], 2);
      ctx.check(
        `${name}: a pale stem stands from the ground under the sign up to it`,
        Math.min(...stem) > 100,
        JSON.stringify({ stem, c, under, r }),
      );
    }
    if (name === "lost-fog") {
      await page.evaluate(() => window.__lab.setFrameView("fog-mask"));
      await page.evaluate(() => window.__lab.frame());
      const mask = decode(await page.screenshot());
      await page.evaluate(() => window.__lab.setFrameView("final"));
      ctx.check(
        "lost-fog: the ground under the sign is unseen",
        sum(pixel(mask, under[0], under[1])) < 60,
        JSON.stringify({ under }),
      );
    }
    if (name.endsWith("roof")) {
      // The eye's line through the sign's centre carries on down to the
      // roof's height inside the house's footprint.
      const t = (at[2] - roofTop) / Math.tan(view.pitch);
      const behind = [at[0] - t * Math.cos(view.yaw), at[1] - t * Math.sin(view.yaw)];
      const [dx, dy] = [behind[0] - footprint.center[0], behind[1] - footprint.center[1]];
      const inside = [footprint.half[0], footprint.half[1]].map((half, k) => {
        const axis =
          k === 0
            ? [Math.cos(footprint.yaw), Math.sin(footprint.yaw)]
            : [-Math.sin(footprint.yaw), Math.cos(footprint.yaw)];
        return Math.abs(dx * axis[0] + dy * axis[1]) <= half;
      });
      ctx.check(
        `${name}: the house's roof lies behind the sign`,
        inside.every(Boolean),
        JSON.stringify({ behind, footprint }),
      );
    }
  }
}

/** Switch to `variant` (a fresh battle), let it run until blue identifies
 *  an enemy helicopter, or with `until` "aloft" holds an aloft contact (at
 *  most 30 s), pause it and return its observation. */
async function begin(page, variant, until = "identified") {
  await lab(page, (v) => window.__lab.route.variant(v), variant);
  await page.waitForFunction(() => window.__lab.route?.reset);
  await lab(page, () => window.__lab.route.reset());
  await page.waitForFunction(
    (until) => {
      const o = window.__lab.route?.observation();
      if (!(window.__lab.route?.tick() > 3 && o)) return false;
      return until === "aloft"
        ? o.contacts.some((c) => c.aloft)
        : o.identified.some((u) => u.kind === "test_heli");
    },
    until,
    { timeout: 30000 },
  );
  await lab(page, () => window.__lab.route.pause());
  return obs(page);
}
