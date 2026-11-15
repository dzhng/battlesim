// Two real pages, two credentials, one threaded local authority.
import { spawn, execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { once } from "node:events";
import { captureSeats, resultColours } from "./_battle-colours.mjs";

const root = new URL("../../", import.meta.url).pathname;
const evidence = new URL("../../throwaway/multiplayer/slice08/", import.meta.url).pathname;
const observation = (page) => page.evaluate(() => window.__lab.route.observation());

async function server() {
  execFileSync("bun", ["apps/game-server/build.mjs"], { cwd: root });
  const child = spawn("node", ["throwaway/game-server/main.mjs", "--dev"], {
    cwd: root,
    env: { ...process.env, PORT: "0" },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let log = "";
  child.stderr.on("data", (data) => {
    log += data;
  });
  const address = await new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      child.kill();
      reject(new Error(`server startup: ${log}`));
    }, 30_000);
    child.once("exit", (code) => {
      clearTimeout(timeout);
      reject(new Error(`server exited ${code}: ${log}`));
    });
    child.stdout.on("data", (data) => {
      log += data;
      const match = /Game server: (http:\/\/127\.0\.0\.1:\d+)/.exec(log);
      if (match) {
        clearTimeout(timeout);
        resolve(match[1]);
      }
    });
  });
  return {
    address,
    hold: () => child.kill("SIGSTOP"),
    resume: () => child.kill("SIGCONT"),
    async stop() {
      const exited = once(child, "exit");
      child.kill("SIGCONT");
      child.kill("SIGTERM");
      await exited;
      await writeFile(`${evidence}server.log`, log);
    },
  };
}

async function purchase(page, ticket) {
  const card = await page.evaluate(async ({ side, setup, player }) => {
    const { catalogSet } = await import("/src/battle/catalog/sets.ts");
    const catalog = await catalogSet("game");
    const economy = window.__lab.route.observation().skirmish.players.find((p) => p.id === player);
    // Choose affordable mobile ground scouts from the live catalog, without
    // fixing a unit id, a price, or a tuning value in the test.
    const cards = catalog.units.cards.filter(
      (card) =>
        card.disabled_reason === null &&
        card.roster.factions.includes(setup.factions[side]) &&
        card.cost <= economy.credits &&
        catalog.units.has(card.id) &&
        "wheeled" in catalog.units.type(card.id).mobility,
    );
    cards.sort(
      (a, b) =>
        catalog.units.type(b.id).sensors.ground_m - catalog.units.type(a.id).sensors.ground_m,
    );
    return cards[0];
  }, ticket);
  if (!card) throw new Error("the faction has no affordable wheeled scout");
  const start = await page.evaluate(() => window.__lab.route.prepared().start.at);
  await page.evaluate((at) => {
    const camera = window.__lab.camera();
    window.__lab.setCamera({ ...camera, target: [...at, 0], distance: 160 });
  }, start);
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("textbox", { name: "Search units" }).fill(card.name);
  // A family click chooses its first affordable variant; the test observes
  // what the real picker chooses, rather than imposing a loadout.
  await page.getByRole("tabpanel").getByRole("button").first().click();
  await page.mouse.move(640, 360);
  await page.waitForFunction(() => window.__lab.route.purchaseGhost()?.valid === true, null, {
    timeout: 30_000,
  });
  await page.mouse.click(640, 360);
  await page.waitForFunction(() =>
    window.__lab.route
      .acks()
      .some(
        ({ order, ack }) =>
          order.kind === "confirm_purchase" && !ack.error && ack.refused.length === 0,
      ),
  );
}

async function controls(ctx, page, side) {
  const before = (await observation(page)).tick;
  const action = /^(?:pause|resume|restart)\b|speed|(?:export|save).*replay/i;
  const forbidden = page
    .getByRole("button", { name: action })
    .or(page.getByRole("link", { name: action }))
    .or(page.getByRole("slider", { name: /speed/i }))
    .or(page.getByRole("spinbutton", { name: /speed/i }))
    .or(page.getByRole("combobox", { name: /speed/i }));
  ctx.check(
    `${side}: the battle HUD offers no pause, speed or replay export`,
    (await forbidden.count()) === 0,
  );
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("dialog", { name: "Menu", exact: true }).waitFor();
  ctx.check(
    `${side}: local authority controls cannot be found in the online menu`,
    (await forbidden.count()) === 0,
  );
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  ctx.check(`${side}: settings offer no speed or replay export`, (await forbidden.count()) === 0);
  await page.getByRole("button", { name: "Close settings", exact: true }).click();
  await page.waitForFunction((tick) => window.__lab.route.tick() > tick, before);
  ctx.check(
    `${side}: opening the menu leaves the battle running`,
    (await observation(page)).tick > before,
  );
  await page.getByRole("button", { name: "Back to battle", exact: true }).click();
}

