// Narrow viewport containment only; generation/editor contracts belong to vitest.
export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 600, height: 900 } });
  await page.goto(ctx.url);
  await page.locator(".mw-plan svg").waitFor({ timeout: 120000 });
  await page.locator(".mw-controls").scrollIntoViewIfNeeded();
  ctx.check("narrow controls remain within the viewport", !(await page.evaluate(() => document.querySelector(".map-workbench").scrollWidth > innerWidth)));
  await page.locator(".mw-inspection").scrollIntoViewIfNeeded();
  const plan = await page.locator(".mw-plan").boundingBox();
  ctx.check("narrow plan stays visible", !!plan && plan.x >= 0 && plan.x + plan.width <= 600);
  await page.getByRole("button", { name: "Review save" }).evaluate((node) => node.scrollIntoView({ block: "end" }));
  const review = await page.getByRole("button", { name: "Review save" }).boundingBox();
  ctx.check("narrow save control stays inside the viewport", !!review && review.x >= 0 && review.x + review.width <= 600);
  await page.close();
}
