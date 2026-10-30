// @vitest-environment node
import { expect, test } from "vitest";
import type { Browser } from "playwright";
import { launchBrowser } from "../browser.mjs";
import { fileURLToPath } from "node:url";
import { browserTestServer } from "./support/browserServer";

// A battle that failed to start has nothing to play: its refusal is shown
// with the menu, and no battlefield is drawn under it.
test("a rejected replay shows its refusal and draws no battlefield", async () => {
  const server = await browserTestServer();
  let browser: Browser | undefined;
  const failures: string[] = [];
  const responses: Promise<void>[] = [];
  try {
    browser = await launchBrowser({
      args: ["--disable-gpu", "--disable-software-rasterizer"],
    });
    const page = await browser.newPage();
    page.on("requestfailed", (request) => {
      failures.push(`${request.url()}: ${request.failure()?.errorText}`);
    });
    page.on("response", (response) => {
      if (response.status() < 400) return;
      responses.push(
        (async () => {
          const body = await response.text().catch(String);
          failures.push(`${response.status()} ${response.url()}: ${body.slice(0, 4000)}`);
        })(),
      );
    });
    const base = server.url;
    // Seed storage on this origin without booting the menu's GPU warm-up.
    // The rejected battle must reach its own refusal on a GPU-less browser.
    await page.goto(`${base}__test_setup__`);
    await page.evaluate(
      async (root) => {
        const importModule = new Function("url", "return import(url)");
        const { loadWasm } = await importModule(`${root}/web/src/battle/sim/module.ts`);
        const { catalogSet } = await importModule(`${root}/web/src/battle/catalog/sets.ts`);
        const { rememberReplay } = await importModule(`${root}/apps/battle-lab/src/replayFile.tsx`);
        const { loadMap } = await importModule(`${root}/web/src/maps/browser.ts`);
        const wasm = await loadWasm();
        const { rules } = await catalogSet("game");
        // Battles with no units, so the game's catalog admits them; each on
        // its own test map.
        const scenario = async (id: string) =>
          JSON.stringify({ map: (await loadMap(id)).definition, rules, units: [] });
        const map = (await loadMap("street")).definition;
        // A saved battle whose commands were recorded on another scenario.
        const other = new wasm.Battle(await scenario("geometry"), 1);
        try {
          await rememberReplay(
            JSON.stringify({
              battle: {
                scenario: await scenario("street"),
                report: {
                  request: {
                    map_source: {
                      kind: "generated",
                      request: { type: "open", size: "small", seed: "1" },
                    },
                    players: [
                      {
                        id: "blue-1",
                        side: "blue",
                        faction: "us",
                        controller: "human",
                        human_capable: true,
                      },
                      {
                        id: "red-1",
                        side: "red",
                        faction: "eastern",
                        controller: "active_ai",
                        human_capable: false,
                      },
                    ],
                    battle_seed: 1,
                  },
                  size: map.size,
                  extents: { rendered: [0, 0, map.size[0], map.size[1]] },
                  start: { at: [180, 790], yaw: 0 },
                },
              },
              replay: other.replay_json(),
            }),
          );
        } finally {
          other.free();
        }
      },
      `/@fs/${fileURLToPath(new URL("../..", import.meta.url))}`,
    );
    await page.goto(`${base}battle?replay=saved`);
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
    expect(await page.getByRole("heading", { name: "Aborted" }).isVisible()).toBe(true);
    // Why it was refused is said on the screen, not only behind Details.
    expect(await page.getByTestId("error").innerText()).toMatch(
      /Replay does not match this scenario\./,
    );
    await page.getByRole("button", { name: "Details", exact: true }).click();
    expect(await page.getByTestId("error-details").innerText()).toBe(
      "replay does not match this scenario",
    );
    expect(await page.locator("canvas").count()).toBe(0);
    expect(await page.getByTestId("battle-panel").count()).toBe(0);
    expect(
      await page.getByRole("link", { name: "Back to the menu", exact: true }).getAttribute("href"),
    ).toMatch(/^\/(\?|$)/);
    await page.screenshot({
      path: fileURLToPath(new URL("../../throwaway/battle-failure.png", import.meta.url)),
    });
  } catch (error) {
    await Promise.all(responses);
    if (failures.length) throw new Error(failures.join("\n"), { cause: error });
    throw error;
  } finally {
    await browser?.close();
    await server.close();
  }
}, 60000);
