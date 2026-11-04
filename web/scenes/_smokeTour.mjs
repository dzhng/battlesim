import { installWorkerStartFence } from "./_workerStart.mjs";
// Smoke and dust over the production street battle, paused and running.
import { lab, obs, advance, until, snapshot, openBattle, aim } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";

/** Pixels whose colour differs by more than 16 levels inside a box. */
function changedIn(a, b, [x0, y0, x1, y1]) {
  let n = 0;
  for (let y = Math.max(0, y0); y < Math.min(a.height, y1); y += 2)
    for (let x = Math.max(0, x0); x < Math.min(a.width, x1); x += 2) {
      const p = pixel(a, x, y);
      const q = pixel(b, x, y);
      if (Math.abs(p[0] - q[0]) + Math.abs(p[1] - q[1]) + Math.abs(p[2] - q[2]) > 16) n++;
    }
  return n;
}

/** The aftermath. The first wreck blue learns burns and smokes
 *  over it, paused and playing; a moving tank kicks dust behind it; pause
 *  holds the smoke and reset clears it. `SMOKE_GIF=1` also writes the
 *  frames of a burning wreck and of a tank's dust, tick by tick. */
export async function smokeTour(ctx) {
  const page = await openBattle(ctx, {
    viewport: { width: 1920, height: 1080 },
    tick: 30,
    initScript: installWorkerStartFence,
  });
  await lab(page, () => {
    const o = window.__lab.route.observation();
    window.__lab.route.command({
      kind: "attack_move",
      units: o.own.filter((u) => u.kind !== "test_tank").map((u) => u.id),
      gesture: 1,
      goal: [1000, 800],
    });
    // Tanks drive on (a plain move, still firing at will): an attack-move
    // halts them at the first target, and the wood's AT is one from the start.
    window.__lab.route.command({
      kind: "move",
      units: o.own.filter((u) => u.kind === "test_tank").map((u) => u.id),
      gesture: 2,
      goal: [1000, 800],
      route: "shortest",
    });
  });
  // A tank under way, framed from behind its shoulder, kicking up dust.
  const before = await obs(page);
  await advance(page, 90);
  const moving = await obs(page);
  const tank = moving.own.find((u) => {
    const was = before.own.find((b) => b.id === u.id);
    return (
      u.kind === "test_tank" &&
      was &&
      Math.hypot(u.position[0] - was.position[0], u.position[1] - was.position[1]) > 3
    );
  });
  if (tank) {
    await aim(page, tank.position, { distance: 32, pitch: 0.4, yaw: tank.yaw + Math.PI * 0.75 });
    if (process.env.SMOKE_GIF === "1")
      for (let k = 0; k < 90; k++) {
        await advance(page, 1);
        const t = (await obs(page)).own.find((u) => u.id === tank.id);
        await aim(page, t.position, { distance: 32, pitch: 0.4, yaw: tank.yaw + Math.PI * 0.75 });
        await snapshot(ctx, page, `gif-dust-${String(k).padStart(3, "0")}.png`);
      }
    const now = (await obs(page)).own.find((u) => u.id === tank.id);
    await aim(page, now.position, { distance: 32, pitch: 0.4, yaw: tank.yaw + Math.PI * 0.75 });
    const shotPng = decode(await snapshot(ctx, page, "smoke-dust-1920x1080.png"));
    await lab(page, () => window.__lab.suppressEffects(true));
    const bare = decode(await snapshot(ctx, page, "smoke-dust-bare-1920x1080.png"));
    await lab(page, () => window.__lab.suppressEffects(false));
    // Behind the tank: the side of it away from where it heads.
    const back = [
      now.position[0] - Math.cos(now.yaw) * 6,
      now.position[1] - Math.sin(now.yaw) * 6,
      now.position[2] + 1,
    ];
    const [bx, by] = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), back);
    const dust = changedIn(shotPng, bare, [bx - 90, by - 90, bx + 90, by + 90]);
    ctx.check(
      "a moving tank kicks up dust behind it",
      dust > 200,
      JSON.stringify({ tick: moving.tick, tank: tank.id, dustPixels: dust }),
    );
  } else ctx.check("a moving tank kicks up dust behind it", false, "no tank under way");

  const o = await until(
    page,
    (f) => f.knownProps.some((p) => p.kind.endsWith("_wreck")),
    30 * 150,
    15,
  );
  if (!o) {
    ctx.check("a known wreck burns and smokes", false, "no wreck by tick 4600");
    await page.close();
    return;
  }
  const wreck = o.knownProps.find((p) => p.kind.endsWith("_wreck"));
  const at = [wreck.center[0], wreck.center[1], wreck.baseZ];
  const view = { distance: 70, pitch: 0.5, yaw: -1.2 };
  const shots = {};
  for (const [age, step] of [
    [2, 60],
    [10, 240],
  ]) {
    await advance(page, step);
    await aim(page, at, view);
    shots[age] = decode(await snapshot(ctx, page, `smoke-wreck-${age}s-1920x1080.png`));
    await lab(page, () => window.__lab.setFrameView("world"));
    await snapshot(ctx, page, `smoke-wreck-${age}s-world-1920x1080.png`);
    await lab(page, () => window.__lab.setFrameView("final"));
  }
  if (process.env.SMOKE_GIF === "1")
    for (let k = 0; k < 90; k++) {
      await advance(page, 1);
      await snapshot(ctx, page, `gif-wreck-${String(k).padStart(3, "0")}.png`);
    }
  await lab(page, () => window.__lab.suppressEffects(true));
  const bare = decode(await snapshot(ctx, page, "smoke-wreck-bare-1920x1080.png"));
  await lab(page, () => window.__lab.suppressEffects(false));
  const [wx, wy] = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 8), at);
  const column = changedIn(shots[10], bare, [wx - 200, wy - 300, wx + 200, wy + 150]);
  const stats = await lab(page, () => window.__lab.route.effects());
  ctx.check(
    "a known wreck burns and smokes over it",
    column > 1500 && stats.sources > 0 && stats.dropped === 0,
    JSON.stringify({ tick: o.tick, wreck, columnPixels: column, ...stats }),
  );

  // Paused, the smoke holds still: the same frame twice.
  const held = decode(await snapshot(ctx, page, "smoke-paused-a-1920x1080.png"));
  await page.waitForTimeout(500);
  const again = decode(await snapshot(ctx, page, "smoke-paused-b-1920x1080.png"));
  const drift = changedIn(held, again, [wx - 200, wy - 300, wx + 200, wy + 150]);
  ctx.check("paused, the smoke holds still", drift === 0, JSON.stringify({ drift }));

  // Playing, it rises on.
  await lab(page, () => window.__lab.route.resume());
  await page.waitForTimeout(1500);
  await lab(page, () => window.__lab.route.pause());
  await aim(page, at, view);
  const playing = decode(await snapshot(ctx, page, "smoke-playing-1920x1080.png"));
  const rose = changedIn(held, playing, [wx - 200, wy - 300, wx + 200, wy + 150]);
  ctx.check("playing, the smoke moves on", rose > 200, JSON.stringify({ rose }));

  // Reset: a new battle, no smoke left from the old one.
  await page.evaluate(() => window.__pauseNextWorkerStart());
  await lab(page, () => window.__lab.route.reset());
  await page.waitForFunction(() => window.__lab?.ready && window.__lab.route?.tick() === 0);
  await advance(page, 4);
  await lab(page, () => window.__lab.frame());
  const cleared = await lab(page, () => ({
    effects: window.__lab.route.effects(),
    frame: window.__lab.stats().effects,
  }));
  ctx.check(
    "reset clears every effect",
    cleared.effects.sources === 0 && cleared.effects.live === 0,
    JSON.stringify(cleared),
  );
  await page.close();
}
