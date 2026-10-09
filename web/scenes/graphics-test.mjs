// Browser graphics-test smoke: opens the production Settings route, starts the
// full menu reel, and proves the route owns a cancellable run. The long
// displayed-frame comparison remains an explicit user action/evidence run.
export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1280, height: 800 } });
  await ctx.openLab(page, `${ctx.url}/graphics-test`, 120000);
  await page.getByRole("button", { name: "Run graphics test" }).click({ timeout: 300000 });
  await page.locator(".graphics-test-progress").waitFor({ timeout: 30000 });
  const cancel = page.locator(".graphics-test-progress").getByRole("button", { name: "Cancel" });
  ctx.check("graphics test exposes a cancellable progress surface", await cancel.count() === 1, "cancel button");
  await cancel.click();
  await page.waitForFunction(() => window.__graphicsTest?.stage === "results", undefined, { timeout: 30000 });
  const outcome = await page.evaluate(() => window.__graphicsTest?.report?.outcome);
  ctx.check("cancelling the graphics test produces a partial report", outcome?.status === "cancelled", JSON.stringify(outcome));
}
