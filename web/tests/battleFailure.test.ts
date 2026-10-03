// @vitest-environment node
import { expect, test } from "vitest";
import { chromium, type Browser } from "playwright";
import { createServer } from "vite";
import { fileURLToPath } from "node:url";

// A battle that failed to start has nothing to play: its refusal is shown
// with the menu, and no battlefield is drawn under it.
test("a rejected replay shows its refusal and draws no battlefield", async () => {
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
      args: ["--disable-gpu", "--disable-software-rasterizer"],
    });
    const page = await browser.newPage();
    const base = server.resolvedUrls!.local[0];
    await page.goto(base);
    await page.evaluate(
      async (root) => {
        const importModule = new Function("url", "return import(url)");
        const { loadWasm } = await importModule(`${root}/web/src/battle/sim/module.ts`);
        const { villageScenario } = await importModule(`${root}/apps/battle-lab/src/savedMaps.tsx`);
        const { rememberReplay } = await importModule(`${root}/apps/battle-lab/src/replayFile.tsx`);
        const wasm = await loadWasm();
        const battle = new wasm.Battle(await villageScenario(wasm, "village", "ordinary"), 1);
        try {
          await rememberReplay(
            JSON.stringify({
              variant: "prepared_crossfire",
              replay: battle.replay_json(),
            }),
          );
        } finally {
          battle.free();
        }
      },
      `/@fs/${fileURLToPath(new URL("../..", import.meta.url))}`,
    );
    await page.goto(`${base}replay/village`);
    try {
      await page.getByTestId("error").waitFor({ timeout: 10000 });
    } catch (error) {
      await page.screenshot({
        path: fileURLToPath(
          new URL("../../throwaway/battle-failure-rejected.png", import.meta.url),
        ),
      });
      throw error;
    }
    expect(await page.getByTestId("error").innerText()).toBe("replay does not match this scenario");
    expect(await page.locator("canvas").count()).toBe(0);
    expect(await page.getByTestId("battle-panel").innerText()).not.toContain("0:00");
    await page.getByRole("button", { name: "Menu", exact: true }).click();
    expect(await page.getByRole("link", { name: "Main menu", exact: true }).isVisible()).toBe(true);
    await page.getByRole("button", { name: "Resume", exact: true }).click();
    await page.screenshot({
      path: fileURLToPath(new URL("../../throwaway/battle-failure.png", import.meta.url)),
    });
  } finally {
    await browser?.close();
    await server.close();
  }
}, 60000);
