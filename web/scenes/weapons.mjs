// Slice 08: independent mounts, action reasons, the acquisition grace and
// every attack order, driven through the real command path.
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { lab, obs, advance } from "./_lab.mjs";

const own = (o, id) => o.own.find((u) => u.id === id);

async function panelCrop(ctx, page, name) {
  await page.evaluate(() => window.__lab.frame());
  const shot = await page.screenshot();
  const box = await page.getByTestId("action-panel").boundingBox();
  await writeCrop(
    decode(shot),
    ctx.evidencePath(name),
    box.x + box.width / 2,
    box.y + box.height / 2,
    box.width / 2 + 4,
    box.height / 2 + 4,
    2,
  );
  return shot;
}

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 20000 });
  await lab(page, () => window.__lab.route.pause());
  await lab(page, () => window.__lab.route.select([0, 1]));

  // The tank's cannon takes the identified tank with AP.
  await advance(page, 75);
  let o = await obs(page);
  const cannon = own(o, 0).mounts[0];
  const redTank = o.identified.find((e) => e.kind === "tank");
  ctx.check(
    "the cannon locks the identified tank with armour-piercing rounds",
    !!redTank &&
      cannon.target?.kind === "identified" &&
      cannon.target.id === redTank.id &&
      (cannon.loaded === 0 || cannon.loaded === null),
    JSON.stringify(cannon),
  );
  const engaging = await panelCrop(ctx, page, "crop-action-panel-engaging-2x.png");
  await writeFile(ctx.evidencePath("frame-engaging-1280x800.png"), engaging);

  // The red tank passes the wall: a brief loss keeps the same lock and aim.
  let grace = null;
  let reacquired = null;
  let tracer = null;
  for (let i = 0; i < 400 && !reacquired; i++) {
    await advance(page, 1);
    o = await obs(page);
    const c = own(o, 0).mounts[0];
    // Keep a tracer still near the shooter, so the frame shows it.
    const near = o.projectiles.find((p) => Math.hypot(p.from[0] - 260, p.from[1] - 230) < 120);
    if (!tracer && near) {
      tracer = near;
      await page.evaluate(() => window.__lab.frame());
      await writeFile(ctx.evidencePath("frame-tracer-1280x800.png"), await page.screenshot());
    }
    if (!grace && c.reason === "tracking_last_sighting") {
      grace = { tick: o.tick, target: c.target, aim: c.aim };
      const shot = await panelCrop(ctx, page, "crop-action-panel-grace-2x.png");
      await writeFile(ctx.evidencePath("frame-grace-1280x800.png"), shot);
    } else if (grace && c.target?.kind === "identified" && c.reason !== "tracking_last_sighting") {
      reacquired = { tick: o.tick, target: c.target };
    }
  }
  ctx.check(
    "losing sight for under 1.5 s keeps the acquisition (V12)",
    !!grace &&
      !!reacquired &&
      reacquired.target.id === grace.target.id &&
      reacquired.tick - grace.tick <= 45,
    JSON.stringify({ grace, reacquired }),
  );
  ctx.check(
    "visible rounds are published as flight segments",
    !!tracer && [...tracer.from, ...tracer.to].every(Number.isFinite),
    JSON.stringify(tracer),
  );

  // Until then the rifles' only choice is the tank they cannot hurt (the
  // default-gun fallback). From tick 420 a hidden squad fires, and its firing
  // area outranks that fallback for these general-purpose weapons.
  await advance(page, Math.max(0, 470 - o.tick));
  o = await obs(page);
  const rifles = own(o, 1).mounts;
  ctx.check(
    "the rifle squad prefers a firing area to a tank it cannot hurt",
    rifles.some((m) => m.target?.kind === "contact") &&
      !rifles.some((m) => m.target?.kind === "identified"),
    JSON.stringify(rifles.map((m) => [m.reason, m.target])),
  );
  // Every attack order through the one command path.
  const demo = async (ids, name) => {
    await lab(page, (s) => window.__lab.route.select(s), ids);
    await page.waitForFunction((s) => window.__lab.route.selected().join() === s.join(), ids);
    const ack = await lab(page, (n) => window.__lab.route.demo(n), name);
    await advance(page, 2);
    return ack;
  };
  const ground = await demo([1], "Attack ground");
  await advance(page, 10);
  o = await obs(page);
  ctx.check(
    "an attack on ground aims the squad's weapons at that point",
    ground?.error === null && own(o, 1).mounts.some((m) => m.target?.kind === "ground"),
    JSON.stringify({ ack: ground, mounts: own(o, 1).mounts.map((m) => m.target) }),
  );
  const hold = await demo([1], "Return fire only");
  o = await obs(page);
  ctx.check(
    "the engagement policy switches per unit",
    hold?.error === null &&
      own(o, 1).engagement === "return_fire_only" &&
      own(o, 0).engagement === "fire_at_will",
    JSON.stringify({ ack: hold, blue: o.own.map((u) => u.engagement) }),
  );
  const move = await demo([0], "Attack-move east");
  await advance(page, 30);
  o = await obs(page);
  const tank = own(o, 0);
  // Halting while engaging is pinned by the Rust weapon tests.
  ctx.check(
    "attack-move is accepted with its goal",
    move?.error === null && tank.goal?.join() === "600,230",
    JSON.stringify({ ack: move, goal: tank.goal, reasons: tank.mounts.map((m) => m.reason) }),
  );
  const final = await panelCrop(ctx, page, "crop-action-panel-orders-2x.png");
  await writeFile(ctx.evidencePath("frame-orders-1280x800.png"), final);
}
