// The cursor keeps up with a moving pointer: what a click would order shows
// on every frame of a sweep, never only once the pointer rests.
import { lab, obs, openBattle } from "./_lab.mjs";
import { expects, project, select } from "./_cursorOrders.mjs";

const base = (ctx) => new URL(ctx.url).origin;

/** Move the pointer through `points` (world positions), a frame apiece, and
 *  return each frame's cursor action and pointer intent. */
async function sweep(page, points) {
  await page.evaluate(() => {
    const seen = (window.__sweepSeen = []);
    const sample = () => {
      seen.push([document.documentElement.dataset.cursor, window.__lab.route.pointerIntent().kind]);
      if (window.__sweepSeen === seen) requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
  });
  for (const point of points) {
    const at = await project(page, point);
    if (!at || at.some((v) => !Number.isFinite(v))) continue;
    await page.mouse.move(...at);
    await page.evaluate(() => new Promise((r) => requestAnimationFrame(r)));
  }
  return page.evaluate(() => {
    const seen = window.__sweepSeen;
    window.__sweepSeen = null;
    return seen;
  });
}

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
  // Open ground inside the map, clear of the tank's card (over a unit or its
  // card the arrow is plain), stepping a metre at a time.
  const ground = (k) => [tank.position[0] - 15, tank.position[1] - 30 - k, 0];
  const commands = [
    ["r", "reverse_move", (o) => o.kind === "move" && o.direction === "reverse"],
    ["c", "attack_move", (o) => o.kind === "attack_move"],
  ];
  for (const [key, badge, sent] of commands) {
    await page.mouse.move(...(await project(page, ground(0))));
    await page.keyboard.press(key);
    await expects(page, badge);
    const seen = await sweep(
      page,
      Array.from({ length: 24 }, (_, k) => ground(k + 1)),
    );
    ctx.check(
      `armed ${badge}: its badge holds on every frame of a sweep`,
      seen.length > 20 && seen.every(([action]) => action === badge),
      JSON.stringify([...new Set(seen.map(([action]) => action))]),
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

/** A squad's pointer sweeping from open ground onto a building it can enter
 *  shows the garrison badge on the frame it arrives, before the simulation
 *  has answered, while the pointer keeps moving, and keeps it across the
 *  roof. */
export async function buildingCursor(ctx) {
  const page = await openBattle(ctx, { url: `${base(ctx)}/labs/garrison`, tick: 40 });
  await lab(page, () =>
    window.__lab.setCamera({
      ...window.__lab.camera(),
      target: [346, 252, 0],
      distance: 130,
      pitch: 0.95,
      yaw: -0.75,
    }),
  );
  await select(page, [0]);
  // The battle runs, as a player's does: the simulation answers the cursor
  // between its own ticks.
  await lab(page, () => window.__lab.route.resume());
  // Up the street to the house's middle, then across its roof.
  const [cx, cy, roof] = [360, 250, 8];
  const points = [
    ...Array.from({ length: 30 }, (_, k) => [cx - 40 + k, cy, 0]),
    ...Array.from({ length: 16 }, (_, k) => [cx - 8 + k, cy, roof]),
  ];
  const seen = await sweep(page, points);
  const arrived = seen.findIndex(([, intent]) => intent === "occupy_building");
  const shown = seen.findIndex(([action], i) => i >= arrived && action === "garrison");
  ctx.check(
    "a moving pointer arriving on a building shows the garrison badge on arrival",
    arrived >= 0 && shown === arrived,
    JSON.stringify({
      arrived,
      shown,
      frames: seen.length,
      tail: seen.slice(arrived, arrived + 12),
    }),
  );
  ctx.check(
    "the garrison badge holds while the pointer crosses the building",
    shown >= 0 &&
      seen
        .slice(shown)
        .every(([action, intent]) => intent !== "occupy_building" || action === "garrison"),
    JSON.stringify([...new Set(seen.slice(Math.max(shown, 0)).map(([a]) => a))]),
  );
  await page.close();
}

/** Each frame from now until `act` settles plus `frames` more: the cursor
 *  action, the outlined garrison target and the count of held move marks. */
async function watch(page, act, frames) {
  await page.evaluate(() => {
    const seen = (window.__restSeen = []);
    const sample = () => {
      seen.push([
        document.documentElement.dataset.cursor,
        window.__lab.route.garrisonTarget(),
        window.__lab.route.movePreview().length,
      ]);
      if (window.__restSeen === seen) requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
  });
  await act();
  for (let k = 0; k < frames; k++)
    await page.evaluate(() => new Promise((r) => requestAnimationFrame(r)));
  return page.evaluate(() => {
    const seen = window.__restSeen;
    window.__restSeen = null;
    return seen;
  });
}

/** A pointer at rest keeps what it offers while the rest of the side's
 *  orders change around it: the badge and outline over a building a squad
 *  can enter, and the ghosts of a held right-drag over open ground. */
export async function restingCursor(ctx) {
  const page = await openBattle(ctx, { url: `${base(ctx)}/labs/garrison`, tick: 40 });
  await lab(page, () =>
    window.__lab.setCamera({
      ...window.__lab.camera(),
      target: [346, 252, 0],
      distance: 130,
      pitch: 0.95,
      yaw: -0.75,
    }),
  );
  // Squad 0 points; squad 1, not selected, takes new orders meanwhile.
  await select(page, [0]);
  await lab(page, () => window.__lab.route.resume());
  let gesture = 9100;
  const orderOther = async () => {
    for (const goal of [
      [320, 270],
      [322, 274],
      [318, 268],
    ]) {
      await lab(page, (order) => window.__lab.route.command(order), {
        kind: "move",
        units: [1],
        gesture: gesture++,
        goal,
        route: "shortest",
      });
      for (let k = 0; k < 6; k++)
        await page.evaluate(() => new Promise((r) => requestAnimationFrame(r)));
    }
  };

  await page.mouse.move(...(await project(page, [360, 250, 8])));
  await expects(page, "garrison");
  await page.waitForFunction(() => window.__lab.route.garrisonTarget() !== null);
  const resting = await watch(page, orderOther, 30);
  ctx.check(
    "a resting pointer's garrison badge holds while other units take orders",
    resting.length > 20 && resting.every(([action]) => action === "garrison"),
    JSON.stringify(resting.map(([action]) => action)),
  );
  ctx.check(
    "the building it offers stays outlined while other units take orders",
    resting.every(([, outlined]) => outlined !== null),
    JSON.stringify(resting.map(([, outlined]) => outlined)),
  );

  // A right-drag on open ground west of the house, held while turning.
  const from = await project(page, [330, 250, 0]);
  const to = await project(page, [330, 240, 0]);
  await page.mouse.move(...from);
  await page.mouse.down({ button: "right" });
  await page.mouse.move(...to, { steps: 4 });
  await page.waitForFunction(() => window.__lab.route.movePreview().length > 0);
  const held = await watch(page, orderOther, 30);
  await page.keyboard.press("Escape");
  await page.mouse.up({ button: "right" });
  ctx.check(
    "a held drag's ghosts stay while other units take orders",
    held.length > 20 && held.every(([, , marks]) => marks > 0),
    JSON.stringify(held.map(([, , marks]) => marks)),
  );
  await page.close();
}
