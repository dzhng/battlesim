// The probe that timed each growth in the reference battle (the ledger has its
// results). It is not a registered scene. To run it again, copy it to
// web/scenes/growth-probe.mjs and add a fixture with the same id whose route is
// the page link in freeze-report-2026-11-14.json ("tool": true, "serial": true,
// "catalog": "game", "loading": false, a long timeout), run it at high
// graphics, then remove both: neither is to be committed.
import { advance, lab } from "./_lab.mjs";

export async function run(ctx) {
  const page = await ctx.newPage();
  const events = [];
  page.on("console", (message) => {
    const m = /^\[battle (\d+) ms\] (battle staged|art committed|scene rebuild|long frame|scene installed)\b/.exec(
      message.text(),
    );
    if (!m) return;
    const event = { at: Number(m[1]), kind: m[2], detail: null };
    events.push(event);
    message.args()[1]?.jsonValue().then((d) => (event.detail = d)).catch(() => {});
  });
  await ctx.openLab(page, ctx.url, 300000);
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(() => window.__lab?.route?.status().status === "paused");
  const start = page.getByRole("button", { name: "START BATTLE", exact: true });
  if (await start.count()) await start.click().catch(() => {});
  const staged = () => events.filter((e) => e.kind === "battle staged").length;
  const committedAfterLastStage = () => {
    const i = events.map((e) => e.kind).lastIndexOf("battle staged");
    return i < 0 || events.slice(i).some((e) => e.kind === "art committed");
  };
  const began = Date.now();
  for (let round = 0; round < 40 && staged() < 6 && Date.now() - began < 25 * 60_000; round++) {
    await advance(page, 60 * 10);
    const waitFrom = Date.now();
    while (!committedAfterLastStage() && Date.now() - waitFrom < 120_000)
      await page.waitForTimeout(250);
    console.log("PROBE round", round, "staged", staged(), "events", events.length);
  }
  await page.waitForTimeout(1000);
  for (const e of events) console.log("PROBE", JSON.stringify(e));
  ctx.check("probe ran", true, "");
  await page.close();
}
