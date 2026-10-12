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
  await page.mouse.click(600, 600);
  ctx.check(
    "placing on the canvas adds an authored unit",
    (await page.locator(".builder-unit").count()) >= 3,
  );
  ctx.check(
    "selected-unit inspector exposes side and facing",
    (await page.getByRole("region", { name: "Selected unit inspector" }).isVisible()),
  );
  await page.getByRole("button", { name: "Add human slot" }).click();
  const newSlot = page.getByRole("button", { name: /Place unit for blue-2/ });
  await newSlot.click();
  await page.mouse.click(420, 580);
  ctx.check("same-side units can be assigned to distinct player slots", (await page.locator(".builder-unit").count()) >= 4);
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download" }).click();
  const artifact = await download;
  await artifact.saveAs(ctx.evidencePath("scenario.json"));
  ctx.check("download exports a self-contained scenario package", (await artifact.failure()) === null);
  await writeFile(ctx.evidencePath("overview.png"), await page.screenshot());
}
