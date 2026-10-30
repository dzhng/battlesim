import {
  advance,
  obs,
  openMenu,
  closeMenu,
  presented,
  snapshot,
  groundCss,
  pointerOffCanvas,
  menuShown,
  openMenuPage,
} from "./_lab.mjs";
import { readFileSync } from "node:fs";

/** The sound catalog's music: which tracks the menu and a battle may play. */
const MUSIC = JSON.parse(
  readFileSync(new URL("../../fixtures/sounds.json", import.meta.url)),
).music;

/** The actual player entry and retained page owners, across complete battles:
 *  the menu's Deploy into a generated battle, its pause controls, restart,
 *  history, a saved replay and the way back to the menu. */
export async function appJourney(ctx) {
  const page = await ctx.newPage();
  const cdp = await page.context().newCDPSession(page);
  await page.addInitScript(() => {
    // wgpu drops WebGPU handles without calling destroy(). Strong observer keys
    // would retain every departed scene; only weak identities belong here.
    const resources = new Map();
    const queues = [];
    let created = 0;
    const allocations = (before = Infinity) => {
      const counts = { buffers: 0, textures: 0 };
      for (const [order, { reference, kind }] of resources) {
        if (!reference.deref()) resources.delete(order);
        else if (order < before) counts[kind]++;
      }
      return counts;
    };
    allocations.created = () => created;
    const onDestroy = (order, destroy) =>
      function (...args) {
        resources.delete(order);
        return destroy.apply(this, args);
      };
    const observe = (resource, kind) => {
      const order = created++;
      resources.set(order, { reference: new WeakRef(resource), kind });
      resource.destroy = onDestroy(order, resource.destroy);
      return resource;
    };
    const probe = {
      devices: 0,
      contexts: [],
      music: [],
      allocations,
      drain: () => Promise.all(queues.map((queue) => queue.onSubmittedWorkDone())),
    };
    window.__appJourney = probe;
    const request = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = async function (...args) {
      const device = await request.apply(this, args);
      probe.devices++;
      queues.push(device.queue);
      for (const [method, kind] of [
        ["createBuffer", "buffers"],
        ["createTexture", "textures"],
      ]) {
        const create = device[method];
        device[method] = function (...descriptors) {
          return observe(create.apply(this, descriptors), kind);
        };
      }
      return device;
    };
    const NativeContext = AudioContext;
    window.AudioContext = class extends NativeContext {
      constructor(...args) {
        super(...args);
        probe.contexts.push(this);
      }
    };
    // Music streams from media elements: each start's file and position.
    const play = HTMLMediaElement.prototype.play;
    HTMLMediaElement.prototype.play = function (...args) {
      if (this.src.includes("/audio/music/"))
        probe.music.push({ src: this.src, offset: this.currentTime });
      return play.apply(this, args);
    };
  });
  await page.goto(new URL(ctx.url).origin);
  await menuShown(page);
  await page.waitForFunction(() => window.__appJourney.music.length > 0);
  const origin = await page.evaluate(() => performance.timeOrigin);
  // The menu's one battle is its backdrop: up once its veil first lifts.
  await page.waitForFunction(
    () => Number(document.querySelector(".menu-backdrop-veil")?.style.opacity ?? 1) < 1,
    null,
    { timeout: 120000 },
  );
  await snapshot(ctx, page, "app-journey-menu.png");
  const backdrop = page.workers();
  ctx.check(
    "menu streams its music beside one battle worker, its backdrop's",
    backdrop.length === 1,
    `${backdrop.length} workers`,
  );
  const backdropClosed = Promise.all(backdrop.map((worker) => worker.waitForEvent("close")));
  // Ordinary Play on a small open map: preparation admits a fresh seed, and
  // the address it publishes names that exact battle.
  await openMenuPage(page, "Skirmish");
  await page.getByTestId("menu-map-open").click();
  await page.getByTestId("menu-size-small").click();
  await page.getByTestId("menu-deploy").click();
  await backdropClosed;

  const ready = async () => {
    await page.waitForFunction(() => window.__lab?.ready && window.__lab.route?.tick() > 3, null, {
      timeout: 120000,
    });
    await page.evaluate(() => window.__lab.route.pause());
  };
  // One deliberate collection per lifecycle checkpoint. The new backdrop has
  // drawn, and prior queue work has completed; a surviving old wrapper fails.
  const releasedBefore = async (mark) => {
    await menuShown(page);
    await page.waitForFunction(
      () => Number(document.querySelector(".menu-backdrop-veil")?.style.opacity ?? 1) < 1,
      null,
      { timeout: 120000 },
    );
    await page.evaluate(() => window.__appJourney.drain());
    await cdp.send("HeapProfiler.collectGarbage");
    return page.evaluate((before) => window.__appJourney.allocations(before), mark);
  };
  await ready();
  const seedAddress = page.url();
  await advance(page, 90 - (await obs(page)).tick);
  await presented(page);
  const digest = await page.evaluate(() => window.__lab.route.digest());
  await snapshot(ctx, page, "app-journey-play.png");
  const observed = await page.evaluate(() => ({
    created: window.__appJourney.allocations.created(),
    live: window.__appJourney.allocations(),
  }));
  ctx.check(
    "resource lifetime observer sees the live battle's WebGPU buffers and textures",
    observed.created > 0 && observed.live.buffers > 0 && observed.live.textures > 0,
    JSON.stringify(observed),
  );
  // A real skirmish's map grows trees: the battle's presentation is handed
  // the world's scenery art, so its forests are placed and some are drawn,
  // far from the opening camera and near from a low one looking across the
  // ground (straight down at 150 m the camera may see only a field).
  const far = await page.evaluate(() => window.__lab.stats().scenery);
  const opening = await page.evaluate(() => window.__lab.camera());
  await page.evaluate(() => {
    const c = window.__lab.camera();
    window.__lab.setCamera({ ...c, distance: 150, pitch: 0.3 });
  });
  for (let k = 0; k < 30; k++) await page.evaluate(() => window.__lab.frame?.());
  await page.waitForTimeout(1000);
  const near = await page.evaluate(() => window.__lab.stats().scenery);
  await snapshot(ctx, page, "app-journey-trees.png");
  await page.evaluate((c) => window.__lab.setCamera(c), opening);
  ctx.check(
    "a played skirmish places its forests and draws some of them",
    far.forest.placed > 0 &&
      far.forest.tiers.some((n) => n > 0) &&
      near.forest.tiers.some((n) => n > 0),
    JSON.stringify({
      placed: far.forest.placed,
      far: far.forest.tiers,
      near: near.forest.tiers,
      backdrop: far.backdrop.tiers,
      understorey: far.understorey.tiers,
    }),
  );
  const toolbar = page.getByRole("toolbar", { name: "Commands", exact: true });

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
  // A unit of the player's own: bought and placed at blue's entry, the
  // battle readied, and in through the road-edge entry.
  const entry = await page.evaluate(() => window.__lab.route.prepared().start.at);
  await page.evaluate(
    (at) => window.__lab.setCamera({ ...window.__lab.camera(), target: [...at, 0], distance: 120 }),
    entry,
  );
  const view = page.viewportSize();
  const [cx, cy] = [view.width / 2, view.height / 2];
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "VEH", exact: true }).click();
  // A family's variants are its info cards, open while its card is hovered.
  await page.getByRole("button", { name: "M1 Abrams", exact: true }).hover();
  await page.getByRole("button", { name: /^SEP v2 — \d+ credits$/ }).click();
  await page.mouse.move(cx, cy);
  await page.waitForTimeout(100);
  await page.mouse.click(cx, cy);
  await advance(page, 1);
  await page.getByRole("button", { name: "START BATTLE", exact: true }).click();
  await advance(page, 60 * 30);
  const own = (await obs(page)).own[0]?.id;
  await page.evaluate((id) => window.__lab.route.select([id]), own);
  ctx.check(
    "a selected live unit exposes the real command toolbar",
    own !== undefined && (await toolbar.count()) === 1,
  );
  // Positive controls: the same keys must work outside the menu.
  await page.keyboard.press("KeyF");
  await advance(page, 1);
  // The purchase and ready are acknowledged already: wait for the policy's.
  await page.waitForFunction(() =>
    window.__lab.route.acks().some((entry) => entry.order.kind === "set_engagement"),
  );
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
  // A prepared battle's authority is the worker that prepared it.
  const old = page.workers().find((worker) => worker.url().includes("/prepare/worker.ts"));
  const closed = old.waitForEvent("close");
  await openMenu(page);
  await page.getByRole("button", { name: "Restart", exact: true }).click();
  await closed;
  await ready();
  await advance(page, 90 - (await obs(page)).tick);
  await presented(page);
  ctx.check(
    "restart creates a fresh authority with the same exact battle",
    !page.workers().includes(old) &&
      (await page.evaluate(() => window.__lab.route.digest())) === digest,
  );

  await openMenu(page);
  const departed = Promise.all(page.workers().map((worker) => worker.waitForEvent("close")));
  // Everything the battle made comes before this mark; the menu's backdrop,
  // which starts building at once, allocates after it.
  let mark = await page.evaluate(() => window.__appJourney.allocations.created());
  await page.getByRole("link", { name: "Main menu", exact: true }).click();
  await departed;
  const departedAllocations = await releasedBefore(mark);
  ctx.check(
    "main-menu return releases the departed battle's WebGPU resources",
    departedAllocations.buffers === 0 && departedAllocations.textures === 0,
    JSON.stringify(departedAllocations),
  );
  await page.goBack();
  await ready();
  await advance(page, 90 - (await obs(page)).tick);
  ctx.check(
    "history return starts fresh exact inputs",
    page.url() === seedAddress &&
      (await page.evaluate(() => window.__lab.route.digest())) === digest,
  );
  await page.goForward();
  await menuShown(page);
  await page.goBack();
  await ready();
  await openMenu(page);
  await page.getByRole("link", { name: "Watch saved replay", exact: true }).click();
  await ready();
  await advance(page, 90 - (await obs(page)).tick);
  await presented(page);
  ctx.check(
    "saved replay preserves the captured digest and has no command buttons",
    (await page.evaluate(() => window.__lab.route.digest())) === digest &&
      (await toolbar.count()) === 0,
  );
  await snapshot(ctx, page, "app-journey-replay.png");
  await openMenu(page);
  mark = await page.evaluate(() => window.__appJourney.allocations.created());
  await page.getByRole("link", { name: "Main menu", exact: true }).click();
  const released = await releasedBefore(mark);
  const owners = await page.evaluate((allocations) => {
    const p = window.__appJourney;
    return {
      devices: p.devices,
      contexts: p.contexts.length,
      running: p.contexts[0].state,
      // The track id is the served file's name before its content hash.
      music: p.music.map((m) => ({
        track: new URL(m.src).pathname.split("/").pop().split(".")[0],
        offset: m.offset,
      })),
      allocations,
      timeOrigin: performance.timeOrigin,
    };
  }, released);
  // The menu replays its one track from the start on every return; a battle
  // plays its own score (tension and combat tracks), also from the start.
  const [menuTrack] = MUSIC.moods.menu;
  const battleTracks = new Set([...MUSIC.moods.tension, ...MUSIC.moods.combat]);
  const menuStarts = owners.music.filter(({ track }) => track === menuTrack);
  ctx.check(
    "complete player journey retains one GPU/audio bank and releases battle allocations",
    owners.devices === 1 &&
      owners.contexts === 1 &&
      owners.running === "running" &&
      owners.music[0]?.track === menuTrack &&
      owners.music.at(-1)?.track === menuTrack &&
      menuStarts.length >= 2 &&
      owners.music.some(({ track }) => battleTracks.has(track)) &&
      owners.music.every(({ track }) => track === menuTrack || battleTracks.has(track)) &&
      owners.music.every(({ offset }) => offset === 0) &&
      owners.allocations.buffers === 0 &&
      owners.allocations.textures === 0 &&
      owners.timeOrigin === origin,
    JSON.stringify(owners),
  );
  await ctx.writeEvidence("app-journey.json", { digest, owners });
  await cdp.detach();
  await page.close();
}

