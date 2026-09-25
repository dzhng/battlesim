// Slice 04: predictable routes, group intent, gestures, traffic and blockage.
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";

const lab = (page, fn, arg) => page.evaluate(fn, arg);
const unit = (page, id) =>
  lab(page, (i) => window.__lab.route.observation().own.find((u) => u.id === i), id);
const pathLength = (u) => {
  let [x, y] = u.position;
  let total = 0;
  for (const [px, py] of u.route) {
    total += Math.hypot(px - x, py - y);
    [x, y] = [px, py];
  }
  return total;
};
const until = async (page, predicate, arg, timeout = 20000) =>
  page.waitForFunction(predicate, arg, { timeout, polling: 100 });

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await until(page, () => window.__lab.route?.tick() > 3);

  // Shortest vs fastest from the same start area to neighbouring goals.
  await lab(page, () => window.__lab.route.demo("Shortest vs fastest"));
  await until(page, () =>
    window.__lab.route
      .observation()
      .own.filter((u) => u.kind === "tank")
      .every((u) => u.route.length),
  );
  const [shortTank, fastTank] = await lab(page, () =>
    window.__lab.route.observation().own.filter((u) => u.kind === "tank"),
  );
  ctx.check(
    "the fastest route takes the roads round the forest; the shortest cuts through",
    fastTank.policy === "fastest" &&
      shortTank.policy === "shortest" &&
      pathLength(fastTank) > pathLength(shortTank) * 1.5,
    JSON.stringify({
      shortest: pathLength(shortTank).toFixed(0),
      fastest: pathLength(fastTank).toFixed(0),
    }),
  );
  await page.evaluate(() => window.__lab.frame());
  const frame = await page.screenshot();
  await writeFile(ctx.evidencePath("frame-routes-1280x800.png"), frame);
  const junction = await lab(page, () => window.__lab.projectToCss(40, 60, 0));
  await writeCrop(
    decode(frame),
    ctx.evidencePath("crop-junction-3x.png"),
    junction[0],
    junction[1],
    90,
    60,
    3,
  );

  // Race them at full speed: the fast route arrives first despite its length.
  await lab(page, () => window.__lab.route.pause());
  let arrived = { fast: 0, short: 0 };
  for (let i = 0; i < 80 && (!arrived.fast || !arrived.short); i++) {
    await lab(page, () => window.__lab.route.advance(60));
    const tick = await lab(page, () => window.__lab.route.tick());
    const [s, f] = await Promise.all([unit(page, shortTank.id), unit(page, fastTank.id)]);
    if (!arrived.short && s.state === "idle") arrived.short = tick;
    if (!arrived.fast && f.state === "idle") arrived.fast = tick;
  }
  ctx.check(
    "the fast route arrives first",
    arrived.fast > 0 && arrived.fast < arrived.short,
    JSON.stringify(arrived),
  );

  // The road wall (tick 150) is learned on contact; a unit sent along the road detours round it.
  const tick = await lab(page, () => window.__lab.route.tick());
  ctx.check("the scenario wall is down", tick > 150, `${tick}`);
  await lab(
    page,
    (id) =>
      window.__lab.route.command({
        kind: "move",
        units: [id],
        gesture: 70,
        goal: [130, 60],
        route: "fastest",
      }),
    fastTank.id,
  );
  await lab(page, () => window.__lab.route.advance(10));
  const before = await unit(page, fastTank.id);
  await lab(
    page,
    (id) =>
      window.__lab.route.command(
        { kind: "move", units: [id], gesture: 71, goal: [230, 60], route: "shortest" },
        true,
      ),
    fastTank.id,
  );
  for (let i = 0; i < 40; i++) {
    await lab(page, () => window.__lab.route.advance(60));
    if ((await unit(page, fastTank.id)).state === "idle") break;
  }
  const after = await unit(page, fastTank.id);
  ctx.check(
    "a unit meets the new wall, replans and still arrives past it",
    after.state === "idle" && Math.hypot(after.position[0] - 230, after.position[1] - 60) < 2,
    JSON.stringify({ before: before.position, after: after.position, state: after.state }),
  );

  // Onto the plateau top: blocked, destination kept, reason shown.
  await lab(page, () => window.__lab.route.demo("Onto the cliff top"));
  await lab(page, () => window.__lab.route.advance(5));
  const supply = await lab(page, () =>
    window.__lab.route.observation().own.find((u) => u.kind === "supply"),
  );
  const panel = await page.getByTestId("selection").textContent();
  ctx.check(
    "an unreachable destination reports route blocked and keeps its order",
    supply.state === "route_blocked" && supply.goal && /route blocked/.test(panel),
    panel,
  );

  // Infantry use a gap vehicles cannot.
  await lab(page, () => window.__lab.route.demo("Infantry through the gap"));
  await lab(page, () => window.__lab.route.advance(3));
  const rifles = await lab(page, () =>
    window.__lab.route.observation().own.filter((u) => u.kind === "rifle"),
  );
  ctx.check(
    "rifle squads route through the 5 m gap",
    rifles.every(
      (r) =>
        r.route.some(([x, y]) => Math.abs(x - 200) < 8 && Math.abs(y - 325) < 8) ||
        pathLength(r) < 210,
    ),
    JSON.stringify(rifles.map((r) => pathLength(r).toFixed(0))),
  );

  // Real pointer gestures: box select, right-click, double right-click, Shift queue, S stop.
  await lab(page, () => window.__lab.route.resume());
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [60, 205, 0], distance: 120 }),
  );
  await page.evaluate(() => window.__lab.frame());
  const recon = await lab(page, () =>
    window.__lab.route.observation().own.find((u) => u.kind === "recon"),
  );
  const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), recon.position);
  await page.mouse.move(at[0] - 30, at[1] - 30);
  await page.mouse.down();
  await page.mouse.move(at[0] + 30, at[1] + 30, { steps: 5 });
  await page.mouse.up();
  const selected = await lab(page, () => window.__lab.route.selected());
  ctx.check(
    "drag box selects the unit under it",
    selected.includes(recon.id),
    JSON.stringify(selected),
  );
  await lab(page, (id) => window.__lab.route.select([id]), recon.id);
  const goal = await lab(page, () => window.__lab.projectToCss(95, 195, 0));
  await page.mouse.click(goal[0], goal[1], { button: "right" });
  await page.mouse.click(goal[0] + 2, goal[1] + 1, { button: "right" });
  await until(
    page,
    (id) => window.__lab.route.observation().own.find((u) => u.id === id).policy === "fastest",
    recon.id,
    5000,
  );
  const log = await page.getByTestId("ack-log").textContent();
  ctx.check(
    "double right-click upgrades the same gesture to the fast route",
    /upgrade gesture \d+ to fast route/.test(log) &&
      (await unit(page, recon.id)).queue.length === 0,
    log,
  );
  const later = await lab(page, () => window.__lab.projectToCss(95, 230, 0));
  await page.keyboard.down("Shift");
  await page.mouse.click(later[0], later[1], { button: "right" });
  await page.keyboard.up("Shift");
  await until(
    page,
    (id) => window.__lab.route.observation().own.find((u) => u.id === id).queue.length === 1,
    recon.id,
    5000,
  );
  ctx.check("Shift right-click queues a waypoint", true);
  await page.keyboard.press("s");
  await until(
    page,
    (id) => window.__lab.route.observation().own.find((u) => u.id === id).state === "idle",
    recon.id,
    5000,
  );
  const stopped = await unit(page, recon.id);
  ctx.check(
    "S stops and clears the queue",
    stopped.queue.length === 0 && stopped.goal === null,
    JSON.stringify(stopped.state),
  );
}
