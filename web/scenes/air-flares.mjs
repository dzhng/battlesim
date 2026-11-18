// One physical flare and its diverted missile, through the shared battle view.
import { advance, gpuWarnings, lab, obs, openBattle } from "./_lab.mjs";
import { shoot } from "./_air.mjs";

export async function run(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1252, height: 808 } });
  const warnings = gpuWarnings(page);
  try {
    let o = await obs(page);
    for (let i = 0; i < 500 && !o.guided.length; i++) {
      await advance(page, 1);
      o = await obs(page);
    }
    ctx.check("the launcher fires", o.guided.length > 0);
    if (!o.guided.length) return;
    const launch = o.tick;
    await advance(page, 1);
    o = await obs(page);
    const flare = o.flares[0];
    const missile = o.guided[0];
    const helicopter = o.identified.find((u) => u.kind === "test_heli");
    ctx.check(
      "the diverted missile follows a falling flare beside the helicopter",
      !!flare &&
        !!missile &&
        !!helicopter &&
        !missile.supported &&
        flare.to[2] < flare.from[2] &&
        Math.hypot(...missile.point.map((v, k) => v - helicopter.position[k])) < 30,
      JSON.stringify({ flare, missile, helicopter: helicopter?.position }),
    );
    const moments = [];
    for (const after of [10, 30, 55]) {
      await advance(page, launch + after - o.tick);
      o = await obs(page);
      moments.push({ tick: o.tick, flares: o.flares, guided: o.guided });
      for (const [name, target, distance] of [
        ["wide", [1600, 1500, 0], 750],
        ["heli", [1800, 1500, 0], 217.67386166416838],
      ]) {
        await lab(
          page,
          ({ target, distance }) =>
            window.__lab.setCamera({
              ...window.__lab.camera(),
              target,
              distance,
              pitch: 0.9581375216268732,
              yaw: Math.PI / 2,
            }),
          { target, distance },
        );
        await shoot(ctx, page, `${name}-${after}`);
      }
    }
    for (let i = 0; i < 180 && o.guided.length; i++) {
      await advance(page, 1);
      o = await obs(page);
    }
    moments.push({ tick: o.tick, flares: o.flares, guided: o.guided, blasts: o.blasts });
    ctx.check(
      "the diverted missile bursts beside the helicopter's flare",
      o.guided.length === 0 &&
        o.blasts.some((b) => Math.hypot(...b.point.map((v, k) => v - helicopter.position[k])) < 80),
      JSON.stringify(moments.at(-1)),
    );
    await shoot(ctx, page, "heli-miss");
    await ctx.writeEvidence("flare-moments.json", moments);
    ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
  } finally {
    await page.close();
  }
}
