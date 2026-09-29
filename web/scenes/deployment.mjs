// Deployment through the browser command, publication and DOM path. Rust
// deployment tests own the duration and repeated-reversal matrix.
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { lab, advance, openBattle } from "./_lab.mjs";
import { unitType, village } from "./_units.mjs";

const N = Math.round(unitType("supply").capabilities.deploy.seconds * village.tick_hz);

const supply = async (page) =>
  lab(page, () => window.__lab.route.observation().own.find((u) => u.kind === "supply"));
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

/** Full frame, a 2× crop holding the truck and the ground 6.5 m round it
 *  (where no mark of its deployment lies: that is its panel's row), and a 2×
 *  readout crop. */
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
  // The ground 6.5 m round the truck, projected at 32 points.
  const around = await lab(
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
  const xs = around.map((q) => q[0]),
    ys = around.map((q) => q[1]);
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
  framing.push({ name, at, clear: around.every(inside) });
  return at;
}

const framing = [];

export async function run(ctx) {
  const page = await openBattle(ctx);
  await page.waitForFunction(() => window.__lab.route.selected().length === 1);
  const command = async (order, queued = false) => {
    const ack = await lab(page, ([o, q]) => window.__lab.route.command(o, q), [order, queued]);
    ctx.check(
      `${order.kind} command ${ack?.seq} is accepted`,
      ack?.error === null,
      JSON.stringify(ack),
    );
  };

  // A stopped supply unit sets up from spawn: progress is tick / N.
  const t0 = await lab(page, () => window.__lab.route.tick());
  await advance(page, Math.round(N * 0.4) - t0);
  let u = await supply(page);
  const home = u.position;
  await capture(ctx, page, "deploying-40");
  ctx.check(
    "a stopped supply unit deploys toward fully deployed",
    u.deployment.target === "deployed" && u.deployment.progress > 0 && u.deployment.progress < 1,
    JSON.stringify(u.deployment),
  );

  // A move packs in place before translation; exact reversal timings belong to Rust.
  await command(move([140, 100]));
  await advance(page, 1);
  u = await supply(page);
  ctx.check(
    "the move publishes packing before the truck translates",
    u.state === "packing" && u.deployment.progress > 0 && same(u.position, home),
    JSON.stringify({ state: u.state, deployment: u.deployment, at: u.position }),
  );
  await advance(page, Math.round(N * 0.4) + 19);
  u = await supply(page);
  const moved = u.position;
  ctx.check(
    "movement starts only once packed",
    moved[0] > home[0] + 3 && u.state === "moving" && u.deployment.progress === 0,
    JSON.stringify({ at: moved, state: u.state }),
  );

  // Stop clears the move; setup from zero takes the whole duration.
  await command(STOP);
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

  // Queued movement uses the same browser path and presents packing in place.
  await command(move([80, 100]), true);
  await advance(page, Math.round(N / 4));
  await capture(ctx, page, "packing-75");
  u = await supply(page);
  ctx.check(
    "a queued move while deployed publishes packing in place",
    u.state === "packing" &&
      u.deployment.target === "packed" &&
      u.deployment.progress > 0 &&
      u.deployment.progress < 1 &&
      same(u.position, moved),
    JSON.stringify({ state: u.state, deployment: u.deployment }),
  );

  // Stop cancels movement; explicit Pack then holds the truck packed.
  await command(STOP);
  await command(deploy(false));
  await advance(page, N);
  const packed = await supply(page);
  await advance(page, 30);
  u = await supply(page);
  await capture(ctx, page, "packed");
  ctx.check(
    "Pack leaves the truck stationary and packed",
    packed.deployment.progress === 0 &&
      u.deployment.progress === 0 &&
      u.deployment.target === "packed" &&
      u.goal === null &&
      u.queue.length === 0 &&
      same(u.position, moved),
    JSON.stringify({ deployment: u.deployment, goal: u.goal, at: u.position }),
  );
  ctx.check(
    "every capture frames the truck clear of the panel",
    framing.every((f) => f.clear),
    JSON.stringify(framing.map((f) => [f.name, f.at.map(Math.round)])),
  );
}
