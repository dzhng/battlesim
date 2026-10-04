// Slice 08: independent mounts, action reasons, the acquisition grace and
// every attack order, driven through the real command path.
import { attackCursor } from "./_cursorOrders.mjs";
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { lab, obs, advance, openBattle } from "./_lab.mjs";
import { game } from "./_units.mjs";

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

async function feedReachable(ctx, page, name) {
  const original = page.viewportSize();
  await page.evaluate(() => window.scrollTo(0, 0));
  for (const viewport of [original, { width: 800, height: 600 }]) {
    await page.setViewportSize(viewport);
    await page.evaluate(() => window.__lab.frame());
    const receipt = await page.getByTestId("weapons-panel").evaluate((panel) => {
      const feed = panel.querySelector('[data-testid="feed-panel"]');
      const bounds = (el) => {
        const r = el.getBoundingClientRect();
        return { top: r.top, right: r.right, bottom: r.bottom, left: r.left };
      };
      const panelBounds = bounds(panel);
      const rows = [...feed.querySelectorAll("[data-feed]")].map((row) => {
        row.scrollIntoView({ block: "center" });
        const r = bounds(row),
          p = bounds(panel),
          f = bounds(feed);
        return {
          row: row.dataset.feed,
          bounds: r,
          panel: p,
          feed: f,
          reachable:
            r.top >= Math.max(0, p.top, f.top) &&
            r.bottom <= Math.min(innerHeight, p.bottom, f.bottom) &&
            r.left >= Math.max(0, p.left, f.left) &&
            r.right <= Math.min(innerWidth, p.right, f.right),
        };
      });
      return { panel: panelBounds, rows, viewport: [innerWidth, innerHeight] };
    });
    const id = `${name}-${viewport.width}x${viewport.height}`;
    ctx.check(
      `${id}: every animation feed entry is reachable inside the viewport`,
      receipt.panel.top >= 0 &&
        receipt.panel.bottom <= viewport.height &&
        receipt.panel.left >= 0 &&
        receipt.panel.right <= viewport.width &&
        receipt.rows.length > 0 &&
        receipt.rows.every((row) => row.reachable),
      JSON.stringify(receipt),
    );
    await ctx.writeEvidence(`${id}.json`, receipt);
    await writeFile(ctx.evidencePath(`frame-${id}-feed-end.png`), await page.screenshot());
    await page.getByTestId("weapons-panel").evaluate((panel) => {
      panel.scrollTop = 0;
      panel.querySelector('[data-testid="feed-panel"]').scrollTop = 0;
    });
  }
  await page.setViewportSize(original);
  await page.evaluate(() => {
    window.scrollTo(0, 0);
    return window.__lab.frame();
  });
}

