// @vitest-environment node
import { expect, test } from "vitest";
import { chromium, type Browser } from "playwright";
import { createServer } from "vite";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";

// No GPU device or drawing: this tests the real worker ownership boundary.
test("a preparation worker becomes the battle authority and replays its commands", async () => {
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
    await page.goto(server.resolvedUrls!.local[0]);
    const documents = Object.fromEntries(
      ["presets", "templates", "recipes"].map((name, i) => [
        name,
        readFileSync(
          new URL(
            `../../fixtures/${["map-presets", "prototype-building-templates", "encounters"][i]}.json`,
            import.meta.url,
          ),
          "utf8",
        ),
      ]),
    );
    const result = await page.evaluate(
      async ({ documents, rules, root }) => {
        // Keep browser imports outside Vitest's server-side import rewriting.
        const importModule = new Function("url", "return import(url)");
        const { prepareBattle } = await importModule(`${root}/battle/prepare/client.ts`);
        const { createSimClient } = await importModule(`${root}/battle/sim/client.ts`);
        const { loadWasm } = await importModule(`${root}/battle/sim/module.ts`);
        const preparation = prepareBattle(
          {
            type: "prepare",
            request: {
              map_source: { kind: "catalogue", id: "village" },
              recipe_id: "lean",
              encounter_seed: "1",
              battle_seed: 1,
            },
            documents: { ...documents, rules },
          },
          () => {},
        );
        const prepared = await preparation.battle;
        const client = createSimClient({
          scenario: prepared.scenario,
          seed: 1,
          side: "blue",
          transport: "worker",
          connect: prepared.connect,
        });
        let digest = "";
        client.onPublication((p: { digest: string; release(): void }) => {
          digest = p.digest;
          p.release();
        });
        try {
          await client.ready;
          client.pause();
          client.start();
          const tick = await client.advance(3);
          const replay = (await loadWasm()).Battle.from_replay(
            prepared.scenario,
            await client.replay(),
          );
          try {
            for (let i = 0; i < tick; i++) replay.step();
            return { tick, digest, replay: replay.digest() };
          } finally {
            replay.free();
          }
        } finally {
          client.dispose();
          preparation.cancel();
        }
      },
      {
        documents,
        rules: JSON.stringify(GAME_RULES),
        root: `/@fs/${fileURLToPath(new URL("../src", import.meta.url))}`,
      },
    );
    expect(result.tick).toBe(3);
    expect(result.digest).toBe(result.replay);
  } finally {
    await browser?.close();
    await server.close();
  }
}, 60000);
