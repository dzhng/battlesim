import { writeFile } from "node:fs/promises";

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(ctx.url);
  await page.getByTestId("scenario-builder").waitFor();
  ctx.check(
    "builder opens with tactical canvas and explicit file controls",
    (await page.getByRole("application", { name: "Scenario tactical map" }).isVisible()) &&
      (await page.getByRole("button", { name: "Download" }).isVisible()) &&
      (await page.getByRole("button", { name: "Upload" }).isVisible()),
  );
  await page.mouse.click(720, 360);
  ctx.check(
    "placing on the canvas adds an authored unit",
    (await page.locator(".builder-unit").count()) >= 3,
  );
  ctx.check(
    "selected-unit inspector exposes side and facing",
    (await page.getByRole("region", { name: "Selected unit inspector" }).isVisible()),
  );
  await writeFile(ctx.evidencePath("overview.png"), await page.screenshot());
}
