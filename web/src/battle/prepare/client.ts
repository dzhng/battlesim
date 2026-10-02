/** Main-thread side of battle preparation: one worker per request. */
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

export interface Preparation {
  readonly battle: Promise<PreparedBattle>;
  /** Stop a request that is no longer wanted: its worker is closed and
   *  `battle` never settles, so a stale result cannot start a battle. */
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
  const battle = new Promise<PreparedBattle>((resolve, reject) => {
    worker.onmessage = (event: MessageEvent<PrepareReply>) => {
      if (cancelled) return;
      const reply = event.data;
      if (reply.type === "stage") return onStage(reply.stage);
      worker.terminate();
      if (reply.type === "prepared") resolve(reply.battle);
      else if (reply.type === "refused")
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
