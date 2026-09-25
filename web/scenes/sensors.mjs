// Slice 05: visible versus obstructed ground; only identified enemies exist.
import { writeFile } from "node:fs/promises";
import { decode, pixel, writeCrop } from "./_png.mjs";
import { lab } from "./_lab.mjs";

const luminance = ([r, g, b]) => 0.3 * r + 0.5 * g + 0.2 * b;

/** Enemy instances drawn vs soldiers/vehicles the identified list accounts for. */
const enemyAccounting = (page) =>
  lab(page, () => {
    const obs = window.__lab.route.observation();
    const own = new Set(obs.own.map((u) => u.id));
    const expected = obs.identified.reduce((n, e) => n + Math.max(1, e.members.length), 0);
    const drawn =
      window.__lab.instances().length -
      obs.own.reduce((n, u) => n + Math.max(1, u.members.length), 0);
    return { expected, drawn, identified: obs.identified.length, own: own.size };
  });

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 20000 });
  await lab(page, () => window.__lab.route.pause());
  await lab(page, () => window.__lab.route.advance(2));
  await page.evaluate(() => window.__lab.frame());

  const start = await enemyAccounting(page);
  ctx.check(
    "drawn enemies are exactly the identified ones",
    start.drawn === start.expected && start.identified > 0,
    JSON.stringify(start),
  );

  // Fog: hidden ground darkens, visible ground does not change.
  const hidden = await lab(page, () => window.__lab.projectToCss(840, 470, 0));
  const open = await lab(page, () => window.__lab.projectToCss(330, 300, 0));
  const fogged = decode(await page.screenshot());
  await writeFile(ctx.evidencePath("frame-fog-1280x800.png"), await page.screenshot());
  await lab(page, () => window.__lab.route.setFog(false));
  await page.evaluate(() => window.__lab.frame());
  const clear = decode(await page.screenshot());
  await lab(page, () => window.__lab.route.setFog(true));
  const dHidden = luminance(pixel(clear, ...hidden)) - luminance(pixel(fogged, ...hidden));
  const dOpen = Math.abs(luminance(pixel(clear, ...open)) - luminance(pixel(fogged, ...open)));
  ctx.check(
    "fog darkens ground behind the ridge and leaves seen ground alone",
    dHidden > 20 && dOpen < 3,
    JSON.stringify({ dHidden, dOpen }),
  );
  const edge = await lab(page, () => window.__lab.projectToCss(420, 100, 0));
  await writeCrop(fogged, ctx.evidencePath("crop-forest-edge-3x.png"), edge[0], edge[1], 90, 60, 3);
  const hill = await lab(page, () => window.__lab.projectToCss(700, 480, 10));
  await writeCrop(fogged, ctx.evidencePath("crop-hillside-2x.png"), hill[0], hill[1], 140, 90, 2);

  // Drive the scripted tour until red's tank is behind the ridge.
  let behind = null;
  for (let i = 0; i < 320 && !behind; i++) {
    await lab(page, () => window.__lab.route.advance(15));
    const s = await lab(page, () => window.__lab.route.tick());
    if (s > 30) {
      await lab(page, () => window.__lab.route.setSide("red"));
      await lab(page, () => window.__lab.route.advance(1));
      const tank = await lab(page, () =>
        window.__lab.route.observation().own.find((u) => u.kind === "tank"),
      );
      await lab(page, () => window.__lab.route.setSide("blue"));
      await lab(page, () => window.__lab.route.advance(1));
      if (tank && Math.hypot(tank.position[0] - 820, tank.position[1] - 440) < 15)
        behind = tank.position;
    }
  }
  ctx.check(
    "the scripted red tank reaches the ridge's far side",
    behind !== null,
    JSON.stringify(behind),
  );
  await page.evaluate(() => window.__lab.frame());
  const hiddenNow = await lab(page, () =>
    window.__lab.route.observation().identified.filter((e) => e.kind === "tank"),
  );
  const accounting = await enemyAccounting(page);
  ctx.check(
    "behind the ridge it is neither listed nor drawn",
    hiddenNow.length === 0 && accounting.drawn === accounting.expected,
    JSON.stringify({ hiddenNow, accounting }),
  );
  await writeFile(ctx.evidencePath("frame-ridge-hidden-1280x800.png"), await page.screenshot());

  // Side switch is explicit and swaps the whole view.
  await lab(page, () => window.__lab.route.setSide("red"));
  await lab(page, () => window.__lab.route.advance(1));
  await page.evaluate(() => window.__lab.frame());
  const label = await page.getByTestId("viewing-as").textContent();
  const redOwn = await lab(page, () =>
    window.__lab.route
      .observation()
      .own.map((u) => u.kind)
      .sort(),
  );
  ctx.check(
    "the side switch is labelled and shows red's own units",
    /Viewing as RED/.test(label) && redOwn.join() === "rifle,tank",
    `${label} ${redOwn}`,
  );
  await lab(page, () => window.__lab.route.setSide("blue"));
  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [1280, 800],
    dpr: 1,
  });
}