/** Matched battle-view captures on the street test map's battle, plus real
 *  card selection above compact commands. */
export async function armyJourney(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1600, height: 900 } });
  await page.goto(`${new URL(ctx.url).origin}/labs/street?seed=1`);
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
  await pointerOffCanvas(page);
  await snapshot(ctx, page, "army-player-single.png");
  const own = (await obs(page)).own;
  const roster = page.locator(".hud-army-card");
  ctx.check(
    "player army cards contain all own units, including unselected units",
    (await roster.count()) === own.length &&
      (await page.locator('.hud-army-card[aria-pressed="true"]').count()) === 1,
  );
  const armyDigest = await page.evaluate(() => window.__lab.route.digest());
  // The street battle's digest at tick 90. It moves only with the street map
  // (here, its three corner shops), the rules or what the digest folds.
  ctx.check(
    "matched army capture preserves the original battle digest",
    armyDigest === "fdd604ec48fc8b26",
    armyDigest,
  );
  await roster.first().click();
  await roster.nth(1).click({ modifiers: ["Shift"] });
  const selection = await page.evaluate(() => window.__lab.route.selected());
  ctx.check(
    "real card click and Shift-click select exactly the two card owners",
    selection.length === 2 && selection.includes(own[0].id) && selection.includes(own[1].id),
    JSON.stringify(selection),
  );
  await pointerOffCanvas(page);
  await snapshot(ctx, page, "army-player-multiple.png");
  await roster.nth(1).hover();
  ctx.check(
    "real hovered army card owns the complete fact panel",
    (await page.getByRole("tooltip").getAttribute("data-unit")) === String(own[1].id),
  );
  await snapshot(ctx, page, "army-player-hover.png");
  await pointerOffCanvas(page);
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
  const rifle = own.find((unit) => unit.kind === "test_rifle");
  const truck = own.find((unit) => unit.kind === "test_supply");
  const goal = [truck.position[0] + 50, truck.position[1]];
  await page.evaluate((ids) => window.__lab.route.select(ids), [rifle.id, truck.id]);
  await page.keyboard.press("c");
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
