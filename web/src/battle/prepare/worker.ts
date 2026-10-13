/** Preparation worker entry: one request, its stages, one answer, and then
 *  the battle. Map resolution, generation and fielding the forces never run
 *  on the page's thread. The world preparation built stays here and the battle
 *  starts from it, so after its answer this worker is the battle's authority.
 *  The page closes it to cancel, which frees everything it allocated. */
import init, * as wasm from "@wasm/game_wasm.js";
import { workerAuthority, type WorkerScope } from "../sim/workerAuthority";
import type { Authority } from "../sim/authority";
import { PreparationRefused, prepare, prepareReplay } from "./prepare";
import { prepareScenarioBundle, type ScenarioBundlePreparationModule } from "./scenarioBundle";
import type { PrepareWorkerRequest } from "./protocol";

const scope = self as unknown as WorkerScope;
let authority: Authority | null = null;
self.addEventListener("message", (event: MessageEvent<PrepareWorkerRequest>) => {
  if (
    event.data.type !== "prepare" &&
    event.data.type !== "prepare-replay" &&
    event.data.type !== "prepare-bundle"
  ) {
    authority?.handle(event.data);
    return;
  }
  const message = event.data;
  void init()
    .then(async ({ memory }) => {
      if (message.type === "prepare-bundle") {
        self.postMessage({ type: "stage", stage: "bundle" });
        const prepared = prepareScenarioBundle(
          wasm as unknown as ScenarioBundlePreparationModule,
          message.bundle,
          message.active_human,
        );
        const controllers = wasm.scenario_bundle_controllers(
          prepared.canonical,
          message.active_human === undefined
            ? prepared.report.human_capable_slots[0]
            : (message.active_human ?? undefined),
        );
        authority = workerAuthority(scope, async () => ({
          memory,
          createBattle: (scenario, seed) => {
            const battle = prepared.world.into_battle(scenario, seed);
            battle.set_controllers(controllers);
            return battle;
          },
          replayBattle: (scenario, replay) => prepared.world.into_replay(scenario, replay),
        }));
        self.postMessage({
          type: "bundle-prepared",
          battle: {
            canonical: prepared.canonical,
            scenario: prepared.scenario,
            report: prepared.report,
          },
        });
        return;
      }
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
