// Slice 15: the village battle. Blue plays through the production controls;
// the encounter status, variant, seed, pause/reset and replay export work.
import { lab, obs, advance, snapshot } from "./_lab.mjs";

const text = (page, id) => page.getByTestId(id).innerText();

async function shot(ctx, page, name) {
  await snapshot(ctx, page, `frame-${name}.png`);
}

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await shot(ctx, page, "start-1280x800");
  ctx.check(
    "the status names the variant and seed",
    /Ordinary ambush · seed \d+/.test(await text(page, "status")),
    await text(page, "status"),
  );
  ctx.check(
    "the encounter status is published and running",
    /^Hold the village: 0\/\d+ s held · in progress$/.test(await text(page, "encounter")),
    await text(page, "encounter"),
  );

  // Select the tanks and right-click the ground: an accepted move.
  const o = await obs(page);
  const tanks = o.own.filter((u) => u.kind === "tank").map((u) => u.id);
  await lab(page, (ids) => window.__lab.route.select(ids), tanks);
  await page.waitForFunction((n) => window.__lab.route.selected().length === n, tanks.length);
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [360, 800, 0], distance: 600 }),
  );
  const spot = await lab(page, () => window.__lab.projectToCss(420, 800, 0));
  await page.mouse.click(spot[0], spot[1], { button: "right" });
  await page.waitForFunction(() => window.__lab.route.acks().length > 0, undefined, {
    timeout: 5000,
  });
  const ack = (await lab(page, () => window.__lab.route.acks()))[0];
  ctx.check(
    "right-clicking the ground moves the selected tanks",
    /move/.test(ack.label) && ack.ack.error === null,
    JSON.stringify(ack),
  );
  await advance(page, 30 * 20);
  const moved = await obs(page);
  await shot(ctx, page, "advance-1280x800");
  ctx.check(
    "the tanks drove toward the order",
    moved.own
      .filter((u) => tanks.includes(u.id))
      .every((u) => Math.hypot(u.position[0] - 420, u.position[1] - 800) < 150),
    JSON.stringify(moved.own.filter((u) => tanks.includes(u.id)).map((u) => u.position)),
  );
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [420, 800, 0], distance: 350 }),
  );
  await shot(ctx, page, "tanks-close-1280x800");

  // Export: the file names its variant and carries the accepted commands.
  const file = await lab(page, () => window.__lab.route.exportReplay());
  ctx.check(
    "the replay export records its variant and commands",
    file?.variant === "ordinary" && JSON.parse(file.replay).accepted?.length >= 1,
    file && file.replay.slice(0, 120),
  );

  // Pause holds the tick; resume continues.
  const held = await lab(page, () => window.__lab.route.tick());
  await page.waitForTimeout(400);
  ctx.check(
    "paused, the battle does not advance",
    (await lab(page, () => window.__lab.route.tick())) === held,
  );

  // Reset starts again from the seed with an empty log.
  await page.getByRole("button", { name: "Reset" }).click();
  await page.waitForFunction(
    () => window.__lab.route.acks().length === 0 && window.__lab.route.tick() < 60,
  );
  ctx.check("reset rebuilds from the seed", true);

  // The seed and the variant are the player's to change.
  await page.getByLabel("Seed").fill("7");
  await page.waitForFunction(
    () => /seed 7 /.test(document.querySelector("[data-testid=status]")?.textContent ?? ""),
    undefined,
    { timeout: 30000 },
  );
  ctx.check("a new seed restarts the battle on that seed", true);
  await page.getByLabel("Variant").selectOption("prepared_crossfire");
  await page.waitForFunction(
    () =>
      /Prepared crossfire/.test(document.querySelector("[data-testid=status]")?.textContent ?? ""),
    undefined,
    { timeout: 30000 },
  );
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  ctx.check("the variant select loads the crossfire battle", true);
}
