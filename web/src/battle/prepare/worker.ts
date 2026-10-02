/** Preparation owns the world until the same worker starts its battle. */
import init, * as wasm from "@wasm/game_wasm.js";
import { publicWorldOf, publicWorldBuffers } from "../sim/publicWorld";
import { loadEncounter, loadMap } from "../../maps/browser";
import { workerAuthority, type WorkerScope } from "../sim/workerAuthority";
import type { Authority } from "../sim/authority";
import { PreparationRefused, prepare } from "./prepare";
import type { PrepareWorkerRequest } from "./protocol";

const scope = self as unknown as WorkerScope;
let authority: Authority | null = null;
self.addEventListener("message", (event: MessageEvent<PrepareWorkerRequest>) => {
  if (event.data.type === "static") {
    const { map, rules } = event.data;
    void init()
      .then(() => {
        const view = new wasm.WorldView(map, rules);
        try {
          const world = publicWorldOf(view, wasm.world_layout(rules));
          self.postMessage({ type: "static", world }, { transfer: publicWorldBuffers(world) });
        } finally {
          view.free();
        }
      })
      .catch((error: unknown) => self.postMessage({ type: "error", message: String(error) }))
      .finally(() => self.close());
    return;
  }
  if (event.data.type !== "prepare") {
    authority?.handle(event.data);
    return;
  }
  const { request, documents, stress } = event.data;
  void init()
    .then(async ({ memory }) => {
      const result = await prepare(
        wasm,
        memory,
        request,
        documents,
        { loadMap, loadEncounter },
        (stage) => self.postMessage({ type: "stage", stage }),
        undefined,
        stress,
      );
      const { world, scenario, report } = result;
      self.postMessage({ type: "stage", stage: "world" });
      const exportStarted = performance.now();
      const publicWorld = publicWorldOf(world, world.layout());
      report.publicExportMs = performance.now() - exportStarted;
      report.wasmBytes = memory.buffer.byteLength;
      report.publicBytes =
        publicWorldBuffers(publicWorld).reduce((n, b) => n + b.byteLength, 0) +
        new TextEncoder().encode(publicWorld.queries).byteLength;
      authority = workerAuthority(scope, async () => ({
        memory,
        takePublicWorld: () => null,
        createBattle: (scenario, seed) => world.into_battle(scenario, seed),
        replayBattle: (scenario, replay) => world.into_replay(scenario, replay),
      }));
      self.postMessage(
        { type: "prepared", battle: { scenario, report, publicWorld } },
        { transfer: publicWorldBuffers(publicWorld) },
      );
    })
    .catch((error: unknown) => {
      self.postMessage(
        error instanceof PreparationRefused
          ? { type: "refused", stage: error.stage, diagnostics: error.diagnostics }
          : { type: "error", message: error instanceof Error ? error.message : String(error) },
      );
      self.close();
    });
});
