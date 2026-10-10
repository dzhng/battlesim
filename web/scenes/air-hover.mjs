// Test helicopters hovering at cruise height: drawn up there, rotors turning
// from frame to frame, shadow on the ground below and off along the sun, and
// where each is over the ground read from its ground marker and drop line
// (D18): over open ground, over a roof, an identified enemy's, selected and
// not, and from the map's zoom. The pose driver's and model detail's tests
// own the rotor arithmetic and the shadow's culling, the overlay's tests the
// marks' geometry; this judges the drawn frame.
import { writeFile } from "node:fs/promises";
import { decode, mostChanged, pixel, around, writeCrop } from "./_png.mjs";
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
import { game, hull } from "./_units.mjs";

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
  const lineAt = (u, k, g) => [u.position[0], u.position[1], g + (u.position[2] - g) * k];
  const overlays = await overlaysOnly(page);
  const [mid, side] = [
    brightest(overlays, await css(lineAt(heli, 0.5, ground)), 3),
    brightest(overlays, await css([heli.position[0] + 4, heli.position[1], ground + lift / 2]), 3),
  ];
  ctx.check(
    "a drop line joins the airframe to the ground under it",
    sum(mid) > 120 && sum(side) < 20,
    JSON.stringify({ mid, side }),
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
  const ghostLine = brightest(
    ghostLines,
    await css([goal[0], goal[1], goalGround + cruise / 2]),
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
  const house = await lab(page, () => window.__lab.route.buildings()[0]);
  const roofTop = Math.max(...house.authored.map((p) => p.baseZ + 2 * p.half[2]));
  await aim(page, [155, 470, 12], { distance: 110, pitch: 0.72, yaw: -1.25 }, { onGround: false });
  await advance(page, 1);
  await capture("roof", roofHeli);
  await capture("enemy", enemy);
  const roofOverlays = await overlaysOnly(page);
  const roofGround = await surface(roofHeli.position);
  const onLine = (u, z) => css([u.position[0], u.position[1], z]);
  const [above, below] = [
    brightest(roofOverlays, await onLine(roofHeli, (roofTop + roofHeli.position[2]) / 2), 2),
    brightest(roofOverlays, await onLine(roofHeli, (roofGround + roofTop) / 2), 2),
  ];
  ctx.check(
    "over a roof, the drop line stands down to the roof and the roof hides it below",
    sum(above) > 120 && sum(below) < 20,
    JSON.stringify({ above, below, roofTop }),
  );
  const red = (c) => c[0] > 100 && c[0] > 1.6 * c[1];
  const enemyMid = brightest(
    roofOverlays,
    await css(lineAt(enemy, 0.5, await surface(enemy.position))),
    3,
  );
  ctx.check(
    "an identified enemy helicopter's drop line is drawn, red",
    red(enemyMid),
    JSON.stringify({ enemyMid }),
  );

  // From the map's zoom: the airframes are small and the marks still say
  // where each is over the ground.
  await aim(page, [156, 477], { distance: 350, pitch: 0.85, yaw: -1.25 });
  await advance(page, 1);
  await capture("map-roof", roofHeli);
  const mapOverlays = await overlaysOnly(page);
  const mapMid = brightest(mapOverlays, await css(lineAt(roofHeli, 0.85, roofGround)), 2);
  ctx.check(
    "from the map's zoom the drop line is still drawn",
    sum(mapMid) > 120,
    JSON.stringify({ mapMid }),
  );

  // An enemy over ground blue cannot see: the house hides the ground behind
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
    await aim(page, [270, 60], view);
    await advance(page, 1);
    await capture(name, hidden);
    const underCss = await css([hidden.position[0], hidden.position[1], hiddenGround]);
    await page.evaluate(() => window.__lab.setFrameView("fog-mask"));
    await page.evaluate(() => window.__lab.frame());
    const mask = decode(await page.screenshot());
    await page.evaluate(() => window.__lab.setFrameView("final"));
    const fogged = sum(pixel(mask, underCss[0], underCss[1])) < 60;
    const fogOverlays = await overlaysOnly(page);
    const mid = brightest(fogOverlays, await css(lineAt(hidden, 0.85, hiddenGround)), 2);
    ctx.check(
      `${name}: over unseen ground, the enemy's drop line is drawn, red`,
      fogged && red(mid),
      JSON.stringify({ fogged, mid }),
    );
  }

  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}

/** Switch to `variant` (a fresh battle), let it run until blue identifies
 *  an enemy helicopter (at most 30 s), pause it and return its observation. */
async function begin(page, variant) {
  await lab(page, (v) => window.__lab.route.variant(v), variant);
  await page.waitForFunction(() => window.__lab.route?.reset);
  await lab(page, () => window.__lab.route.reset());
  await page.waitForFunction(
    () =>
      window.__lab.route?.tick() > 3 &&
      window.__lab.route.observation()?.identified.some((u) => u.kind === "test_heli"),
    undefined,
    { timeout: 30000 },
  );
  await lab(page, () => window.__lab.route.pause());
  return obs(page);
}
