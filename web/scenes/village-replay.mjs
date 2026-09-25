// Slice 15: a saved village battle replays to the same digests with input
// and the defender off; a replay for the other variant is refused clearly.
import { lab, advance } from "./_lab.mjs";

const ticks = (page) => lab(page, () => window.__lab.route.tick());

export async function run(ctx) {
  const page = await ctx.newPage();
  const battle = ctx.url.replace("/replay/village", "/battle/village");
  // Play: the tanks push, the defender answers; save at a known tick.
  await ctx.openLab(page, battle);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  const o = await lab(page, () => window.__lab.route.observation());
  const tanks = o.own.filter((u) => u.kind === "tank").map((u) => u.id);
  await lab(page, (ids) => window.__lab.route.select(ids), tanks);
  await page.waitForFunction((n) => window.__lab.route.selected().length === n, tanks.length);
  await page.keyboard.press("a");
  const spot = await lab(page, () => window.__lab.projectToCss(900, 800, 0));
  await page.mouse.click(spot[0], spot[1], { button: "right" });
  await page.waitForFunction(() => window.__lab.route.acks().length > 0);
  await advance(page, 30 * 40);
  const end = await ticks(page);
  const probes = [end - 600, end - 1, end];
  const want = await lab(page, (ts) => ts.map((t) => window.__lab.route.digest(t)), probes);
  const file = await lab(page, () => window.__lab.route.exportReplay());

  // Replay the saved file in the viewer.
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, end - (await ticks(page)));
  const got = await lab(page, (ts) => ts.map((t) => window.__lab.route.digest(t)), probes);
  ctx.check(
    "the replay reaches the same digests as the played battle",
    want.every((d) => d) && want.join() === got.join(),
    JSON.stringify({ probes, want, got }),
  );
  ctx.check(
    "the replay shows the encounter status",
    /s held/.test(await page.getByTestId("encounter").innerText()),
  );
  await lab(page, (ids) => window.__lab.route.select(ids), tanks);
  await page.mouse.click(spot[0], spot[1], { button: "right" });
  await page.keyboard.press("s");
  ctx.check(
    "input is off while replaying",
    (await lab(page, () => window.__lab.route.acks())).length === 0 &&
      (await page.getByTestId("ack-log").count()) === 0,
  );
  await page.evaluate(() => window.__lab.frame());
  await page.screenshot({ path: ctx.evidencePath("frame-replay-1280x800.png") });

  // The same commands on the other variant's scenario: refused, clearly.
  const wrong = JSON.stringify({ ...file, variant: "prepared_crossfire" });
  await page.getByTestId("replay-file").setInputFiles({
    name: "wrong.json",
    mimeType: "application/json",
    buffer: Buffer.from(wrong),
  });
  await page.waitForSelector("[data-testid=error]", { timeout: 30000 });
  const error = await page.getByTestId("error").innerText();
  ctx.check(
    "a replay for another scenario is refused",
    /does not match this scenario/.test(error),
    error,
  );
}