async function orderMove(page, goal, fast = false) {
  const unit = (await observation(page)).own[0];
  await page.evaluate(
    ({ id, goal }) => {
      window.__lab.route.select([id]);
      const camera = window.__lab.camera();
      window.__lab.setCamera({ ...camera, target: [...goal, 0], distance: 220 });
    },
    { id: unit.id, goal },
  );
  await page.waitForTimeout(200);
  const css = await page.evaluate(
    ([x, y]) => window.__lab.projectToCss(x, y, window.__lab.route.surfaceZ(x, y)),
    goal,
  );
  if (!css) throw new Error("move destination is not drawn");
  if (fast) await page.mouse.dblclick(css[0], css[1], { button: "right" });
  else await page.mouse.click(css[0], css[1], { button: "right" });
  await page.waitForFunction(() =>
    window.__lab.route
      .acks()
      .some(({ order, ack }) => order.kind === "move" && !ack.error && ack.refused.length === 0),
  );
  return unit;
}

async function move(page, goal) {
  const unit = await orderMove(page, goal);
  await page.waitForFunction(
    ({ id, position, goal }) => {
      const u = window.__lab.route.observation().own.find((u) => u.id === id);
      const before = Math.hypot(goal[0] - position[0], goal[1] - position[1]);
      return u && Math.hypot(goal[0] - u.position[0], goal[1] - u.position[1]) < before * 0.75;
    },
    { id: unit.id, position: unit.position, goal },
    { timeout: 60_000 },
  );
}

