/** Preparation worker entry: one request, its stages, one answer, and then
 *  the battle. Map resolution, generation and fielding the forces never run
 *  on the page's thread. The world preparation built stays here and the battle
 *  starts from it, so after its answer this worker is the battle's authority.
 *  The page closes it to cancel, which frees everything it allocated. */
import init, * as wasm from "@wasm/game_wasm.js";
import { workerAuthority, type WorkerScope } from "../sim/workerAuthority";
import type { Authority } from "../sim/authority";
import { PreparationRefused, prepare, prepareReplay } from "./prepare";
import type { PrepareWorkerRequest } from "./protocol";

const scope = self as unknown as WorkerScope;
let authority: Authority | null = null;
self.addEventListener("message", (event: MessageEvent<PrepareWorkerRequest>) => {
  if (event.data.type !== "prepare" && event.data.type !== "prepare-replay") {
    authority?.handle(event.data);
    return;
  }
  const message = event.data;
  void init()
    .then(async ({ memory }) => {
      const { world, scenario, report } =
        message.type === "prepare-replay"
          ? prepareReplay(wasm, message.battle)
          : await prepare(
              wasm,
              memory,
              message.request,
              message.documents,
              (stage) => self.postMessage({ type: "stage", stage }),
              undefined,
              message.stress,
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
