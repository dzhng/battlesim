// @vitest-environment node
import { expect, test } from "vitest";
import type { Browser, Page } from "playwright";
import { launchBrowser } from "../browser.mjs";
import { browserTestServer } from "./support/browserServer";

// The shipped entry, links, loading cover and preparation worker. Menu warm-up
// admits the page GPU, but this history proof cancels before drawing a battle.
// The menu stays inert under its loading cover until its backdrop battle is
// ready: about 20 s on a warm dev server, near a minute on a cold one, whose
// first visit transforms every module and serves every asset. So the test
// waits on the cover's progress and fails when it stops advancing (a hang),
// not on a total that a cold or busy machine can pass.
// A first launch then measures the graphics tiers behind the menu (another
// 30 s or so); this is a returning player's, whose detected tier is kept.
test("client exit and history keep the document while discarding preparation visits", async () => {
  const server = await browserTestServer();
  let browser: Browser | undefined;
  try {
    browser = await launchBrowser({ args: ["--mute-audio"] });
    const page = await browser.newPage();
    await page.addInitScript(() =>
      localStorage.setItem(
        "battle.graphics",
        JSON.stringify({
          mode: "auto",
          detectedMode: "high",
          lastResult: { averageFps: 60, onePercentLowFps: 60, selectedMode: "high" },
        }),
      ),
    );
    await page.goto(server.url);
    await coverLifted(page);
    await page.getByRole("button", { name: "Skirmish", exact: true }).click();
    // Finish the lobby's preview worker before listening for the battle worker.
    await page.locator('[data-testid="lobby-map"][data-status="ready"]').waitFor();
    const origin = await page.evaluate(() => performance.timeOrigin);
    const workerOpened = page.waitForEvent("worker");
    await page.getByRole("link", { name: "Deploy" }).click();
    const worker = await workerOpened;
    const closed = worker.waitForEvent("close");
    await page.getByRole("link", { name: "Cancel", exact: true }).click();
    await closed;
    await page.getByRole("heading", { name: "Skirmish", exact: true }).waitFor();
    expect(await page.evaluate(() => performance.timeOrigin)).toBe(origin);
    const freshWorkerOpened = page.waitForEvent("worker");
    await page.goBack();
    const freshWorker = await freshWorkerOpened;
    expect(freshWorker).not.toBe(worker);
    expect(new URL(page.url()).pathname).toBe("/battle");
    const freshClosed = freshWorker.waitForEvent("close");
    await page.goBack();
    await freshClosed;
    await page.getByRole("button", { name: "Skirmish", exact: true }).waitFor();
    const forwardOpened = page.waitForEvent("worker");
    await page.goForward();
    await forwardOpened;
    await page.getByRole("link", { name: "Cancel", exact: true }).click();
    await page.getByRole("heading", { name: "Skirmish", exact: true }).waitFor();
    expect(await page.evaluate(() => performance.timeOrigin)).toBe(origin);
  } finally {
    await browser?.close();
    await server.close();
  }
}, 300_000);

/** The longest the loading cover may show one stage and byte count. */
const STALL_MS = 60_000;

/** Wait until the menu's loading cover is gone and the menu takes input,
 *  failing if the cover's stage and byte count stop changing. */
async function coverLifted(page: Page) {
  const cover = page.getByTestId("loading");
  let shown = "";
  let since = Date.now();
  for (;;) {
    const now = await page.evaluate(() => ({
      covered: document.querySelector("[data-testid=loading]") !== null,
      inert: document.querySelector("main.menu")?.hasAttribute("inert") ?? true,
      progress: document.querySelector("[data-testid=loading]")?.textContent ?? "",
    }));
    if (!now.covered && !now.inert) return;
    expect(await cover.getByTestId("error").count(), "the menu's backdrop failed").toBe(0);
    if (now.progress !== shown) [shown, since] = [now.progress, Date.now()];
    expect(Date.now() - since, `loading stalled at "${shown}"`).toBeLessThan(STALL_MS);
    await page.waitForTimeout(250);
  }
}
