// The player's production faction skirmish, through its ordinary picker and cursor.
import { modelGhostAgreement } from "./_modelGhost.mjs";
import {
  advance,
  aim,
  obs,
  own,
  openBattle,
  pointerOffCanvas,
  presented,
  snapshot,
  until,
} from "./_lab.mjs";
const check = (ctx, ok, name, detail) => ctx.check(name, ok, detail);
export async function run(ctx) {
  await modelGhostAgreement(ctx);
  await logCards(ctx);
  const page = await ctx.newPage();
  const failures = [];
  page.on("response", async (response) => {
    if (response.status() >= 400)
      failures.push({
        url: response.url(),
        status: response.status(),
        body: (await response.text()).slice(0, 4000),
      });
  });
  try {
    await ctx.openLab(page, ctx.url, 45000);
  } catch (error) {
    await ctx.writeEvidence("startup-failures.json", failures);
    await page.screenshot({ path: ctx.evidencePath("startup.png") });
    throw error;
  }
  // The live FPS readout is diagnostic and changes with asset upload timing;
  // keep journey pictures focused on the player's view.
  await page.addStyleTag({ content: ".frame-rate { display: none !important; }" });
  // The lab is ready before the simulation's first observation arrives.
  await page.waitForFunction(() => window.__lab.route.observation() !== null);
  await page.evaluate(() => window.__lab.route.pause());
  // Pin GPU animation phase as well as simulation state before the journey.
  const captureTick = 60;
  const arrivedTick = (await obs(page)).tick;
  if (arrivedTick > captureTick)
    throw new Error(`roster capture started late at tick ${arrivedTick}`);
  await advance(page, captureTick - arrivedTick);
  await presented(page);
  const initial = await obs(page);
  check(
    ctx,
    initial.skirmish?.phase === "preparation" && initial.own.length === 0,
    "skirmish starts in empty preparation",
  );
  await snapshot(ctx, page, "preparation.png");
  // The player can zoom before opening the roster; the placement path must
  // keep this live camera rather than remounting the viewport on a new model.
  const entry = await page.evaluate(() => window.__lab.route.prepared().start.at);
  await page.evaluate((at) => {
    const camera = window.__lab.camera();
    window.__lab.setCamera({ ...camera, target: [...at, 0], distance: 120 });
  }, entry);
  await page.mouse.move(640, 340);
  await page.mouse.wheel(0, -320);
  await page.waitForTimeout(500);
  const cameraBeforePurchase = await page.evaluate(() => window.__lab.camera());
  await snapshot(ctx, page, "journey-zoom.png");
  await ctx.matchBaseline(page, "journey-zoom");
  // A search naming one variant, then Enter, buys it and arms the ghost: the
  // picker's own keyboard path.
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  const search = page.getByRole("textbox", { name: "Search units" });
  await search.fill("abrams sep v2 trophy");
  check(
    ctx,
    (await page.getByRole("tabpanel", { name: "Search results" }).getByRole("button").count()) ===
      1,
    "a search naming one variant narrows the picker to its one card",
  );
  await search.press("Enter");
  await page.mouse.move(640, 340);
  await page.waitForFunction(
    () => {
      const ghost = window.__lab.route.purchaseGhost?.();
      return ghost && ghost.destination && ghost.variant;
    },
    undefined,
    { timeout: 250 },
  );
  await snapshot(ctx, page, "search-enter-ghost.png");
  await page.keyboard.press("Escape");
  check(ctx, own(await obs(page)).credits === 1000, "Escape cancels the searched purchase free");
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "MBT", exact: true }).click();
  await snapshot(ctx, page, "mbt-cards.png");
  // Hovering a card opens its variant's info card over the picker; clicking
  // it places exactly that variant.
  const abrams = page.getByRole("button", { name: "M1A2 SEP v2", exact: true });
  await abrams.hover();
  await snapshot(ctx, page, "abrams-info.png");
  await abrams.click();
  await page.mouse.move(640, 340);
  await page.waitForFunction(
    () => {
      const ghost = window.__lab.route.purchaseGhost?.();
      return ghost && ghost.destination && ghost.variant;
    },
    undefined,
    { timeout: 250 },
  );
  const previewGhost = await page.evaluate(() => window.__lab.route.purchaseGhost());
  check(ctx, previewGhost?.variant, "selecting a reinforcement creates a cursor ghost");
  check(
    ctx,
    !(await page.getByTestId("loading").isVisible()),
    "selecting a previously unfielded reinforcement does not show a loading cover",
  );
  const cameraWithGhost = await page.evaluate(() => window.__lab.camera());
  ctx.check(
    "showing a reinforcement ghost keeps the zoomed camera",
    JSON.stringify(cameraWithGhost) === JSON.stringify(cameraBeforePurchase),
  );
  await page.waitForTimeout(100);
  await snapshot(ctx, page, "purchase-ghost.png");
  await ctx.matchBaseline(page, "purchase-ghost");
  await page.keyboard.press("Escape");
  check(ctx, own(await obs(page)).credits === 1000, "free placement and Escape spend no credits");
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  // The picker reopens on its first tab; pointing at a tab switches to it.
  await page.getByRole("tab", { name: "MBT", exact: true }).hover();
  await page.getByRole("button", { name: "M1A2 SEP v2", exact: true }).click();
  await page.mouse.move(640, 340);
  await page.waitForTimeout(100);
  const cameraAtPlacement = await page.evaluate(() => window.__lab.camera());
  await page.mouse.click(640, 340);
  await advance(page, 1);
  await page.waitForFunction(
    () =>
      (
        window.__lab.route.observation().skirmish.players.find((p) => p.controller === "human") ??
        window.__lab.route.observation().skirmish.players[0]
      ).pending.length === 1,
  );
  check(
    ctx,
    !(await page.getByTestId("loading").isVisible()),
    "placing a reinforcement does not return to the Deploying loading screen",
  );
  const cameraAfterPlacement = await page.evaluate(() => window.__lab.camera());
  ctx.check(
    "confirming a reinforcement keeps the player's camera and viewport mounted",
    JSON.stringify(cameraAfterPlacement) === JSON.stringify(cameraAtPlacement),
  );
  const reserved = await obs(page);
  check(
    ctx,
    reserved.own.length === 0 && own(reserved).credits === 650 && own(reserved).unitCount === 1,
    "cursor confirmation atomically reserves one variant during preparation",
  );
  // A held left drag places at its press and turns the unit toward its
  // release. This is the second gesture the player uses in the same journey.
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "MBT", exact: true }).hover();
  await page.getByRole("button", { name: "M1A2 SEP v2", exact: true }).click();
  const dragFrom = [560, 360];
  const dragTo = [700, 360];
  await page.mouse.move(...dragFrom);
  await page.mouse.down();
  // Where the press places it, once the authority has answered: a hull is
  // sent to the nearest ground it parks on, as a move there would be.
  await page.waitForFunction(() => window.__lab.route.purchaseGhost()?.valid === true);
  const dragPressGhost = await page.evaluate(() => window.__lab.route.purchaseGhost());
  await page.mouse.move(...dragTo, { steps: 12 });
  await page.waitForTimeout(100);
  const dragGhost = await page.evaluate(() => window.__lab.route.purchaseGhost());
  ctx.check(
    "drag placement keeps the press position and updates facing",
    dragGhost &&
      dragPressGhost &&
      dragGhost.destination.every((v, i) => Math.abs(v - dragPressGhost.destination[i]) < 0.01) &&
      Math.abs(dragGhost.facing - previewGhost.facing) > 0.01,
    JSON.stringify({ press: dragPressGhost?.destination, drag: dragGhost?.destination }),
  );
  await page.mouse.up();
  await advance(page, 1);
  await page.waitForFunction(
    () =>
      (
        window.__lab.route.observation().skirmish.players.find((p) => p.controller === "human") ??
        window.__lab.route.observation().skirmish.players[0]
      ).pending.length === 2,
  );
  await snapshot(ctx, page, "journey-placed-facing.png");
  await ctx.matchBaseline(page, "journey-placed-facing");
  await page.getByRole("button", { name: "START BATTLE", exact: true }).click();
  const active = await until(
    page,
    (o) => o.own.length >= 1 && o.skirmish.phase === "active",
    60 * 60,
    60,
  );
  check(
    ctx,
    active && active.own.length >= 1 && active.skirmish.phase === "active",
    "reinforcement physically enters through the ordinary road-edge spawn",
  );
  await snapshot(ctx, page, "first-reinforcement.png");
  check(ctx, active.skirmish.phase === "active", "ready/timeout starts the match");
  check(ctx, active.own.length >= 1, "the deployed force enters the active battle");
  const selected = active.own[0];
  await page.evaluate((id) => window.__lab.route.select([id]), selected.id);
  const target = [selected.position[0] + 30, selected.position[1] + 12];
  const targetCss = await page.evaluate(
    ([x, y]) => window.__lab.projectToCss(x, y, window.__lab.route.surfaceZ(x, y)),
    target,
  );
  check(ctx, targetCss, "the selected unit's next destination is on screen");
  await page.mouse.click(targetCss[0], targetCss[1], { button: "right" });
  await page.waitForFunction(() =>
    window.__lab.route.acks().some(({ order, ack }) => order.kind === "move" && !ack.error),
  );
  const moveAck = await page.evaluate(() =>
    window.__lab.route.acks().find(({ order, ack }) => order.kind === "move" && !ack.error),
  );
  check(ctx, moveAck, "a right-click moves the selected unit");
  // The approved journey pictures are taken once the move is under way.
  await advance(page, 300);
  const facingFrom = targetCss;
  const facingTo = [facingFrom[0] + 80, facingFrom[1]];
  await page.mouse.move(...facingFrom);
  await page.mouse.down({ button: "right" });
  await page.mouse.move(...facingTo, { steps: 12 });
  await page.mouse.up({ button: "right" });
  await page.waitForFunction(() =>
    window.__lab.route
      .acks()
      .some(({ order, ack }) => order.kind === "move" && order.facing !== undefined && !ack.error),
  );
  await page.keyboard.down("Space");
  await page.waitForTimeout(100);
  check(
    ctx,
    await page.evaluate(() => window.__lab.route.showOrders()),
    "Space reveals the order marker",
  );
  await snapshot(ctx, page, "journey-space-orders.png");
  await ctx.matchBaseline(page, "journey-space-orders");
  await page.keyboard.up("Space");
  await transportJourney(ctx, page, entry);
  await ctx.writeEvidence("match.json", { initial: initial.skirmish, active: active.skirmish });
  await page.close();
}

