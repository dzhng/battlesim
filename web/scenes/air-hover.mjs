// The test helicopter hovering at cruise height: it is drawn up there, its
// rotors turn from frame to frame, and its shadow lies on the ground below
// and off along the sun. The pose driver's and model detail's tests own the
// rotor arithmetic and the shadow's culling; this judges the drawn frame.
import { writeFile } from "node:fs/promises";
import { decode, mostChanged, pixel, around, writeCrop } from "./_png.mjs";
import { lab, obs, advance, presented, openBattle, gpuWarnings, hideHud } from "./_lab.mjs";
import { game } from "./_units.mjs";

/** The side's helicopter and jeep, as observed. */
const units = async (page) => {
  const o = await obs(page);
  return {
    heli: o.own.find((u) => u.kind === "test_heli"),
    jeep: o.own.find((u) => u.kind === "test_jeep"),
  };
};

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

export async function run(ctx) {
  const page = await openBattle(ctx);
  const warnings = gpuWarnings(page);
  await hideHud(page);
  await advance(page, 4);
  await presented(page);
  const { heli, jeep } = await units(page);
  const ground = await lab(page, (p) => window.__lab.route.surfaceZ(p[0], p[1]), heli.position);
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

  /** Draw the presented tick, save the full frame and the 2× crop holding the
   *  helicopter and its shadow, and return the decoded frame. */
  const capture = async (name) => {
    await presented(page);
    await page.evaluate(() => window.__lab.frame());
    const shot = await page.screenshot();
    await writeFile(ctx.evidencePath(`frame-${name}-1280x800.png`), shot);
    const png = decode(shot);
    const [a, b] = [await css(centre), await css(shadow)];
    // The airframe alone, its rotor disc and all.
    await writeCrop(png, ctx.evidencePath(`crop-heli-${name}-2x.png`), a[0], a[1], 170, 90, 2);
    const pad = 120;
    const [x0, x1] = [Math.min(a[0], b[0]) - pad, Math.max(a[0], b[0]) + pad];
    const [y0, y1] = [Math.min(a[1], b[1]) - pad, Math.max(a[1], b[1]) + pad];
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
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
