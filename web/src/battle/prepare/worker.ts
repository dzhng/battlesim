/** Preparation worker entry: one request, its stages, one answer, and then
 *  the battle. Map resolution, generation and encounter planning never run on
 *  the page's thread. The world the planner used stays here and the battle
 *  starts from it, so after its answer this worker is the battle's authority.
 *  The page closes it to cancel, which frees everything it allocated. */
import init, * as wasm from "@wasm/game_wasm.js";
import { loadEncounter, loadMap } from "../../maps/browser";
import { workerAuthority, type WorkerScope } from "../sim/workerAuthority";
import type { Authority } from "../sim/authority";
import { PreparationRefused, prepare } from "./prepare";
import type { PrepareWorkerRequest } from "./protocol";

const scope = self as unknown as WorkerScope;
let authority: Authority | null = null;
self.addEventListener("message", (event: MessageEvent<PrepareWorkerRequest>) => {
  if (event.data.type !== "prepare") {
    authority?.handle(event.data);
    return;
  }
  const { request, documents, stress } = event.data;
  void init()
    .then(async ({ memory }) => {
      const { world, scenario, report } = await prepare(
        wasm,
        memory,
        request,
        documents,
        { loadMap, loadEncounter },
        (stage) => self.postMessage({ type: "stage", stage }),
        undefined,
        stress,
      );
      authority = workerAuthority(scope, async () => ({
        memory,
        createBattle: (scenario, seed) => world.into_battle(scenario, seed),
        replayBattle: (scenario, replay) => world.into_replay(scenario, replay),
      }));
      self.postMessage({ type: "prepared", battle: { scenario, report } });
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
