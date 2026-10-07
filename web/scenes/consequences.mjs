// Slice 09: impacts, suppression and lasting remains, through real orders.
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { lab, obs, advance, until, snapshot, openBattle, aim, presented } from "./_lab.mjs";

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
  const page = await openBattle(ctx);

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
    !!o && !o.identified.some((e) => e.kind === "test_tank"),
    JSON.stringify(o?.identified),
  );

  // The truck is sent through the passage: its route goes round the wreck.
  await demo(page, "Truck east through the gap");
  // The route is planned over a few ticks while the truck holds.
  let truck;
  for (let i = 0; i < 100; i++) {
    await advance(page, 4);
    o = await obs(page);
    truck = o.own.find((u) => u.kind === "test_supply");
    if (truck.state !== "planning") break;
  }
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
  // The friendly squad's worst published suppression tier.
  const TIERS = ["none", "suppressed", "pinned"];
  let worst = "none";
  let halo = null;
  for (let t = 0; t < 600; t += 15) {
    await advance(page, 15);
    o = await obs(page);
    const squad = o.own.find((u) => u.kind === "test_rifle");
    if (squad && TIERS.indexOf(squad.suppression) > TIERS.indexOf(worst)) {
      worst = squad.suppression;
      if (!halo) halo = await frame(ctx, page, "open");
    }
  }
  ctx.check(
    "HE kills in the open and blue sees the fallen",
    !!o &&
      o.corpses.some((c) => !c.own && Math.hypot(c.position[0] - 360, c.position[1] - 148) < 15),
    JSON.stringify(o?.corpses),
  );
  ctx.check("the friendly squad nearby is suppressed too", worst !== "none", `worst ${worst}`);
  const openLosses = o.corpses.filter((c) => !c.own).length;
  png = halo ?? (await frame(ctx, page, "open"));
  await crop(ctx, page, png, "crop-open-squad-4x.png", [358, 158], [40, 32], 4);

  // Then the same 20 s into the forest squad; the fallen in the open stay put.
  const before = JSON.stringify(o.corpses);
  await demo(page, "HE on the forest squad");
  // The first tree blue sees felled: framed close as it falls, then lying
  // by its stump, pressed under a man's height (it gives no cover).
  let spent = 0;
  o = await until(
    page,
    (f) => f.fallenBodies.length > 0,
    600,
    1,
    () => spent++,
  );
  const tree = o?.fallenBodies[0];
  ctx.check("a burst in the wood fells a tree blue sees", !!tree, JSON.stringify(o?.fallenBodies));
  if (tree) {
    // From above (a wood's crowns hide anything lower), side-on to the fall.
    const yaw = Math.atan2(tree.toward[1], tree.toward[0]) + Math.PI / 2;
    const at = [tree.at[0] + tree.toward[0] * 5, tree.at[1] + tree.toward[1] * 5];
    await aim(page, at, { distance: 30, pitch: 1.35, yaw });
    await advance(page, 36);
    await presented(page);
    const falling = await lab(page, () => window.__lab.stats().scenery.felled);
    await snapshot(ctx, page, "felled-falling.png");
    await advance(page, 90);
    await presented(page);
    const lying = await lab(page, () => window.__lab.stats().scenery.felled);
    await snapshot(ctx, page, "felled-lying.png");
    await aim(page, at, { distance: 55, pitch: 0.9, yaw });
    await presented(page);
    await snapshot(ctx, page, "felled-lying-game.png");
    spent += 126;
    ctx.check(
      "a felled tree falls on the clock, then lies by its stump",
      falling.trees > 0 && falling.falling && lying.trees > 0 && !lying.falling && lying.stumps > 0,
      JSON.stringify({ tree, falling, lying }),
    );
  }
  await look(page, [330, 250], 330);
  await advance(page, Math.max(0, 600 - spent));
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
  await page.close();
  await ownDeath(ctx);
}

/** A retained living soldier must play death before entering the static layer.
 *  A real ground attack supplies the event independently of assault balance. */
export async function ownDeath(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 } });
  const squad = (await obs(page)).own.find((u) => u.kind === "test_rifle");
  await aim(page, squad.position, { distance: 35, pitch: 0.6 });
  await presented(page);
  await snapshot(ctx, page, "own-death-alive.png");
  const aliveModels = await lab(page, () => window.__lab.stats().models);
  ctx.check(
    "the own squad is rendered alive before damage",
    squad.memberIds.length > 0 && aliveModels.skinned > 0,
    JSON.stringify({ ids: squad.memberIds, skinned: aliveModels.skinned }),
  );
  const ack = await lab(page, () =>
    window.__lab.route.command({
      kind: "attack",
      units: [0],
      target: { kind: "ground", point: [356, 166, 0] },
    }),
  );
  const own = (o) => o.corpses.find((c) => c.own && squad.memberIds.includes(c.soldier));
  const fallen = await until(page, (o) => !!own(o), 600, 1);
  ctx.check(
    "the ground attack produces a real casualty from the rendered own squad",
    ack.error === null && !!fallen,
    JSON.stringify({ ack, ids: squad.memberIds, tick: fallen?.tick }),
  );
  if (!fallen) {
    await page.close();
    return;
  }
  const soldier = own(fallen);
  await presented(page);
  const immediate = await lab(page, () => window.__lab.route.lying());
  await snapshot(ctx, page, "own-death-playing.png");
  ctx.check(
    "a soldier just seen falling plays death before becoming static",
    !immediate.includes(soldier.soldier),
    JSON.stringify({ soldier, immediate }),
  );
  await lab(page, () => window.__lab.route.command({ kind: "stop", units: [0] }));
  let lying, models;
  for (let t = 0; t < 150; t += 30) {
    await advance(page, 30);
    await presented(page);
    lying = await lab(page, () => window.__lab.route.lying());
    models = await lab(page, () => window.__lab.stats().models);
    if (lying.includes(soldier.soldier)) break;
  }
  await snapshot(ctx, page, "own-death-static.png");
  ctx.check(
    "the same fallen soldier finishes death and is drawn as a static corpse",
    lying.includes(soldier.soldier) && models.corpses > 0,
    JSON.stringify({ soldier: soldier.soldier, lying, models }),
  );
  await advance(page, 30);
  await presented(page);
  ctx.check(
    "the finished corpse remains static",
    (await lab(page, () => window.__lab.route.lying())).includes(soldier.soldier),
  );
  await ctx.writeEvidence("own-death.json", {
    ids: squad.memberIds,
    ack,
    soldier,
    immediate,
    lying,
    models,
  });
  await page.close();
}
