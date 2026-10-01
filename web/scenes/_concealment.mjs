// The production card reads the authority's bonus, including when nearby
// enemies could still identify the unit. The compact name/health row stays compact.
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
      .filter((u) => u.kind === "rifle")
      .sort(
        (a, b) =>
          Math.hypot(a.position[0] - 866, a.position[1] - 956) -
          Math.hypot(b.position[0] - 866, b.position[1] - 956),
      )[0];
    await aim(page, unit.position, { distance: 65, pitch: 0.8, yaw: 0 });
    await presented(page);
    const card = page.locator(`.ro-unit[data-unit="${unit.id}"]`);
    await card.hover();
    await page.waitForTimeout(200);
    await snapshot(ctx, page, "concealment-card.png");
    const hidden = card.locator('[data-state="hidden"]');
    ctx.check(
      "forest concealment reaches the own unit's expanded card",
      unit.concealed &&
        (await hidden.count()) === 1 &&
        (await hidden.isVisible()) &&
        (await hidden.textContent()).trim() === "HIDDEN",
      JSON.stringify({ id: unit.id, concealed: unit.concealed }),
    );
  } finally {
    await page.close();
  }
}
