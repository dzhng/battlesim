import { advance, obs, openMenu, closeMenu, presented, snapshot, groundCss } from "./_lab.mjs";

/** The actual player entry and retained page owners, across complete battles. */
export async function appJourney(ctx) {
  const page = await ctx.newPage();
  await page.addInitScript(() => {
    const probe = { devices: 0, contexts: [], music: [], allocations: null };
    window.__appJourney = probe;
    const request = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = async function (...args) {
      const device = await request.apply(this, args);
      probe.devices++;
      return device;
    };
    const NativeContext = AudioContext;
    window.AudioContext = class extends NativeContext {
      constructor(...args) {
        super(...args);
        probe.contexts.push(this);
      }
    };
    const start = AudioBufferSourceNode.prototype.start;
    AudioBufferSourceNode.prototype.start = function (...args) {
      if (this.loop && this.buffer?.duration > 200)
        probe.music.push({ buffer: this.buffer, offset: args[1] ?? 0 });
      return start.apply(this, args);
    };
  });
  await page.goto(new URL(ctx.url).origin);
  await page.getByTestId("menu-deploy").waitFor();
  await page.waitForFunction(() => window.__appJourney.music.length > 0);
  const origin = await page.evaluate(() => performance.timeOrigin);
  ctx.check(
    "menu prepares the actual full recording without a battle worker",
    page.workers().length === 0,
  );
  await page.getByRole("button", { name: "Developer", exact: true }).click();
  await page.getByRole("link", { name: "Village", exact: true }).click();

  const ready = async () => {
    await page.waitForFunction(() => window.__lab?.ready && window.__lab.route?.tick() > 3);
    await page.evaluate(() => window.__lab.route.pause());
    await page.evaluate(() => {
      window.__appJourney.allocations = window.__lab.allocations;
    });
  };
  await ready();
  const seedAddress = page.url();
  await advance(page, 90 - (await obs(page)).tick);
  await presented(page);
  const digest = await page.evaluate(() => window.__lab.route.digest());
  const own = (await obs(page)).own[0].id;
  await page.evaluate((id) => window.__lab.route.select([id]), own);
  await snapshot(ctx, page, "app-journey-play.png");
  const toolbar = page.getByRole("toolbar", { name: "Commands", exact: true });
  ctx.check("a selected live unit exposes the real command toolbar", (await toolbar.count()) === 1);

  // The real pause controls own keyboard, pointer and camera input.
  await openMenu(page);
  const camera = await page.evaluate(() => window.__lab.camera());
  const log = await page.evaluate(() => window.__lab.route.acks());
  await page.getByRole("button", { name: "Resume", exact: true }).focus();
  await page.keyboard.press("KeyF");
  await page.keyboard.down("ArrowRight");
  await page.waitForTimeout(80);
  await page.keyboard.up("ArrowRight");
  await page.mouse.wheel(0, 300);
  await snapshot(ctx, page, "app-journey-pause.png");
  const covered = await page.evaluate(() => ({
    camera: window.__lab.camera(),
    acks: window.__lab.route.acks(),
    tick: window.__lab.route.tick(),
  }));
  ctx.check(
    "pause controls cannot move the camera, advance or issue commands",
    JSON.stringify(covered.camera) === JSON.stringify(camera) &&
      JSON.stringify(covered.acks) === JSON.stringify(log) &&
      covered.tick === 90,
    JSON.stringify(covered),
  );
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Save replay", exact: true }).click();
  await download;
  await closeMenu(page);
  // Positive controls: the same keys must work outside the menu.
  await page.keyboard.press("KeyF");
  await advance(page, 1);
  await page.waitForFunction(() => window.__lab.route.acks().length > 0);
  const activeLog = await page.evaluate(() => window.__lab.route.acks());
  ctx.check(
    "uncovered fire-policy shortcut reaches the authority",
    activeLog.some((entry) => entry.order.kind === "set_engagement" && entry.ack.error === null),
    JSON.stringify(activeLog),
  );
  await page.keyboard.down("ArrowRight");
  await page.waitForTimeout(80);
  await page.keyboard.up("ArrowRight");
  const steered = await page.evaluate(() => window.__lab.camera());
  ctx.check(
    "uncovered arrow key steers the camera",
    JSON.stringify(steered) !== JSON.stringify(camera),
  );

  // Restart closes the old authority; replaying exact inputs gives its digest.
  const old = page.workers().find((worker) => worker.url().includes("/sim/worker.ts"));
  const closed = old.waitForEvent("close");
  await openMenu(page);
  await page.getByRole("button", { name: "Restart", exact: true }).click();
  await closed;
  await ready();
  await advance(page, 90 - (await obs(page)).tick);
  ctx.check(
    "restart creates a fresh authority with the same exact battle",
    !page.workers().includes(old) &&
      (await page.evaluate(() => window.__lab.route.digest())) === digest,
  );

  await openMenu(page);
  const departed = Promise.all(page.workers().map((worker) => worker.waitForEvent("close")));
  await page.getByRole("link", { name: "Main menu", exact: true }).click();
  await departed;
  await page.getByTestId("menu-deploy").waitFor();
  await page.waitForFunction(() => {
    const counts = window.__appJourney.allocations();
    return counts.buffers === 0 && counts.textures === 0;
  });
  await page.goBack();
  await ready();
  await advance(page, 90 - (await obs(page)).tick);
  ctx.check(
    "history return starts fresh exact inputs",
    page.url() === seedAddress &&
      (await page.evaluate(() => window.__lab.route.digest())) === digest,
  );
  await page.goForward();
  await page.getByTestId("menu-deploy").waitFor();
  await page.goBack();
  await ready();
  await openMenu(page);
  await page.getByRole("link", { name: "Watch saved replay", exact: true }).click();
  await ready();
  await advance(page, 90 - (await obs(page)).tick);
  await presented(page);
  await page.evaluate(() =>
    window.__lab.route.select([window.__lab.route.observation().own[0].id]),
  );
  await page.locator('.hud-army-card[aria-pressed="true"]').waitFor();
  ctx.check(
    "saved replay preserves the captured digest and has no command buttons",
    (await page.evaluate(() => window.__lab.route.digest())) === digest &&
      (await toolbar.count()) === 0,
  );
  await snapshot(ctx, page, "app-journey-replay.png");
  await openMenu(page);
  await page.getByRole("link", { name: "Main menu", exact: true }).click();
  await page.getByTestId("menu-deploy").waitFor();
  await page.waitForFunction(() => {
    const counts = window.__appJourney.allocations();
    return counts.buffers === 0 && counts.textures === 0;
  });
  const owners = await page.evaluate(() => {
    const p = window.__appJourney;
    return {
      devices: p.devices,
      contexts: p.contexts.length,
      running: p.contexts[0].state,
      offsets: p.music.map((m) => m.offset),
      sameBuffer: p.music.every((m) => m.buffer === p.music[0].buffer),
      allocations: p.allocations(),
      timeOrigin: performance.timeOrigin,
    };
  });
  ctx.check(
    "complete player journey retains one GPU/audio bank and releases battle allocations",
    owners.devices === 1 &&
      owners.contexts === 1 &&
      owners.running === "running" &&
      owners.sameBuffer &&
      owners.offsets.every((offset) => offset === 0) &&
      owners.allocations.buffers === 0 &&
      owners.allocations.textures === 0 &&
      owners.timeOrigin === origin,
    JSON.stringify(owners),
  );
  await ctx.writeEvidence("app-journey.json", { digest, owners });
  await page.close();
}

