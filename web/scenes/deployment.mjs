// Slice 12: one reversible deployment value, driven through the real command
// path. Durations come from the one fixture owner.
import { readFile, writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { lab, advance } from "./_lab.mjs";

const village = JSON.parse(
  await readFile(new URL("../../fixtures/village.json", import.meta.url), "utf8"),
);
const N = Math.round(village.service.deploy_and_pack_s * village.tick_hz);

const supply = async (page) =>
  lab(page, () => window.__lab.route.observation().own.find((u) => u.kind === "supply"));
const command = (page, order, queued = false) =>
  lab(page, ([o, q]) => window.__lab.route.command(o, q), [order, queued]);
const move = (goal) => ({ kind: "move", units: [0], gesture: 1, goal, route: "shortest" });
const STOP = { kind: "stop", units: [0] };
const deploy = (deployed) => ({ kind: "set_deployment", units: [0], deployed });
const same = (a, b) => a[0] === b[0] && a[1] === b[1];
const ticksOf = (u) => Math.round(u.deployment.progress * N);

/** Wait until the panel shows the latest tick, then draw one frame. */
async function settle(page) {
  const tick = await lab(page, () => window.__lab.route.tick());
  await page.waitForFunction(
    (t) =>
      document.querySelector("[data-testid=deployment-panel]")?.textContent.includes(`Tick ${t}`),
    tick,
  );
  await page.waitForTimeout(60);
  await page.evaluate(() => window.__lab.frame());
}

/** Camera target relative to the truck: every state is framed the same way. */
const FRAMING_OFFSET = [2, 10];

/** Full frame, a 2× crop holding the truck and its whole ring, and a 2× readout crop. */
async function capture(ctx, page, name) {
  const u = await supply(page);
  await lab(
    page,
    ([p, o]) =>
      window.__lab.setCamera({ ...window.__lab.camera(), target: [p[0] + o[0], p[1] + o[1], 0] }),
    [u.position, FRAMING_OFFSET],
  );
  await settle(page);
  const shot = await page.screenshot();
  await writeFile(ctx.evidencePath(`frame-${name}-1280x800.png`), shot);
  const png = decode(shot);
  const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), u.position);
  // The ring (6 m radius, plus arrowhead overhang) projected at 32 points.
  const ring = await lab(
    page,
    (p) =>
      Array.from({ length: 32 }, (_, k) =>
        window.__lab.projectToCss(
          p[0] + 6.5 * Math.cos((k / 32) * Math.PI * 2),
          p[1] + 6.5 * Math.sin((k / 32) * Math.PI * 2),
          p[2],
        ),
      ),
    u.position,
  );
  const xs = ring.map((q) => q[0]),
    ys = ring.map((q) => q[1]);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  await writeCrop(
    png,
    ctx.evidencePath(`crop-supply-${name}-2x.png`),
    (x0 + x1) / 2,
    (y0 + y1) / 2,
    Math.ceil((x1 - x0) / 2) + 16,
    Math.ceil((y1 - y0) / 2) + 16,
    2,
  );
  const box = await page.getByTestId("deployment-readout").boundingBox();
  await writeCrop(
    png,
    ctx.evidencePath(`crop-readout-${name}-2x.png`),
    box.x + box.width / 2,
    box.y + box.height / 2,
    Math.ceil(box.width / 2) + 4,
    Math.ceil(box.height / 2) + 4,
    2,
  );
  const panel = await page.getByTestId("deployment-panel").boundingBox();
  const inside = ([x, y]) => x > panel.x + panel.width + 8 && x < 1272 && y > 8 && y < 792;
  framing.push({ name, at, clear: ring.every(inside) });
  return at;
}

