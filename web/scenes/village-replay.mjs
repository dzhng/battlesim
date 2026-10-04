// Slice 15: a saved village battle replays to the same digests with input
// and the defender off; a replay for the other variant is refused clearly.
import { lab, advance, openMenu } from "./_lab.mjs";

const ticks = (page) => lab(page, () => window.__lab.route.tick());

export async function run(ctx) {
  const page = await ctx.newPage();
  const battle = ctx.url.replace("/replay/village", "/battle/village");
  // Play: the tanks push, the defender answers; save at a known tick.
  await ctx.openLab(page, battle);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(() => window.__lab.route.status().status === "paused");
  const o = await lab(page, () => window.__lab.route.observation());
  const tanks = o.own.filter((u) => u.kind === "tank").map((u) => u.id);
  await lab(page, (ids) => window.__lab.route.select(ids), tanks);
  await page.waitForFunction((n) => window.__lab.route.selected().length === n, tanks.length);
  await page.keyboard.press("r");
  // Out of the close opening framing, so the village is on screen.
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [360, 800, 0], distance: 1150 }),
  );
  const spot = await lab(page, () => window.__lab.projectToCss(900, 800, 0));
  await page.mouse.click(spot[0], spot[1], { button: "right" });
  await page.waitForFunction(() => window.__lab.route.acks().length > 0);
  const end = (await ticks(page)) + 30 * 40;
  const probes = [end - 600, end - 1, end];
  const checkpoints = async () => {
    const digests = [];
    for (const tick of probes) {
      await advance(page, tick - (await ticks(page)));
      digests.push(
        await lab(page, () => ({
          tick: window.__lab.route.tick(),
          digest: window.__lab.route.digest(),
        })),
      );
    }
    return digests;
  };
  const want = await checkpoints();
  const file = await lab(page, () => window.__lab.route.exportReplay());

  // Replay the saved file in the viewer.
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(() => window.__lab.route.status().status === "paused");
  const got = await checkpoints();
  ctx.check(
    "the replay reaches the same digests as the played battle",
    want.every((p, k) => p.tick === probes[k] && p.digest) &&
      got.every((p, k) => p.tick === probes[k] && p.digest === want[k].digest),
    JSON.stringify({ probes, want, got }),
  );
  ctx.check(
    "the replay shows the encounter status",
    /^HOLD \d+\/\d+ s$/.test(await page.getByTestId("encounter").innerText()),
  );
  await lab(page, (ids) => window.__lab.route.select(ids), tanks);
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [360, 800, 0], distance: 1150 }),
  );
  await page.mouse.click(spot[0], spot[1], { button: "right" });
  await page.keyboard.press("Backspace");
  ctx.check(
    "input is off while replaying",
    (await lab(page, () => window.__lab.route.acks())).length === 0 &&
      (await page.getByTestId("ack-log").count()) === 0,
  );
  await page.evaluate(() => window.__lab.frame());
  await page.screenshot({ path: ctx.evidencePath("frame-replay-1280x800.png") });

  // The same commands on the other variant's scenario: refused, clearly.
  const wrong = JSON.stringify({ ...file, variant: "prepared_crossfire" });
  await openMenu(page);
  await page.getByTestId("replay-file").setInputFiles({
    name: "wrong.json",
    mimeType: "application/json",
    buffer: Buffer.from(wrong),
  });
  await page.waitForSelector("[data-testid=error]", { timeout: 30000 });
  await page.getByRole("button", { name: "Details", exact: true }).click();
  const error = await page.getByTestId("error-details").innerText();
  ctx.check(
    "a replay for another scenario is refused",
    /does not match this scenario/.test(error),
    error,
  );
}