export async function run(ctx) {
  await attackCursor(ctx);
  const page = await openBattle(ctx);
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
  await feedReachable(ctx, page, "engaging");

  // The red tank passes the wall: a brief loss keeps the same lock and aim.
  let grace = null;
  let reacquired = null;
  let tracer = null;
  // While no enemy is identified, the squad's grenade may take an area (the
  // hidden tank's firing report).
  let areaWhileUnseen = null;
  const visibleTankChoices = [];
  for (let i = 0; i < 400 && (!reacquired || !areaWhileUnseen); i++) {
    await advance(page, 1);
    o = await obs(page);
    const c = own(o, 0).mounts[0];
    const grenade = own(o, 1).mounts[1];
    const squad = own(o, 1);
    const visible = o.identified.find((e) => e.kind === "tank");
    if (
      visible &&
      o.contacts.some((c) => c.source === "firing") &&
      Math.hypot(
        visible.position[0] - squad.position[0],
        visible.position[1] - squad.position[1],
      ) <= game.weapons.rifle.range_m
    ) {
      visibleTankChoices.push({
        tick: o.tick,
        target: visible.id,
        distance: Math.hypot(
          visible.position[0] - squad.position[0],
          visible.position[1] - squad.position[1],
        ),
        mounts: squad.mounts,
      });
    }
    const area =
      grenade.target?.kind === "contact" &&
      o.contacts.find((c) => c.id === grenade.target.id && c.source === "firing");
    if (!areaWhileUnseen && o.identified.length === 0 && area) {
      areaWhileUnseen = {
        tick: o.tick,
        target: grenade.target,
        area,
        distance:
          area &&
          Math.hypot(area.center[0] - squad.position[0], area.center[1] - squad.position[1]),
        publication: o,
      };
    }
    // Keep a tracer still near the shooter, so the frame shows it.
    const near = o.projectiles.find(
      (p) => Math.hypot(p.path[0][0] - 260, p.path[0][1] - 230) < 120,
    );
    if (!tracer && near) {
      tracer = near;
      await page.evaluate(() => window.__lab.frame());
      await writeFile(ctx.evidencePath("frame-tracer-1280x800.png"), await page.screenshot());
      await feedReachable(ctx, page, "tracer");
    }
    if (!grace && c.reason === "tracking_last_sighting") {
      grace = { tick: o.tick, target: c.target, aim: c.aim };
      const shot = await panelCrop(ctx, page, "crop-action-panel-grace-2x.png");
      await writeFile(ctx.evidencePath("frame-grace-1280x800.png"), shot);
    } else if (
      grace &&
      !reacquired &&
      c.target?.kind === "identified" &&
      c.reason !== "tracking_last_sighting"
    ) {
      reacquired = { tick: o.tick, target: c.target };
    }
  }
  ctx.check(
    "losing sight within the authored grace keeps the acquisition",
    !!grace &&
      !!reacquired &&
      reacquired.target.id === grace.target.id &&
      reacquired.tick - grace.tick <= Math.round(game.sensors.acquisition_grace_s * game.tick_hz),
    JSON.stringify({ grace, reacquired }),
  );
  ctx.check(
    "visible rounds are published as flight segments",
    !!tracer && tracer.path.length >= 2 && tracer.path.flat().every(Number.isFinite),
    JSON.stringify(tracer),
  );
  // The cannon has fired: the decoded-frame inspector lists its pose and shots.
  await page.evaluate(() => window.__lab.frame());
  const feed = await page.getByTestId("feed-panel").innerText();
  const cannonPose = own(await obs(page), 0).weaponPoses[0];
  ctx.check(
    "the feed inspector shows the cannon's pose and shot counter",
    cannonPose.shots > 0 && /own tank #0:\s*m0 \S+ ↑\S+ · [1-9]\d* shots/.test(feed),
    JSON.stringify({ cannonPose, feed }),
  );
  await writeCrop(
    decode(await page.screenshot()),
    ctx.evidencePath("crop-feed-panel-2x.png"),
    ...(await page
      .getByTestId("feed-panel")
      .boundingBox()
      .then((b) => [b.x + b.width / 2, b.y + b.height / 2, b.width / 2 + 4, b.height / 2 + 4, 2])),
  );

  ctx.check(
    "with no enemy identified, the squad's grenade takes an area",
    !!areaWhileUnseen &&
      areaWhileUnseen.area?.source === "firing" &&
      areaWhileUnseen.distance >= game.weapons.grenade.min_range_m &&
      areaWhileUnseen.distance <= game.weapons.grenade.range_m,
    JSON.stringify(
      areaWhileUnseen && {
        tick: areaWhileUnseen.tick,
        target: areaWhileUnseen.target,
        distance: areaWhileUnseen.distance,
      },
    ),
  );
  await ctx.writeEvidence("area-selection.json", areaWhileUnseen);
  // Judge actual coexistence of an identified tank and a firing report.
  // Follow visibility rather than a fixed tick: combat may kill the tank sooner.
  ctx.check(
    "a visible tank in reach takes priority over firing areas",
    visibleTankChoices.length > 0 &&
      visibleTankChoices.every(
        (s) =>
          s.distance <= game.weapons.grenade.range_m &&
          s.mounts.every((m) => m.target?.kind !== "contact"),
      ) &&
      visibleTankChoices.some(
        (s) => s.mounts[0].target?.kind === "identified" && s.mounts[0].target.id === s.target,
      ),
    JSON.stringify(visibleTankChoices),
  );
  await advance(page, Math.max(0, 470 - o.tick));
  o = await obs(page);
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
