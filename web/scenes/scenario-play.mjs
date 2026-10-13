import { writeFile } from "node:fs/promises";
import { access } from "node:fs/promises";

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(ctx.url);
  await page.getByRole("heading", { name: "Play scenario" }).waitFor();
  ctx.check(
    "upload route exposes an explicit scenario file boundary",
    (await page.getByText(/Upload a downloaded scenario package/).isVisible()) &&
      (await page.getByText("Upload scenario").isVisible()),
  );
  const packagePath = ctx.evidencePath("../scenario-builder/scenario.json");
  try {
    await access(packagePath);
    await page.locator('input[type="file"]').setInputFiles(packagePath);
    await page.getByRole("heading", { name: "Choose controller" }).waitFor();
    await page.getByRole("button", { name: "Deploy scenario" }).click();
    await page.getByTestId("loading").waitFor({ state: "visible", timeout: 10000 });
    await page.waitForFunction(
      () =>
        !!document.querySelector('[data-testid="error"]') ||
        (!!document.querySelector('[data-testid="battle-panel"]') &&
          !document.querySelector('[data-testid="loading"]')),
      undefined,
      { timeout: 60000 },
    );
    if (await page.getByTestId("error").isVisible())
      throw new Error(await page.getByTestId("error").innerText());
    ctx.check("downloaded package reaches the normal battle panel", true);
  } catch (error) {
    const details = page.getByRole("button", { name: "Details", exact: true });
    if (await details.isVisible()) await details.click();
    await ctx.writeEvidence("startup.json", {
      error: String(error),
      text: await page.locator("body").innerText(),
    });
    ctx.check("downloaded package reaches the normal battle panel", false, String(error));
  }
  await writeFile(ctx.evidencePath("overview.png"), await page.screenshot());
}