/** A squad bought with its ride mid-battle: hovering the squad's card opens
 *  its transport menu, and clicking a carrier there places the pair. The
 *  carrier drives to the placed spot with the squad aboard, and the squad
 *  stays aboard once it has arrived. */
async function transportJourney(ctx, page, entry) {
  await page.evaluate(
    (at) => window.__lab.setCamera({ ...window.__lab.camera(), target: [...at, 0], distance: 120 }),
    entry,
  );
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "INF", exact: true }).hover();
  await page.getByRole("button", { name: "Rifle Squad", exact: true }).hover();
  const ride = page
    .getByRole("region", { name: "Transport", exact: true })
    .getByRole("button", { name: "Stryker ICV", exact: true });
  await ride.hover();
  check(
    ctx,
    (await page.getByRole("tooltip", { name: "Stryker ICV", exact: true }).isVisible()) &&
      (await page.getByRole("tooltip", { name: "Rifle Squad", exact: true }).isVisible()),
    "pointing at a ride shows both units it buys",
  );
  await snapshot(ctx, page, "transport-menu.png");
  await ride.click();
  // Up the road from the entry, so the carrier drives in to get there.
  const placeAt = [640, 190];
  await page.mouse.move(...placeAt);
  // Clicked as soon as its ghost is under the cursor, not once its preview
  // has answered: the click still buys, where the authority sends it.
  await page.waitForFunction(() => window.__lab.route.purchaseGhost?.()?.carrier);
  await page.mouse.click(...placeAt);
  const bought = () =>
    page.evaluate(
      () =>
        window.__lab.route
          .acks()
          .find(
            ({ order, ack }) => order.kind === "confirm_purchase" && order.carrier && !ack.error,
          )?.order ?? null,
    );
  let purchase = null;
  for (let step = 0; step < 60 && !purchase; step++) {
    await advance(page, 1);
    purchase = await bought();
  }
  check(ctx, purchase, "a purchase clicked before its preview answers is bought");
  if (!purchase) return;
  // Where it was placed, a fading ghost of the carrier says it went through.
  await presented(page);
  await snapshot(ctx, page, "journey-bought-ghost.png");
  // The carrier the player placed, and the squad it holds.
  const pair = (o) => {
    const carrier = o.own.find((u) => u.kind === purchase.carrier);
    const squad = carrier && o.own.find((u) => u.hold?.host.id === carrier.id);
    return { carrier, squad };
  };
  const there = (u) =>
    Math.hypot(u.position[0] - purchase.destination[0], u.position[1] - purchase.destination[1]) <
    6;
  const arrived = await until(
    page,
    (o) => {
      const { carrier, squad } = pair(o);
      return squad?.kind === purchase.variant && carrier.goal === null && there(carrier);
    },
    60 * 60,
    30,
  );
  check(
    ctx,
    arrived &&
      Math.hypot(purchase.destination[0] - entry[0], purchase.destination[1] - entry[1]) > 20,
    "a squad bought with its ride arrives aboard it at the placed spot, in from the entry",
    JSON.stringify({ entry, destination: purchase.destination }),
  );
  // Arriving is not unloading: a while later the squad is still aboard.
  await advance(page, 120);
  const { carrier, squad } = pair(await obs(page));
  check(
    ctx,
    carrier && squad && there(carrier) && squad.hold.host.kind === "carrier",
    "the squad stays aboard its carrier once it has arrived",
  );
  check(
    ctx,
    squad && (await page.locator(`[data-unit="${squad.id}"][data-aboard]`).count()) === 1,
    "the squad's army card carries the aboard mark",
  );
  // The carrier where it was sent, and the deck: its card, the squad's
  // marked aboard beside it.
  await aim(page, carrier.position, { distance: 70 });
  await page.evaluate((id) => window.__lab.route.select([id]), carrier.id);
  await pointerOffCanvas(page);
  await presented(page);
  await snapshot(ctx, page, "journey-transport-arrived.png");
  await ctx.matchBaseline(page, "journey-transport-arrived");
}

/** Capture the LOG interactions in their own preparation, so adding picker
 *  shots cannot change the presentation history of the existing journey. */
async function logCards(ctx) {
  const page = await openBattle(ctx, { tick: 60 });
  await page.addStyleTag({ content: ".frame-rate { display: none !important; }" });
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "LOG", exact: true }).click();
  await pointerOffCanvas(page);
  await snapshot(ctx, page, "log-cards.png");
  await ctx.matchBaseline(page.locator(".hud-purchase-picker"), "log-cards");
  await page.getByRole("tabpanel", { name: "LOG", exact: true }).getByRole("button").nth(1).hover();
  await snapshot(ctx, page, "log-supply-info.png");
  await ctx.matchBaseline(page.locator(".hud-purchase-info"), "log-supply-info");
  await pointerOffCanvas(page);
  await page.close();
}
