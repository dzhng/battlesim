// Fast route smoke for the graphics-test entry point. The full production run
// is intentionally a separate, user-triggered measurement because preparing
// the menu worlds can saturate a software/headless GPU for minutes.
export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1280, height: 800 } });
  const response = await page.goto(ctx.url, { waitUntil: "domcontentloaded", timeout: 30000 });
  ctx.check("graphics-test route responds", response?.ok() === true, `${response?.status() ?? "no response"}`);
  ctx.check("graphics-test route keeps its canonical URL", new URL(page.url()).pathname === "/graphics-test", page.url());
}
