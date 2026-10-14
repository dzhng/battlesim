// The player-facing graphics-test reel smoke: route entry plus one complete
// reel scene, so verify retains the browser-only completion contract.
export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1280, height: 800 } });
  const url = new URL(ctx.url);
  const response = await page.goto(url, { waitUntil: "domcontentloaded", timeout: 30000 });
  ctx.check("graphics-test route responds", response?.ok() === true, `${response?.status() ?? "no response"}`);
  ctx.check(
    "graphics-test route keeps its canonical URL",
    new URL(page.url()).pathname === "/labs/graphics-test",
    page.url(),
  );
  await page.waitForFunction(() => window.__graphicsTest?.stage, undefined, { timeout: 60000 });
  if (await page.evaluate(() => window.__graphicsTest.stage === "refused")) return;
  url.searchParams.set("scenes", "market-town");
  await page.goto(url.href);
  await page.waitForFunction(() => window.__graphicsTest?.stage, undefined, { timeout: 60000 });
  await page.getByRole("button", { name: "Run graphics test" }).click();
  await page.waitForFunction(() => window.__graphicsTest?.stage === "results", undefined, {
    timeout: 240000,
    polling: 500,
  });
  const report = await page.evaluate(() => window.__graphicsTest.report);
  ctx.check(
    "one reel scene plays to completion",
    report.outcome.status === "complete" && report.scenes.length === 1,
    report.outcome.reason,
  );
}