/** Matched player-route captures, plus real card selection above compact commands. */
export async function armyJourney(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1600, height: 900 } });
  await page.goto(`${new URL(ctx.url).origin}/battle/village?seed=1`);
  await page.waitForFunction(() => window.__lab?.ready && window.__lab.route?.tick() > 3);
  await page.evaluate(() => window.__lab.route.pause());
  await advance(page, 90 - (await obs(page)).tick);
  await presented(page);
  await page.evaluate(() => {
    window.__lab.setCamera({
      ...window.__lab.camera(),
      target: [177.2584517672531, 807.2700211819969, 0],
      distance: 65,
      pitch: 0.85,
      yaw: -1.57,
    });
    window.__lab.route.select([4]);
  });
  await page.evaluate(() => document.fonts.ready);
  await page.mouse.move(0, 500);
  await snapshot(ctx, page, "army-player-single.png");
  const own = (await obs(page)).own;
  const roster = page.locator(".hud-army-card");
  ctx.check(
    "player army cards contain all own units, including unselected units",
    (await roster.count()) === own.length &&
      (await page.locator('.hud-army-card[aria-pressed="true"]').count()) === 1,
  );
  ctx.check(
    "matched army capture preserves the original battle digest",
    (await page.evaluate(() => window.__lab.route.digest())) === "905eacbe69a7d504",
  );
  await roster.first().click();
  await roster.nth(1).click({ modifiers: ["Shift"] });
  const selection = await page.evaluate(() => window.__lab.route.selected());
  ctx.check(
    "real card click and Shift-click select exactly the two card owners",
    selection.length === 2 && selection.includes(own[0].id) && selection.includes(own[1].id),
    JSON.stringify(selection),
  );
  await page.mouse.move(0, 500);
  await snapshot(ctx, page, "army-player-multiple.png");
  await roster.nth(1).hover();
  ctx.check(
    "real hovered army card owns the complete fact panel",
    (await page.getByRole("tooltip").getAttribute("data-unit")) === String(own[1].id),
  );
  await snapshot(ctx, page, "army-player-hover.png");
  await page.mouse.move(0, 500);
  await page.evaluate(() => window.__lab.route.select([]));
  ctx.check(
    "clearing selection preserves army cards and disables every command",
    (await roster.count()) === own.length &&
      (await page
        .getByRole("toolbar", { name: "Commands" })
        .evaluate((toolbar) =>
          [...toolbar.querySelectorAll("button")].every((button) => button.disabled),
        )),
  );
  await snapshot(ctx, page, "army-player-unselected.png");
  const rifle = own.find((unit) => unit.kind === "rifle");
  const truck = own.find((unit) => unit.kind === "supply");
  const goal = [truck.position[0] + 50, truck.position[1]];
  await page.evaluate((ids) => window.__lab.route.select(ids), [rifle.id, truck.id]);
  await page.keyboard.press("x");
  const point = await groundCss(page, goal);
  await page.mouse.click(...point, { button: "right" });
  await page.waitForFunction(() => window.__lab.route.acks()[0]?.order.kind === "attack_move");
  const ack = await page.evaluate(() => window.__lab.route.acks()[0]);
  ctx.check(
    "player attack-move admits both armed and unarmed selected units",
    ack.ack.error === null &&
      ack.order.units.includes(rifle.id) &&
      ack.order.units.includes(truck.id),
    JSON.stringify(ack),
  );
  await advance(page, 100);
  const movedTruck = (await obs(page)).own.find((unit) => unit.id === truck.id);
  ctx.check(
    "the unarmed truck moves on the actual attack-move order",
    Math.hypot(
      movedTruck.position[0] - truck.position[0],
      movedTruck.position[1] - truck.position[1],
    ) > 5,
    JSON.stringify({ before: truck.position, after: movedTruck.position }),
  );
  await page.close();
}
