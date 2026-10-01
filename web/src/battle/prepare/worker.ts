/** Preparation worker entry: one request, its stages, one answer. Generation
 *  never runs on the page's thread, and the page closes this worker after
 *  the answer (or to cancel), which frees everything generation allocated. */
import init, * as wasm from "@wasm/game_wasm.js";
import { MapRefused, prepareGeneratedBattle } from "./generatedBattle";
import type { PrepareReply, PrepareRequest } from "./protocol";

interface WorkerScope {
  postMessage(message: PrepareReply): void;
  addEventListener(type: "message", listener: (event: MessageEvent<PrepareRequest>) => void): void;
}
const scope = self as unknown as WorkerScope;

scope.addEventListener("message", (event) => {
  void init()
    .then(({ memory }) => {
      const battle = prepareGeneratedBattle(wasm, memory, event.data, (stage) =>
        scope.postMessage({ type: "stage", stage }),
      );
      scope.postMessage({ type: "prepared", battle });
    })
    .catch((error: unknown) =>
      scope.postMessage(
        error instanceof MapRefused
          ? { type: "refused", diagnostics: error.diagnostics }
          : { type: "error", message: error instanceof Error ? error.message : String(error) },
      ),
    );
});
