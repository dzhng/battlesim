// Slice 09: impacts, suppression and lasting remains, through real orders.
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { lab, obs, advance, until, snapshot } from "./_lab.mjs";

const demo = (page, name) => lab(page, (n) => window.__lab.route.demo(n), name);

/** Frame a shot on world point `target` from `distance` metres. */
const look = (page, target, distance) =>
  lab(page, (v) => window.__lab.setCamera({ ...window.__lab.camera(), ...v }), {
    target: [...target, 0],
    distance,
  });

async function frame(ctx, page, name) {
  const shot = await snapshot(ctx, page, `frame-${name}-1280x800.png`);
  return decode(shot);
}

async function crop(ctx, page, png, name, [x, y], half, scale = 2) {
  const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], 0), [x, y]);
  await writeCrop(png, ctx.evidencePath(name), at[0], at[1], half[0], half[1], scale);
}

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 20000 });
  await lab(page, () => window.__lab.route.pause());

  // The tank duel: red's tank stands in the gap, then dies and leaves a wreck.
  await advance(page, 5);
  await look(page, [430, 330], 260);
  await frame(ctx, page, "tank-alive");
  const attack = await demo(page, "Destroy the red tank");
  let o = await until(page, (f) => f.knownProps.some((p) => p.kind.endsWith("_wreck")), 2400, 30);
  const wreck = o?.knownProps.find((p) => p.kind.endsWith("_wreck"));
  ctx.check(
    "a destroyed tank leaves a wreck where it stood",
    attack?.error === null &&
      !!wreck &&
      Math.hypot(wreck.center[0] - 520, wreck.center[1] - 390) < 2,
    JSON.stringify({ attack, wreck }),
  );
  ctx.check(
    "the destroyed tank is no longer identified",
    !!o && !o.identified.some((e) => e.kind === "tank"),
    JSON.stringify(o?.identified),
  );

  // The truck is sent through the passage: its route goes round the wreck.
  await demo(page, "Truck east through the gap");
  await advance(page, 4);
  o = await obs(page);
  const truck = o.own.find((u) => u.kind === "supply");
  const crosses = (() => {
    let from = truck.position;
    for (const p of truck.route) {
      for (let k = 0; k <= 20; k++) {
        const x = from[0] + ((p[0] - from[0]) * k) / 20,
          y = from[1] + ((p[1] - from[1]) * k) / 20;
        if (
          Math.abs(x - wreck.center[0]) < wreck.half[1] + 1 &&
          Math.abs(y - wreck.center[1]) < wreck.half[0] + 1
        )
          return true;
      }
      from = p;
    }
    return false;
  })();
  ctx.check(
    "a late wreck reroutes the side that saw it",
    truck.route.length > 0 && !crosses,
    JSON.stringify({ route: truck.route }),
  );
  // Framed on the gap, the wall's north end and the detour round it.
  await look(page, [470, 330], 230);
  let png = await frame(ctx, page, "wreck");
  await crop(ctx, page, png, "crop-wreck-passage-3x.png", [520, 380], [70, 80], 3);

  // HE into the open squad for 20 s: red falls, blue's squad nearby shares the blast.
  await look(page, [330, 250], 330);
  await demo(page, "HE on the open squad");
  let pinned = 0;
  let halo = null;
  for (let t = 0; t < 600; t += 15) {
    await advance(page, 15);
    o = await obs(page);
    const squad = o.own.find((u) => u.kind === "rifle");
    if ((squad?.suppression ?? 0) > pinned) {
      pinned = squad.suppression;
      if (!halo && pinned > 0.2) halo = await frame(ctx, page, "open");
    }
  }
  ctx.check(
    "HE kills in the open and blue sees the fallen",
    !!o &&
      o.corpses.some((c) => !c.own && Math.hypot(c.position[0] - 360, c.position[1] - 148) < 15),
    JSON.stringify(o?.corpses),
  );
  ctx.check("the friendly squad nearby is suppressed too", pinned > 0, `peak ${pinned}`);
  const openLosses = o.corpses.filter((c) => !c.own).length;
  png = halo ?? (await frame(ctx, page, "open"));
  await crop(ctx, page, png, "crop-open-squad-4x.png", [358, 158], [40, 32], 4);

  // Then the same 20 s into the forest squad; the fallen in the open stay put.
  const before = JSON.stringify(o.corpses);
  await demo(page, "HE on the forest squad");
  await advance(page, 600);
  o = await obs(page);
  // One seed is a sample, not a verdict: the Rust paired trial owns the rate.
  await writeFile(
    ctx.evidencePath("casualties.txt"),
    `open: ${openLosses} red fallen in 20 s (the first HE needs a 6 s reload from AP)\n` +
      `forest: ${o.corpses.filter((c) => !c.own).length - openLosses} red fallen in 20 s\n`,
  );
  ctx.check(
    "the fallen stay where they fell",
    JSON.parse(before).every((c) =>
      o.corpses.some((d) => d.position.join() === c.position.join() && d.own === c.own),
    ),
    `${JSON.parse(before).length} earlier, ${o.corpses.length} now`,
  );
  png = await frame(ctx, page, "pair");
  await crop(ctx, page, png, "crop-open-squad-after-4x.png", [358, 150], [40, 32], 4);
  await crop(ctx, page, png, "crop-forest-squad-4x.png", [375, 275], [40, 32], 4);
}