export async function run(ctx) {
  await mkdir(evidence, { recursive: true });
  const colours = new URL("../../throwaway/multiplayer/slice11/", import.meta.url).pathname;
  const captures = process.env.SLICE11_BEFORE ? `${colours}before/` : colours;
  await mkdir(captures, { recursive: true });
  await resultColours(ctx, new URL(ctx.url).origin, captures);
  const local = await server();
  const url = `${new URL(ctx.url).origin}/online/dev?server=${encodeURIComponent(local.address)}&quality=${ctx.graphicsQuality}`;
  try {
    const pages = [
      await ctx.newPage({ viewport: { width: 1280, height: 720 } }),
      await ctx.newPage({ viewport: { width: 1280, height: 720 } }),
    ];
    const tickets = [];
    for (const page of pages) {
      const response = page.waitForResponse(
        (r) => r.url().endsWith("/dev/battle") && r.request().method() === "POST",
      );
      const opening = ctx.openLab(page, url, 120_000);
      tickets.push(await (await response).json());
      await opening;
    }
    const mismatch = await ctx.newPage();
    let streams = 0;
    mismatch.on("request", (request) => {
      if (/\/battles\/[^/]+\/stream/.test(request.url())) streams++;
    });
    await mismatch.route("**/dev/battle", (route) =>
      route.fulfill({ json: { ...tickets[0], mapIdentity: "wrong-map" } }),
    );
    await mismatch.goto(url);
    await mismatch.getByRole("alert").waitFor({ timeout: 120_000 });
    const details = mismatch.getByRole("button", { name: "Details", exact: true });
    await details.click();
    ctx.check(
      "a mismatched map refuses before any battle stream opens",
      streams === 0 && /local map differs/.test(await mismatch.getByRole("alert").textContent()),
      `streams opened: ${streams}`,
    );
    if (streams !== 0) throw new Error("A mismatched page opened a battle stream");
    await mismatch.close();
    ctx.check(
      "the first page commands Blue and the second commands Red in the same battle",
      tickets[0].side === "blue" &&
        tickets[1].side === "red" &&
        tickets[0].battle === tickets[1].battle,
    );
    await Promise.all(pages.map((page, i) => purchase(page, tickets[i])));
    await captureSeats(ctx, local, pages, "preparation", captures);
    for (const page of pages) {
      const ready = page.getByRole("button", { name: "START BATTLE", exact: true });
      if (await ready.isVisible()) await ready.click();
    }
    await Promise.all(
      pages.map((page) =>
        page.waitForFunction(
          () =>
            window.__lab.route.observation().own.length > 0 &&
            window.__lab.route.observation().skirmish.phase === "active",
          null,
          { timeout: 120_000 },
        ),
      ),
    );
    const initial = await Promise.all(pages.map(observation));
    for (let i = 0; i < pages.length; i++) {
      const o = initial[i];
      const ticket = tickets[i];
      const owner = o.skirmish.players.find((p) => p.id === ticket.player)?.index;
      ctx.check(
        `${ticket.side}: own records belong to its seated player, never the opposite side`,
        Number.isInteger(owner) &&
          o.groundPatch.side === ticket.side &&
          o.own.length > 0 &&
          o.own.every((u) => u.owner === owner),
      );
      ctx.check(
        `${ticket.side}: the enemy is absent while the two armies are out of sight`,
        o.identified.length === 0,
      );
      await controls(ctx, pages[i], ticket.side);
    }
    const starts = await Promise.all(
      pages.map((page) => page.evaluate(() => window.__lab.route.prepared().start.at)),
    );
    ctx.check(
      "commanders open on different home roads",
      JSON.stringify(starts[0]) !== JSON.stringify(starts[1]),
    );
    const middle = [(starts[0][0] + starts[1][0]) / 2, (starts[0][1] + starts[1][1]) / 2];
    const objectives = initial.map(
      (o, i) =>
        [...o.skirmish.objectives].sort(
          (a, b) =>
            Math.hypot(a.center[0] - starts[i][0], a.center[1] - starts[i][1]) -
            Math.hypot(b.center[0] - starts[i][0], b.center[1] - starts[i][1]),
        )[0],
    );
    try {
      await Promise.all(pages.map((page, i) => orderMove(page, objectives[i].center, true)));
    } catch (error) {
      await ctx.writeEvidence("failed-objective-move.json", {
        objectives,
        observations: await Promise.all(pages.map(observation)),
      });
      throw error;
    }
    await pages[0].waitForTimeout(1500);
    await captureSeats(ctx, local, pages, "moving", captures);
    await Promise.all(
      pages.map((page, i) =>
        page.waitForFunction(
          (side) =>
            window.__lab.route
              .observation()
              .skirmish.objectives.some((o) => o.capturing === side || o.owner === side),
          tickets[i].side,
          { timeout: 300_000 },
        ),
      ),
    );
    await captureSeats(ctx, local, pages, "capture", captures);
    await Promise.all(
      pages.map((page, i) =>
        page.waitForFunction(
          (side) =>
            window.__lab.route.observation().skirmish.objectives.some((o) => o.owner === side),
          tickets[i].side,
          { timeout: 60_000 },
        ),
      ),
    );
    await captureSeats(ctx, local, pages, "held", captures);
    // Both drive toward the same meeting ground. The page issues the order
    // through picking and the ordinary right-click command path.
    await Promise.all(pages.map((page) => move(page, middle)));
    for (let i = 0; i < pages.length; i++) {
      ctx.check(
        `${tickets[i].side}: its right-click order moves its own unit`,
        (await observation(pages[i])).own.some(
          (u) =>
            Math.hypot(
              u.position[0] - initial[i].own[0].position[0],
              u.position[1] - initial[i].own[0].position[1],
            ) > 5,
        ),
      );
    }
    // The two armies are fighting by now, so what a page sees changes from
    // one frame to the next: the proof is taken in the frame where it holds.
    const sighted = await Promise.all(
      pages.map((page) =>
        page
          .waitForFunction(
            () => {
              const o = window.__lab.route.observation();
              const onSeenGround = o.identified.every((unit) => {
                const cell =
                  Math.floor(unit.position[1] / o.fog.cellM) * o.fog.nx +
                  Math.floor(unit.position[0] / o.fog.cellM);
                return !!(o.fog.bits[cell >>> 5] & (1 << (cell & 31)));
              });
              return o.identified.length > 0 && onSeenGround ? o.identified.length : false;
            },
            null,
            { timeout: 240_000 },
          )
          .then(
            async (count) => {
              await page.keyboard.press("Backspace");
              return count.jsonValue();
            },
            () => 0,
          ),
      ),
    );
    await captureSeats(ctx, local, pages, "contact", captures);
    for (let i = 0; i < pages.length; i++) {
      const page = pages[i];
      const side = tickets[i].side;
      ctx.check(
        `${side}: enemy records appear when the enemy enters its sight`,
        sighted[i] > 0,
        `${sighted[i]} identified, each on ground it sees`,
      );
      const o = await observation(page);
      await page.evaluate((at) => {
        const camera = window.__lab.camera();
        window.__lab.setCamera({ ...camera, target: at, distance: 200 });
      }, o.own[0].position);
      await page.waitForTimeout(500);
      ctx.check(
        `${side}: the running battlefield has no error overlay`,
        !(await page.getByRole("alert").isVisible()) &&
          !(await page.getByTestId("loading").isVisible()),
      );
      await page.screenshot({ path: `${evidence}${side}.png` });
    }
    await ctx.writeEvidence("observations.json", {
      initial,
      final: await Promise.all(pages.map(observation)),
    });
    for (const page of pages) await page.close();
  } finally {
    await local.stop();
  }
}
