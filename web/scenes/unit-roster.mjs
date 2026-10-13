// The player's production faction skirmish, through its ordinary picker and cursor.
import assert from "node:assert/strict";
import { modelGhostAgreement } from "./_modelGhost.mjs";
import { advance, obs, presented, snapshot } from "./_lab.mjs";
export async function run(ctx) {
  await modelGhostAgreement(ctx);
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
  await page.evaluate(() => window.__lab.route.pause());
  await presented(page);
  const initial = await obs(page);
  assert(
    initial.skirmish?.phase === "preparation" && initial.own.length === 0,
    "skirmish starts in empty preparation",
  );
  assert(
    initial.skirmish.credits === 1000 && initial.skirmish.occupiedSlots === 0,
    "starting wallet and cap are observed",
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
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "VEH", exact: true }).click();
  await snapshot(ctx, page, "vehicle-families.png");
  // Hovering a family opens its variants' info cards standing on its card,
  // 3 px above it, over the picker; clicking the card would buy at once.
  const abrams = page.getByRole("button", { name: "M1 Abrams", exact: true });
  await abrams.hover();
  const card = await abrams.boundingBox();
  const menu = await page.locator(".hud-purchase-stack").boundingBox();
  assert(
    menu !== null &&
      Math.abs(card.y - (menu.y + menu.height) - 3) < 0.5 &&
      Math.abs(menu.x - card.x) < 0.5,
    `the variant menu stands 3 px above its card: ${JSON.stringify({ card, menu })}`,
  );
  await snapshot(ctx, page, "abrams-variants.png");
  const variant = page.getByRole("button", { name: /^SEP v2 — \d+ credits$/ });
  await variant.click();
  await page.mouse.move(640, 340);
  await page.waitForFunction(() => {
    const ghost = window.__lab.route.purchaseGhost?.();
    return ghost && ghost.destination && ghost.variant;
  });
  const previewGhost = await page.evaluate(() => window.__lab.route.purchaseGhost());
  assert(previewGhost?.variant, "selecting a reinforcement creates a cursor ghost");
  const cameraWithGhost = await page.evaluate(() => window.__lab.camera());
  assert.deepEqual(
    cameraWithGhost,
    cameraBeforePurchase,
    "showing a reinforcement ghost keeps the zoomed camera",
  );
  await page.waitForTimeout(100);
  await snapshot(ctx, page, "purchase-ghost.png");
  await ctx.matchBaseline(page, "purchase-ghost");
  await page.keyboard.press("Escape");
  assert((await obs(page)).skirmish.credits === 1000, "free placement and Escape spend no credits");
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  // The picker reopens on its first tab; pointing at a tab switches to it.
  await page.getByRole("tab", { name: "VEH", exact: true }).hover();
  await page.getByRole("button", { name: "M1 Abrams", exact: true }).hover();
  await page.getByRole("button", { name: /^SEP v2 — \d+ credits$/ }).click();
  await page.mouse.move(640, 340);
  await page.waitForTimeout(100);
  const cameraAtPlacement = await page.evaluate(() => window.__lab.camera());
  await page.mouse.click(640, 340);
  await advance(page, 1);
  await page.waitForFunction(() => window.__lab.route.observation().skirmish.pending.length === 1);
  assert(
    !(await page.getByTestId("loading").isVisible()),
    "placing a reinforcement does not return to the Deploying loading screen",
  );
  const cameraAfterPlacement = await page.evaluate(() => window.__lab.camera());
  assert.deepEqual(
    cameraAfterPlacement,
    cameraAtPlacement,
    "confirming a reinforcement keeps the player's camera and viewport mounted",
  );
  const reserved = await obs(page);
  assert(
    reserved.own.length === 0 &&
      reserved.skirmish.credits === 650 &&
      reserved.skirmish.occupiedSlots === 1,
    "cursor confirmation atomically reserves one variant during preparation",
  );
  // A held left drag places at its press and turns the unit toward its
  // release. This is the second gesture the player uses in the same journey.
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "VEH", exact: true }).hover();
  await page.getByRole("button", { name: "M1 Abrams", exact: true }).hover();
  await page.getByRole("button", { name: /^SEP v2 — \d+ credits$/ }).click();
  const dragFrom = [560, 360];
  const dragTo = [700, 360];
  await page.mouse.move(...dragFrom);
  await page.mouse.down();
  const dragPressGhost = await page.evaluate(() => window.__lab.route.purchaseGhost());
  await page.mouse.move(...dragTo, { steps: 12 });
  await page.waitForTimeout(100);
  const dragGhost = await page.evaluate(() => window.__lab.route.purchaseGhost());
  assert(
    dragGhost &&
      dragPressGhost &&
      dragGhost.destination.every((v, i) => Math.abs(v - dragPressGhost.destination[i]) < 0.01) &&
      Math.abs(dragGhost.facing - previewGhost.facing) > 0.01,
    "drag placement keeps the press position and updates facing",
  );
  await page.mouse.up();
  await advance(page, 1);
  await page.waitForFunction(() => window.__lab.route.observation().skirmish.pending.length === 2);
  await snapshot(ctx, page, "journey-placed-facing.png");
  await ctx.matchBaseline(page, "journey-placed-facing");
  await page.getByRole("button", { name: "START BATTLE", exact: true }).click();
  await advance(page, 60 * 30);
  const active = await obs(page);
  assert(
    active.own.length >= 1 && active.skirmish.pending.length === 0,
    "reinforcement physically enters through the ordinary road-edge spawn",
  );
  await snapshot(ctx, page, "first-reinforcement.png");
  assert(active.skirmish.phase === "active", "ready/timeout starts the match");
  assert(active.own.length >= 1, "the deployed force enters the active battle");
  const selected = active.own[0];
  await page.evaluate((id) => window.__lab.route.select([id]), selected.id);
  const target = [selected.position[0] + 30, selected.position[1] + 12];
  const targetCss = await page.evaluate(
    ([x, y]) => window.__lab.projectToCss(x, y, window.__lab.route.surfaceZ(x, y)),
    target,
  );
  assert(targetCss, "the selected unit's next destination is on screen");
  await page.mouse.click(targetCss[0], targetCss[1], { button: "right" });
  await page.waitForFunction(() =>
    window.__lab.route.acks().some(({ order, ack }) => order.kind === "move" && !ack.error),
  );
  const moveAck = await page.evaluate(() =>
    window.__lab.route.acks().find(({ order, ack }) => order.kind === "move" && !ack.error),
  );
  assert(moveAck, "a right-click moves the selected unit");
  await advance(page, 60);
  const moved = (await obs()).own.find((unit) => unit.id === selected.id);
  assert(
    moved &&
      Math.hypot(
        moved.position[0] - selected.position[0],
        moved.position[1] - selected.position[1],
      ) > 1,
    "the selected unit advances after its move order",
  );
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
  assert(
    await page.evaluate(() => window.__lab.route.showOrders()),
    "Space reveals the order marker",
  );
  await snapshot(ctx, page, "journey-space-orders.png");
  await ctx.matchBaseline(page, "journey-space-orders");
  await page.keyboard.up("Space");
  await ctx.writeEvidence("match.json", { initial: initial.skirmish, active: active.skirmish });
  await page.close();
}