const framing = [];

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 20000 });
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);

  // A stopped supply unit sets up from spawn: progress is tick / N.
  const t0 = await lab(page, () => window.__lab.route.tick());
  await advance(page, Math.round(N * 0.4) - t0);
  let u = await supply(page);
  const home = u.position;
  await capture(ctx, page, "deploying-40");
  ctx.check(
    "a stopped supply unit deploys toward fully deployed",
    u.deployment.target === "deployed" && Math.abs(u.deployment.progress - 0.4) < 1e-6,
    JSON.stringify(u.deployment),
  );

  // 40% deployed: packing takes 40% of the duration, without translation.
  await command(page, move([140, 100]));
  await advance(page, Math.round(N * 0.4) - 1);
  u = await supply(page);
  ctx.check(
    "a 40%-deployed unit is still packing, unmoved, one tick short of 40% of the duration",
    u.state === "packing" && ticksOf(u) === 1 && same(u.position, home),
    JSON.stringify({ state: u.state, progress: u.deployment.progress, at: u.position }),
  );
  await advance(page, 1);
  u = await supply(page);
  ctx.check(
    "and is fully packed at exactly 40% of the duration",
    u.deployment.progress === 0 && u.deployment.target === "packed",
    JSON.stringify(u.deployment),
  );
  await advance(page, 20);
  u = await supply(page);
  const moved = u.position;
  ctx.check(
    "movement starts only once packed",
    moved[0] > home[0] + 3 && u.state === "moving",
    JSON.stringify({ at: moved, state: u.state }),
  );

  // Stop clears the move; setup from zero takes the whole duration.
  await command(page, STOP);
  await advance(page, 1);
  u = await supply(page);
  ctx.check(
    "Stop clears movement and returns the desired state to deployed",
    u.goal === null && u.queue.length === 0 && u.deployment.target === "deployed",
    JSON.stringify({ goal: u.goal, queue: u.queue, deployment: u.deployment }),
  );
  await advance(page, N - 2);
  u = await supply(page);
  await settle(page);
  const notYet = await page.getByTestId("service-ready").textContent();
  ctx.check(
    "no service readiness one tick before fully deployed",
    ticksOf(u) === N - 1 && /not ready/.test(notYet),
    notYet,
  );
  await advance(page, 1);
  u = await supply(page);
  await capture(ctx, page, "deployed");
  const ready = await page.getByTestId("service-ready").textContent();
  ctx.check(
    "deploying from packed takes the full duration, then service is ready",
    u.deployment.progress === 1 &&
      /ready \(fully deployed\)/.test(ready) &&
      same(u.position, moved),
    JSON.stringify({ deployment: u.deployment, ready }),
  );

  // A queued move while deployed packs first (captured at 75% deployed);
  // halfway, Stop reverses it.
  await command(page, move([80, 100]), true);
  const quarterTicks = Math.round(N / 4);
  await advance(page, quarterTicks);
  await capture(ctx, page, "packing-75");
  const quarter = await supply(page);
  await advance(page, N / 2 - quarterTicks);
  u = await supply(page);
  ctx.check(
    "a queued move while deployed packs first, in place",
    quarter.state === "packing" &&
      ticksOf(quarter) === N - quarterTicks &&
      u.state === "packing" &&
      u.deployment.target === "packed" &&
      Math.abs(u.deployment.progress - 0.5) < 1e-6 &&
      same(u.position, moved),
    JSON.stringify({ state: u.state, deployment: u.deployment }),
  );
  await command(page, STOP);
  await advance(page, N / 2 - 1);
  u = await supply(page);
  const short = u.deployment.progress;
  await advance(page, 1);
  u = await supply(page);
  ctx.check(
    "a 50%-packed reversal redeploys in half the duration",
    short < 1 && u.deployment.progress === 1 && same(u.position, moved),
    JSON.stringify({ oneTickBefore: short, after: u.deployment }),
  );

  // Explicit Pack: the same duration down, and it stays packed.
  await command(page, deploy(false));
  await advance(page, N - 1);
  u = await supply(page);
  const packing = u.deployment.progress;
  await advance(page, 1);
  u = await supply(page);
  await advance(page, 30);
  const held = await supply(page);
  await capture(ctx, page, "packed");
  ctx.check(
    "packing takes the same duration as deploying, and Pack holds it packed",
    packing > 0 && u.deployment.progress === 0 && held.deployment.progress === 0,
    JSON.stringify({ oneTickBefore: packing, packed: u.deployment, held: held.deployment }),
  );

  // Repeated reversals: net progress is exactly the sum of the ticks spent each way.
  await command(page, deploy(true));
  await advance(page, 200);
  let expected = 200;
  for (const [i, span] of [37, 52, 23, 61, 44, 29, 70, 18].entries()) {
    await command(page, i % 2 === 0 ? move([140, 100]) : STOP);
    await advance(page, span);
    expected = Math.min(N, Math.max(0, expected + (i % 2 === 0 ? -span : span)));
  }
  u = await supply(page);
  ctx.check(
    "repeated reversals neither reset nor add progress, and leave no orders behind",
    ticksOf(u) === expected && u.goal === null && u.queue.length === 0 && u.mounts.length === 0,
    JSON.stringify({ ticks: ticksOf(u), expected, goal: u.goal }),
  );
  await capture(ctx, page, "reversals");
  ctx.check(
    "every capture frames the truck and its progress ring clear of the panel",
    framing.every((f) => f.clear),
    JSON.stringify(framing.map((f) => [f.name, f.at.map(Math.round)])),
  );
  const acks = await lab(page, () => window.__lab.route.acks());
  ctx.check(
    "every deployment command was acknowledged without error",
    acks.length > 0 && acks.every((a) => a.ack.error === null),
    JSON.stringify(acks.slice(0, 3).map((a) => a.label)),
  );
}
