/** Main-thread side of battle preparation: one worker per request. */
import type { SimConnection } from "../sim/client";
import type { SimReply } from "../sim/protocol";
import type {
  PrepareDiagnostic,
  PreparedBattle,
  PrepareMessage,
  PrepareReply,
  PrepareStage,
  RefusalStage,
} from "./protocol";

/** Why preparation ended without a battle: a refusal with its diagnostics
 *  and the stage that refused (the request's check, the map, the
 *  encounter), or a failure of the worker itself (`stage` null). */
export class PreparationFailed extends Error {
  constructor(
    message: string,
    readonly diagnostics: PrepareDiagnostic[] = [],
    readonly stage: RefusalStage | null = null,
  ) {
    super(message);
  }
}

export interface PreparedSession extends PreparedBattle {
  connect: SimConnection;
}

export interface Preparation {
  readonly battle: Promise<PreparedSession>;
  /** Close the owned worker. A cancelled pending request never settles,
   * so its answer cannot start a stale battle. */
  cancel(): void;
}

export function prepareBattle(
  message: PrepareMessage,
  onStage: (stage: PrepareStage) => void,
): Preparation {
  const worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  // A cancelled request stays silent even if its answer was already on its
  // way when the worker was closed.
  let cancelled = false;
  const battle = new Promise<PreparedSession>((resolve, reject) => {
    worker.onmessage = (event: MessageEvent<PrepareReply>) => {
      if (cancelled) return;
      const reply = event.data;
      if (reply.type === "stage") return onStage(reply.stage);
      if (reply.type === "prepared") {
        resolve({
          ...reply.battle,
          connect: (receive, fail) => {
            worker.onmessage = (event: MessageEvent<SimReply>) => {
              if (!cancelled) receive(event.data);
            };
            worker.onerror = (event) => {
              if (!cancelled) fail(event.message || "The battle worker failed.");
            };
            return {
              send: (request, transfer) =>
                worker.postMessage(request, { transfer: transfer ?? [] }),
              close: () => {
                cancelled = true;
                worker.terminate();
              },
            };
          },
        });
        return;
      }
      worker.terminate();
      if (reply.type === "refused")
        reject(
          new PreparationFailed(
            reply.diagnostics.map((d) => d.message).join("; "),
            reply.diagnostics,
            reply.stage,
          ),
        );
      else reject(new PreparationFailed(reply.message));
    };
    worker.onerror = (event) => {
      if (cancelled) return;
      worker.terminate();
      reject(new PreparationFailed(event.message || "The preparation worker failed to start."));
    };
  });
  worker.postMessage(message);
  return {
    battle,
    cancel: () => {
      cancelled = true;
      worker.terminate();
    },
  };
}
