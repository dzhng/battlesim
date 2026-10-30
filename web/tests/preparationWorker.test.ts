// @vitest-environment node
import { expect, test } from "vitest";
import type { Browser } from "playwright";
import { launchBrowser } from "../browser.mjs";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import * as wasm from "@wasm/game_wasm.js";
import config from "@fixtures/generated-battle.json";
import { TEST_RULES } from "./catalog";
import { generationRequest } from "../src/maps/source";
import { browserTestServer } from "./support/browserServer";

// No GPU device or drawing: this tests the real worker ownership boundary.
test("a preparation worker becomes the battle authority and replays its commands", async () => {
  const server = await browserTestServer();
  let browser: Browser | undefined;
  try {
    browser = await launchBrowser({
      args: ["--disable-gpu", "--disable-software-rasterizer"],
    });
    const page = await browser.newPage();
    await page.goto(`${server.url}__test_setup__`);
    const shots = join(process.cwd(), "../throwaway/replay-closeout/loading-shots");
    await mkdir(shots, { recursive: true });
    await page.exposeFunction("replayShot", async (name: string) => {
      await page.screenshot({ path: join(shots, `${name}.png`) });
    });
    const documents = Object.fromEntries(
      ["presets", "templates"].map((name, i) => [
        name,
        readFileSync(
          new URL(
            `../../fixtures/${["map-presets", "prototype-building-templates"][i]}.json`,
            import.meta.url,
          ),
          "utf8",
        ),
      ]),
    ) as { presets: string; templates: string };
    wasm.initSync({
      module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
    });
    const map = generationRequest(
      wasm,
      { type: "open", size: "small", seed: "1", profile: "skirmish" },
      documents,
      config.limits,
    );
    const result = await page.evaluate(
      async ({ documents, map, rules, root }) => {
        // Keep browser imports outside Vitest's server-side import rewriting.
        const importModule = new Function("url", "return import(url)");
        const { applyHudTheme } = await importModule(`${root}/battle/present/hudTheme.ts`);
        applyHudTheme();
        const { prepareBattle } = await importModule(`${root}/battle/prepare/client.ts`);
        const { createSimClient } = await importModule(`${root}/battle/sim/client.ts`);
        const { rememberReplay, readSavedReplay, ReplayImport } = await importModule(
          `${root}/../../apps/battle-lab/src/replayFile.tsx`,
        );
        const { default: Battle } = await importModule(
          `${root}/../../apps/battle-lab/src/routes/battle.tsx`,
        );
        const preparation = prepareBattle(
          {
            type: "prepare",
            request: {
              map_source: { kind: "generated", request: map },
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
            documents: { ...documents, rules },
          },
          () => {},
        );
        const prepared = await preparation.battle;
        const client = createSimClient({
          scenario: prepared.scenario,
          seed: 1,
          side: "blue",
          player: "blue-1",
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
          const command = client.command({ kind: "ready" });
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
            const { createElement, createRoot } = await importModule(
              `${root}/../tests/support/browserVisit.tsx`,
            );
            const host = document.createElement("div");
            document.body.replaceChildren();
            document.body.append(host);
            const uiRoot = createRoot(host);
            const { BrowserVisit } = await importModule(
              `${root}/../tests/support/browserVisit.tsx`,
            );
            const renderScreen = (screen: unknown) =>
              uiRoot.render(createElement(BrowserVisit, null, screen));
            // A route renders under its page's catalog, as the router scopes it.
            const { catalogSet } = await importModule(`${root}/battle/catalog/sets.ts`);
            const { SessionCatalogProvider } = await importModule(
              `${root}/battle/catalog/context.tsx`,
            );
            const gameCatalog = await catalogSet("game");
            const scoped = (catalog: unknown, page: unknown) =>
              createElement(SessionCatalogProvider, { catalog }, page);
            let imported: unknown = null;
            renderScreen(
              createElement(ReplayImport, {
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
              // Hold an actual IndexedDB transaction open so the saved replay's
              // read remains pending. It must not offer a guessed replay viewer.
              const db = await new Promise<IDBDatabase>((resolve, reject) => {
                const open = indexedDB.open("battle-replay", 1);
                open.onsuccess = () => resolve(open.result);
                open.onerror = () => reject(open.error);
              });
              const lock = db.transaction("replay", "readwrite");
              lock.oncomplete = () => db.close();
              const store = lock.objectStore("replay");
              let held = true;
              const keep = () => {
                store.get("pending-read-lock").onsuccess = () => {
                  if (held) keep();
                };
              };
              keep();
              const shot = async (name: string) => {
                await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
                await (window as unknown as { replayShot(name: string): Promise<void> }).replayShot(
                  name,
                );
              };
              try {
                window.history.replaceState(null, "", "/battle?replay=saved");
                renderScreen(scoped(gameCatalog, createElement(Battle)));
                while (!host.querySelector('[data-testid="loading"]')) await wait();
                if (host.querySelector('input[type="file"]'))
                  throw new Error("pending replay looked absent");
                await shot("prepared-pending");
                const pulse = host
                  .querySelector('.loading-stages li[data-state="active"] .loading-stage-bar')!
                  .getAnimations()[0];
                pulse.pause();
                pulse.currentTime = 0;
                await shot("prepared-pulse-high");
                pulse.currentTime = Number(pulse.effect!.getComputedTiming().duration) / 2;
                await shot("prepared-pulse-low");
              } finally {
                held = false;
              }
              while (host.textContent?.includes("Reading saved battle")) await wait();
              await shot("replay-ready");
              db.close();
              renderScreen(
                createElement(ReplayImport, {
                  onLoad: (file: unknown) => {
                    imported = file;
                  },
                }),
              );
              // The menu's own file input stays until React replaces the menu.
              while (host.querySelector(".menu") || !host.querySelector('input[type="file"]'))
                await wait();
              const open = indexedDB.open.bind(indexedDB);
              indexedDB.open = () => {
                throw new Error("Storage unavailable");
              };
              const other = { battle: { scenario: "{}", report: { request: {} } }, replay: "{}" };
              const input = host.querySelector<HTMLInputElement>('input[type="file"]')!;
              const transfer = new DataTransfer();
              transfer.items.add(new File([JSON.stringify(other)], "other.json"));
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
              if (JSON.stringify(remembered) !== JSON.stringify(other))
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
        map,
        rules: JSON.stringify(TEST_RULES),
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
