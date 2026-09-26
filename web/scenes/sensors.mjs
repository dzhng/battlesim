// Slice 05: visible versus obstructed ground; only identified enemies exist.
import { writeFile } from "node:fs/promises";
import { decode, pixel, writeCrop } from "./_png.mjs";
import { advance, lab } from "./_lab.mjs";

const luminance = ([r, g, b]) => 0.3 * r + 0.5 * g + 0.2 * b;
/** Rec. 709 luminance, the fog look's own: cooling toward night shifts hue at
 *  equal luminance, so blue must weigh no more than the eye gives it. */
const luma709 = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;

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
  await advance(page, 2);
  await page.evaluate(() => window.__lab.frame());

  const start = await enemyAccounting(page);
  ctx.check(
    "drawn enemies are exactly the identified ones",
    start.drawn === start.expected && start.identified > 0,
    JSON.stringify(start),
  );

  // Fog: hidden ground changes, visible ground does not.
  const hidden = await lab(page, () => window.__lab.projectToCss(840, 470, 0));
  const open = await lab(page, () => window.__lab.projectToCss(330, 300, 0));
  const fogged = decode(await page.screenshot());
  await writeFile(ctx.evidencePath("frame-fog-1280x800.png"), await page.screenshot());
  await lab(page, () => window.__lab.route.setFog(false));
  await page.evaluate(() => window.__lab.frame());
  const clear = decode(await page.screenshot());
  await lab(page, () => window.__lab.route.setFog(true));
  const hiddenClear = luma709(pixel(clear, ...hidden));
  const dHidden = hiddenClear - luma709(pixel(fogged, ...hidden));
  const dOpen = Math.abs(luma709(pixel(clear, ...open)) - luma709(pixel(fogged, ...open)));
  // Relative, since fog applies before tone mapping (battle-look slice 12,
  // decisions.md): a visible step, not a fixed display-value drop. Either
  // way (slice 15b): a style may veil unseen lighter than it was, as the
  // fixture's `veil` does, so it never reads as a shadow.
  ctx.check(
    "fog changes ground behind the ridge and leaves seen ground alone",
    Math.abs(dHidden) / hiddenClear > 0.05 && dOpen < 3,
    JSON.stringify({
      hiddenClear,
      dHidden,
      dOpen,
      clear: pixel(clear, ...hidden),
      fogged: pixel(fogged, ...hidden),
    }),
  );
  // The sight-lobe overlay: blue's tank faces east, so its outline reaches
  // the full range ahead and much less astern; lobes toggle off cleanly.
  // (Astern lies off screen, so the drawn check uses the side reach.)
  const tankSight = await lab(
    page,
    () => window.__lab.route.observation().own.find((u) => u.kind === "tank").sight,
  );
  const lobePoint = (off, inset) => {
    const { eyes, forward, shape, range } = tankSight;
    const c = Math.cos(off);
    const m = shape.side * (1 - c * c) + (c >= 0 ? shape.front : shape.rear) * c * c;
    const r = range * m - inset;
    return [eyes[0][0] + Math.cos(forward + off) * r, eyes[0][1] + Math.sin(forward + off) * r];
  };
  const ahead = lobePoint(0, 1.5);
  const astern = lobePoint(Math.PI, 1.5);
  ctx.check(
    "the tank's lobe reaches farther ahead than astern",
    tankSight.shape.front > tankSight.shape.rear &&
      Math.hypot(ahead[0] - tankSight.eyes[0][0], ahead[1] - tankSight.eyes[0][1]) >
        2 * Math.hypot(astern[0] - tankSight.eyes[0][0], astern[1] - tankSight.eyes[0][1]),
    JSON.stringify(tankSight),
  );
  const aheadCss = await lab(page, ([x, y]) => window.__lab.projectToCss(x, y, 1), ahead);
  const abeam = lobePoint(Math.PI / 2, 1.5);
  const abeamCss = await lab(page, ([x, y]) => window.__lab.projectToCss(x, y, 1), abeam);
  const lobes = decode(await page.screenshot());
  await writeFile(ctx.evidencePath("frame-sight-lobes-1280x800.png"), await page.screenshot());
  await lab(page, () => window.__lab.route.setLobes(false));
  await page.evaluate(() => window.__lab.frame());
  const bare = decode(await page.screenshot());
  await writeFile(ctx.evidencePath("frame-lobes-off-1280x800.png"), await page.screenshot());
  await lab(page, () => window.__lab.route.setLobes(true));
  await page.evaluate(() => window.__lab.frame());
  const dLobe = (p) => Math.abs(luminance(pixel(lobes, ...p)) - luminance(pixel(bare, ...p)));
  ctx.check(
    "the lobe outline is drawn at the tank's front and side reach",
    dLobe(aheadCss) > 20 && dLobe(abeamCss) > 20,
    JSON.stringify({ ahead: dLobe(aheadCss), abeam: dLobe(abeamCss) }),
  );
  // The whole tank lobe framed on its own, for review.
  const opening = await lab(page, () => window.__lab.camera());
  await lab(page, (c) => window.__lab.setCamera(c), {
    ...opening,
    target: [tankSight.eyes[0][0] + 60, tankSight.eyes[0][1] + 60, 0],
    distance: 700,
  });
  await page.evaluate(() => window.__lab.frame());
  await writeFile(ctx.evidencePath("frame-tank-lobe-1280x800.png"), await page.screenshot());
  await lab(page, (c) => window.__lab.setCamera(c), opening);
  await page.evaluate(() => window.__lab.frame());
  const tankCss = await lab(
    page,
    ([x, y]) => window.__lab.projectToCss(x, y, 0),
    tankSight.eyes[0],
  );
  await writeCrop(
    lobes,
    ctx.evidencePath("crop-tank-lobe-2x.png"),
    tankCss[0],
    tankCss[1],
    260,
    180,
    2,
  );

  const edge = await lab(page, () => window.__lab.projectToCss(420, 100, 0));
  await writeCrop(fogged, ctx.evidencePath("crop-forest-edge-3x.png"), edge[0], edge[1], 90, 60, 3);
  const hill = await lab(page, () => window.__lab.projectToCss(700, 480, 10));
  await writeCrop(fogged, ctx.evidencePath("crop-hillside-2x.png"), hill[0], hill[1], 140, 90, 2);

  // Drive the scripted tour until red's tank is behind the ridge.
  let behind = null;
  for (let i = 0; i < 320 && !behind; i++) {
    await advance(page, 15);
    const s = await lab(page, () => window.__lab.route.tick());
    if (s > 30) {
      await lab(page, () => window.__lab.route.setSide("red"));
      await advance(page, 1);
      const tank = await lab(page, () =>
        window.__lab.route.observation().own.find((u) => u.kind === "tank"),
      );
      await lab(page, () => window.__lab.route.setSide("blue"));
      await advance(page, 1);
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
  await advance(page, 1);
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
