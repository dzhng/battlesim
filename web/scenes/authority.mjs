// Slice 03: one worker authority; ordered commands; honest status; replay parity.
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { unitType } from "./_units.mjs";

const route = (page, fn, arg) => page.evaluate(fn, arg);
const tick = (page) => route(page, () => window.__lab.route.tick());
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

/** The jeep and the field works (sandbags, fence, dragon's
 *  teeth) are drawn as their own appearances once the side knows them. */
async function fieldWorks(ctx, page) {
  const wanted = ["sandbags", "fence", "dragon_tooth"];
  await page
    .waitForFunction(
      (names) => {
        const drawn = new Set(window.__lab.route.structures().map((s) => s.appearance));
        return names.every((n) => drawn.has(n));
      },
      wanted,
      { timeout: 10000 },
    )
    .catch(() => {});
  await route(page, () => window.__lab.frame());
  const drawn = await route(page, () => window.__lab.route.structures().map((s) => s.appearance));
  const vehicles = await route(page, () =>
    window.__lab.route.vehicles().map((v) => ({ appearance: v.appearance })),
  );
  ctx.check(
    "the jeep is drawn as its appearance",
    vehicles.some((v) => v.appearance === unitType("test_jeep").appearance),
    JSON.stringify(vehicles.map((v) => v.appearance)),
  );
  ctx.check(
    "the sandbags, fence and dragon's teeth are drawn as their appearances",
    wanted.every((n) => drawn.includes(n)),
    JSON.stringify([...new Set(drawn)]),
  );
  const camera = await route(page, () => window.__lab.camera());
  await route(page, (v) => window.__lab.setCamera({ ...window.__lab.camera(), ...v }), {
    target: [56, 116, 0],
    distance: 36,
    pitch: 0.8,
    yaw: -1.2,
  });
  await route(page, () => window.__lab.frame());
  await writeFile(ctx.evidencePath("field-works.png"), await page.screenshot());
  await route(page, (c) => window.__lab.setCamera(c), camera);
}

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 5, undefined, { timeout: 10000 });
  const t0 = await tick(page);
  await wait(400);
  const t1 = await tick(page);
  ctx.check(
    "the worker ticks in real time once the frame is up",
    t1 - t0 >= 8 && t1 - t0 <= 16,
    `${t0} → ${t1}`,
  );

  await fieldWorks(ctx, page);

  // Select the tank by clicking it, then right-click the ground to move.
  // Under heavy load a click can land before a fresh frame; wait for the
  // selection to register, and click once more at a fresh projection if not.
  const tank = await route(page, () =>
    window.__lab.route.observation().own.find((u) => u.kind === "test_tank"),
  );
  const selectTank = async () => {
    await route(page, () => window.__lab.frame());
    const { position } = await route(
      page,
      (id) => window.__lab.route.observation().own.find((u) => u.id === id),
      tank.id,
    );
    const at = await route(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1), position);
    await page.mouse.click(at[0], at[1]);
    return page
      .waitForFunction(() => window.__lab.route.selected().length === 1, undefined, {
        timeout: 5000,
      })
      .then(() => true)
      .catch(() => false);
  };
  const selected = (await selectTank()) || (await selectTank());
  ctx.check("clicking the tank selects it", selected);
  const goalPx = await route(page, () => window.__lab.projectToCss(85, 150, 0));
  await page.mouse.click(goalPx[0], goalPx[1], { button: "right" });
  await page.waitForFunction(() => window.__lab.route.acks().length > 0);
  const [ack] = await route(page, () => window.__lab.route.acks());
  ctx.check(
    "a right-click move is acknowledged for the next tick",
    ack.seq === 1 && !ack.ack.error,
    JSON.stringify(ack),
  );
  // Startup acceleration is a vehicle rule; authority checks eventual progress.
  await page.waitForFunction(
    ({ id, from }) => {
      const unit = window.__lab.route.observation().own.find((u) => u.id === id);
      return unit.position[0] > from[0] + 1;
    },
    { id: tank.id, from: tank.position },
    { timeout: 10000, polling: 100 },
  );
  const moved = await route(
    page,
    (id) => window.__lab.route.observation().own.find((u) => u.id === id),
    tank.id,
  );
  ctx.check(
    "the tank drives toward the clicked point",
    moved.position[0] > tank.position[0] + 1 && Math.abs(moved.goal[0] - 85) < 1,
    JSON.stringify({ from: tank.position, now: moved.position, goal: moved.goal }),
  );

  const bad = await route(page, () =>
    window.__lab.route.command({
      kind: "move",
      units: [99],
      gesture: 1,
      goal: [10, 10],
      route: "shortest",
    }),
  );
  ctx.check(
    "a command naming an unknown unit is rejected with a reason",
    bad.error?.reason === "unknown_unit",
    JSON.stringify(bad),
  );
  await page.evaluate(() => window.__lab.frame());
  const log = await page.getByTestId("ack-log").textContent();
  ctx.check(
    "the ack log leads with the verdict's mark and names unit, target, reason and tick, newest first",
    /✕ #2 move unit #99 .* · rejected: unknown unit✓ #1 move test_tank #0 to \(8\d, 1\d\d\) · tick \d+/.test(
      log,
    ),
    log,
  );
  const selection = await page.getByTestId("selection").textContent();
  ctx.check(
    "the panel names the selected unit and its order",
    /test_tank #0: moving to \(8\d, 1\d\d\)/.test(selection),
    selection,
  );

  // Withheld credit stalls the battle visibly instead of queuing or dropping.
  await route(page, () => window.__lab.route.setWithhold(true));
  await wait(300);
  const held = await tick(page);
  await wait(400);
  const stillHeld = await tick(page);
  const heldStatus = await page.getByTestId("authority-status").textContent();
  ctx.check(
    "withheld credit stops the ticks and says so",
    held === stillHeld && /waiting-consumer/.test(heldStatus),
    `${held} → ${stillHeld}; ${heldStatus}`,
  );
  await route(page, () => window.__lab.route.setWithhold(false));
  await wait(400);
  ctx.check(
    "returning credit resumes",
    (await tick(page)) > stillHeld + 4,
    `${stillHeld} → ${await tick(page)}`,
  );

  // Pause and step.
  await page.getByRole("button", { name: "Pause" }).click();
  await wait(200);
  const p0 = await tick(page);
  await wait(300);
  ctx.check("pause holds the tick", (await tick(page)) === p0, `${p0}`);
  await page.getByRole("button", { name: "Step" }).click();
  await page.waitForFunction((p) => window.__lab.route.tick() === p + 1, p0, { timeout: 3000 });
  ctx.check("step advances exactly one tick", (await tick(page)) === p0 + 1);

  // A hidden tab suspends the authority.
  await page.getByRole("button", { name: "Resume" }).click();
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", { configurable: true, get: () => true });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await wait(250);
  const h0 = await tick(page);
  await wait(300);
  ctx.check("a hidden tab suspends ticking", (await tick(page)) === h0, `${h0}`);
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", { configurable: true, get: () => false });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await wait(300);

  const frame = await page.screenshot();
  await writeFile(ctx.evidencePath("frame-1280x800.png"), frame);
  await writeCrop(decode(frame), ctx.evidencePath("crop-panel-2x.png"), 190, 150, 180, 150, 2);
  const unitNow = await route(
    page,
    (id) => window.__lab.route.observation().own.find((u) => u.id === id),
    tank.id,
  );
  const unitPx = await route(
    page,
    (p) => window.__lab.projectToCss(p[0], p[1], p[2]),
    unitNow.position,
  );
  await writeCrop(
    decode(frame),
    ctx.evidencePath("crop-tank-3x.png"),
    unitPx[0],
    unitPx[1],
    70,
    50,
    3,
  );

  // Replay in-thread from the accepted commands: every tick digest matches the worker's.
  const check = await route(page, () => window.__lab.route.checkReplay());
  ctx.check(
    "an in-thread replay matches every worker tick digest",
    check.state === "match" && check.ticks > 30,
    JSON.stringify(check),
  );

  // Reset restarts the clock with an empty command log.
  await route(page, () => window.__lab.route.reset());
  await page.waitForFunction(
    () => window.__lab.route.acks().length === 0 && window.__lab.route.tick() < 20,
  );
  ctx.check("reset empties the command log and restarts the clock", true);
}
