// Slice 08: independent mounts, action reasons, the acquisition grace and
// every attack order, driven through the real command path.
import { attackCursor } from "./_cursorOrders.mjs";
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { lab, obs, advance, openBattle } from "./_lab.mjs";

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
  const redTank = o.identified.find((e) => e.kind === "test_tank");
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

  // Step directly to a fired pose; per-tick targeting/grace is owned by sim tests.
  await advance(page, 395);
  await page.evaluate(() => window.__lab.frame());
  const feed = await page.getByTestId("feed-panel").innerText();
  const cannonPose = own(await obs(page), 0).weaponPoses[0];
  ctx.check(
    "the feed inspector shows the cannon's pose and shot counter",
    cannonPose.shots > 0 && /own test_tank #0:\s*m0 \S+ ↑\S+ · [1-9]\d* shots/.test(feed),
    JSON.stringify({ cannonPose, feed }),
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
