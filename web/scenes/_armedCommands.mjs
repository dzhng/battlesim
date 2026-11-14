// Armed commands at the real input seam: a key arms one, its badge holds
// while the pointer sweeps, and a left-click sends it.
import { lab, obs, openBattle } from "./_lab.mjs";
import { expects, project, select } from "./_cursorOrders.mjs";

const base = (ctx) => new URL(ctx.url).origin;
/** An armed command is a left-click away: its badge holds while the pointer
 *  sweeps open ground (every new goal is placed in the background), and a
 *  left-click sends it, keeping the selection. */
export async function armedCursor(ctx) {
  const page = await openBattle(ctx, { url: `${base(ctx)}/labs/movement`, tick: 40 });
  const tank = (await obs(page)).own.find((u) => u.kind === "test_tank");
  await select(page, [tank.id]);
  await lab(
    page,
    (at) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        target: [at[0], at[1], 0],
        distance: 140,
        pitch: 1.1,
      }),
    tank.position,
  );
  const commands = [
    ["r", "reverse_move", (o) => o.kind === "move" && o.direction === "reverse"],
    ["c", "attack_move", (o) => o.kind === "attack_move"],
  ];
  for (const [key, badge, sent] of commands) {
    // Open ground inside the map, clear of the tank's card (over a unit or
    // its card the arrow is plain), stepping a metre at a time.
    const ground = (k) => [tank.position[0] - 15, tank.position[1] - 30 - k, 0];
    const from = await project(page, ground(0));
    await page.mouse.move(...from);
    await page.keyboard.press(key);
    await expects(page, badge);
    // Every frame's cursor while the pointer sweeps on.
    await page.evaluate(() => {
      const seen = (window.__armedSeen = []);
      const sample = () => {
        seen.push(document.documentElement.dataset.cursor);
        if (window.__armedSeen === seen) requestAnimationFrame(sample);
      };
      requestAnimationFrame(sample);
    });
    for (let k = 1; k <= 24; k++) {
      await page.mouse.move(...(await project(page, ground(k))));
      await page.evaluate(() => new Promise((r) => requestAnimationFrame(r)));
    }
    const seen = await page.evaluate(() => {
      const seen = window.__armedSeen;
      window.__armedSeen = null;
      return seen;
    });
    ctx.check(
      `armed ${badge}: its badge holds on every frame of a sweep`,
      seen.length > 20 && seen.every((a) => a === badge),
      JSON.stringify([...new Set(seen)]),
    );
    // The log keeps only the latest few: wait on the newest's sequence.
    const before = await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);
    await page.mouse.click(...(await project(page, ground(24))));
    await page.waitForFunction((n) => (window.__lab.route.acks()[0]?.seq ?? 0) > n, before);
    const { order } = (await lab(page, () => window.__lab.route.acks()))[0];
    ctx.check(
      `armed ${badge}: a left-click sends it and keeps the selection`,
      sent(order) &&
        order.units.join() === String(tank.id) &&
        JSON.stringify(await lab(page, () => window.__lab.route.selected())) ===
          JSON.stringify([tank.id]),
      JSON.stringify(order),
    );
  }
  await page.close();
}
