// @vitest-environment node
import { expect, test } from "vitest";
import { chromium, type Browser } from "playwright";
import { createServer } from "vite";
import { fileURLToPath } from "node:url";

// The shipped entry, links, loading cover and preparation worker. Menu warm-up
// admits the page GPU, but this history proof cancels before drawing a battle.
test("client exit and history keep the document while discarding preparation visits", async () => {
  const server = await createServer({
    configFile: fileURLToPath(new URL("../vite.config.ts", import.meta.url)),
    configLoader: "runner",
    server: { port: 0, host: "127.0.0.1" },
  });
  let browser: Browser | undefined;
  try {
    await server.listen();
    browser = await chromium.launch({
      channel: "chromium",
      args: ["--enable-unsafe-webgpu", "--enable-features=WebGPU", "--mute-audio"],
    });
    const page = await browser.newPage();
    await page.goto(server.resolvedUrls!.local[0]);
    await page.getByTestId("menu-deploy").waitFor();
    const origin = await page.evaluate(() => performance.timeOrigin);
    const workerOpened = page.waitForEvent("worker");
    await page.getByRole("link", { name: "Play Market Town" }).click();
    const worker = await workerOpened;
    const closed = worker.waitForEvent("close");
    await page.getByRole("link", { name: "Cancel", exact: true }).click();
    await closed;
    await page.getByRole("heading", { name: "Battle", exact: true }).waitFor();
    expect(await page.evaluate(() => performance.timeOrigin)).toBe(origin);
    const freshWorkerOpened = page.waitForEvent("worker");
    await page.goBack();
    const freshWorker = await freshWorkerOpened;
    expect(freshWorker).not.toBe(worker);
    expect(new URL(page.url()).pathname).toBe("/battle");
    const freshClosed = freshWorker.waitForEvent("close");
    await page.goBack();
    await freshClosed;
    await page.getByRole("heading", { name: "Battle", exact: true }).waitFor();
    const forwardOpened = page.waitForEvent("worker");
    await page.goForward();
    await forwardOpened;
    await page.getByRole("link", { name: "Cancel", exact: true }).click();
    await page.getByRole("heading", { name: "Battle", exact: true }).waitFor();
    expect(await page.evaluate(() => performance.timeOrigin)).toBe(origin);
  } finally {
    await browser?.close();
    await server.close();
  }
}, 30000);
