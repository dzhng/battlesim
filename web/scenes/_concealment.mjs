// A squad in forest loses HIDDEN to a visible spotter or known engagement. Both the
// compact name-line icon and the expanded state row read the authority flag.
import { openBattle, aim, presented, obs, snapshot } from "./_lab.mjs";

export async function concealmentTour(ctx) {
  const page = await openBattle(ctx, {
    url: ctx.url.replace(/\/battle\/village.*$/, "/battle/village/lean"),
    viewport: { width: 1920, height: 1080 },
    tick: 331,
    grass: true,
  });
  try {
    const frame = await obs(page);
    const unit = frame.own
      .filter((u) => u.kind === "test_rifle")
      .sort(
        (a, b) =>
          Math.hypot(a.position[0] - 866, a.position[1] - 956) -
          Math.hypot(b.position[0] - 866, b.position[1] - 956),
      )[0];
    await aim(page, unit.position, { distance: 65, pitch: 0.8, yaw: 0 });
    await presented(page);
    const card = page.locator(`.ro-unit[data-unit="${unit.id}"]`);
    const nameIcon = card.locator('.ro-name [title="Hidden"] svg');
    ctx.check(
      "known enemy engagement removes HIDDEN from the compact card",
      !unit.concealed &&
        (await nameIcon.count()) === 0 &&
        !(await card.locator('[data-state="hidden"]').isVisible()),
    );
    await snapshot(ctx, page, "concealment-compact.png");
    await card.hover();
    await page.waitForTimeout(200);
    await snapshot(ctx, page, "concealment-card.png");
    const hidden = card.locator('[data-state="hidden"]');
    ctx.check(
      "known enemy engagement removes HIDDEN from the expanded card",
      !unit.concealed && (await hidden.count()) === 0,
      JSON.stringify({ id: unit.id, concealed: unit.concealed }),
    );
  } finally {
    await page.close();
  }
}
