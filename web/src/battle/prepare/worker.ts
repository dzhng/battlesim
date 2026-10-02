/** Preparation worker entry: one request, its stages, one answer. Map
 *  resolution, generation and encounter planning never run on the page's
 *  thread, and the page closes this worker after the answer (or to cancel),
 *  which frees everything preparation allocated. */
import init, * as wasm from "@wasm/game_wasm.js";
import { loadEncounter, loadMap } from "../../maps/browser";
import { PreparationRefused, prepare } from "./prepare";
import type { PrepareMessage, PrepareReply } from "./protocol";

interface WorkerScope {
  postMessage(message: PrepareReply): void;
  addEventListener(type: "message", listener: (event: MessageEvent<PrepareMessage>) => void): void;
}
const scope = self as unknown as WorkerScope;

scope.addEventListener("message", (event) => {
  const { request, documents, stress } = event.data;
  void init()
    .then(({ memory }) =>
      prepare(
        wasm,
        memory,
        request,
        documents,
        { loadMap, loadEncounter },
        (stage) => scope.postMessage({ type: "stage", stage }),
        undefined,
        stress,
      ),
    )
    .then((battle) => scope.postMessage({ type: "prepared", battle }))
    .catch((error: unknown) =>
      scope.postMessage(
        error instanceof PreparationRefused
          ? { type: "refused", stage: error.stage, diagnostics: error.diagnostics }
          : { type: "error", message: error instanceof Error ? error.message : String(error) },
      ),
    );
});
