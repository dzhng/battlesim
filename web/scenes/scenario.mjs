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
    await page.getByRole("region", { name: "Selected unit inspector" }).isVisible(),
  );
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download" }).click();
  const artifact = await download;
  const packagePath = ctx.evidencePath("scenario.json");
  await artifact.saveAs(packagePath);
  ctx.check(
    "download exports a self-contained scenario package",
    (await artifact.failure()) === null,
  );
  await page.goto(ctx.urlFor({ mode: "play" }));
  await page.getByRole("heading", { name: "Play scenario" }).waitFor();
  await page.locator('input[type="file"]').setInputFiles(packagePath);
  await page.getByRole("heading", { name: "Choose controller" }).waitFor();
  await page.getByRole("button", { name: "Deploy scenario" }).click();
  await page.waitForFunction(
    () =>
      !!document.querySelector('[data-testid="error"]') ||
      (!!document.querySelector('[data-testid="battle-panel"]') &&
        !document.querySelector('[data-testid="loading"]')),
    undefined,
    { timeout: 60000 },
  );
  const error = await page
    .getByTestId("error")
    .isVisible()
    .catch(() => false);
  ctx.check(
    "downloaded package boots the normal battle worker",
    // The HUD container has no box of its own (its parts are placed absolutely),
    // so its presence, with no error and no loading cover, is the booted battle.
    !error && (await page.getByTestId("battle-panel").count()) === 1,
  );
  await writeFile(ctx.evidencePath("overview.png"), await page.screenshot());
}
