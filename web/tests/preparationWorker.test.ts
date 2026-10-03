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
        const { rememberReplay, readSavedReplay, ReplayImport } = await importModule(
          `${root}/../../apps/battle-lab/src/replayFile.tsx`,
        );
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
          const command = client.command({ kind: "stop", units: [0] });
          const tick = await client.advance(3);
          if ((await command).error !== null) throw new Error("the recorded command was refused");
          const captured = {
            battle: {
              scenario: prepared.scenario + " ".repeat(48 * 1024 * 1024),
              report: prepared.report,
            },
            replay: await client.replay(),
          };
          await rememberReplay(JSON.stringify(captured));
          const stored = await readSavedReplay();
          if (stored.battle.scenario !== captured.battle.scenario)
            throw new Error("large captured scenario was not retained exactly");
          const playback = prepareBattle(
            {
              type: "prepare-replay",
              battle: stored.battle,
            },
            () => {},
          );
          const restored = await playback.battle;
          const replay = createSimClient({
            scenario: restored.scenario,
            seed: 1,
            side: "blue",
            transport: "worker",
            connect: restored.connect,
            replay: stored.replay,
          });
          let replayDigest = "";
          replay.onPublication((p: { digest: string; release(): void }) => {
            replayDigest = p.digest;
            p.release();
          });
          try {
            await replay.ready;
            replay.pause();
            replay.start();
            await replay.advance(tick);
            // Exercise the importer at the real browser storage edge. A failed
            // write must not hand the new file to its viewer or navigate away.
            const { createElement } = (
              await importModule(`${root}/../../throwaway/vite-cache/deps/react.js`)
            ).default;
            const { createRoot } = (
              await importModule(`${root}/../../throwaway/vite-cache/deps/react-dom_client.js`)
            ).default;
            const host = document.createElement("div");
            document.body.append(host);
            const uiRoot = createRoot(host);
            let imported: unknown = null;
            uiRoot.render(
              createElement(ReplayImport, {
                plays: (file: { variant?: string }) => file.variant === "ordinary",
                onLoad: (file: unknown) => {
                  imported = file;
                },
              }),
            );
            const deadline = performance.now() + 5000;
            const wait = async () => {
              if (performance.now() > deadline)
                throw new Error(`import did not settle: ${host.innerHTML}`);
              await new Promise((r) => setTimeout(r, 0));
            };
            try {
              while (!host.querySelector("input")) await wait();
              const open = indexedDB.open.bind(indexedDB);
              indexedDB.open = () => {
                throw new Error("Storage unavailable");
              };
              const village = { variant: "ordinary", replay: "{}" };
              const input = host.querySelector("input")!;
              const transfer = new DataTransfer();
              transfer.items.add(new File([JSON.stringify(village)], "village.json"));
              input.files = transfer.files;
              input.dispatchEvent(new Event("change", { bubbles: true }));
              try {
                while (!host.textContent?.includes("Storage unavailable")) await wait();
                if (imported !== null) throw new Error("failed storage started another replay");
              } finally {
                indexedDB.open = open;
              }
              if ((await readSavedReplay()).battle.scenario !== captured.battle.scenario)
                throw new Error("failed storage replaced the previous replay");
              input.dispatchEvent(new Event("change", { bubbles: true }));
              while (imported === null) await wait();
              const remembered = await readSavedReplay();
              if (JSON.stringify(remembered) !== JSON.stringify(village))
                throw new Error("import started before its file was persisted");
            } finally {
              uiRoot.unmount();
              host.remove();
            }
            return { tick, digest, replay: replayDigest };
          } finally {
            replay.dispose();
            playback.cancel();
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
