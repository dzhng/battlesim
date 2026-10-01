/** Main-thread side of battle preparation: one worker per request. */
import type {
  PrepareDiagnostic,
  PreparedBattle,
  PrepareReply,
  PrepareRequest,
  PrepareStage,
} from "./protocol";

/** Why preparation ended without a battle: a refusal with its diagnostics
 *  (the generator's when `stage` is `generating`, the encounter planner's
 *  when `placing`), or a failure of the worker itself. */
export class PreparationFailed extends Error {
  constructor(
    message: string,
    readonly diagnostics: PrepareDiagnostic[] = [],
    readonly stage: PrepareStage | null = null,
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
  request: PrepareRequest,
  onStage: (stage: PrepareStage) => void,
): Preparation {
  const worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  const battle = new Promise<PreparedBattle>((resolve, reject) => {
    worker.onmessage = (event: MessageEvent<PrepareReply>) => {
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
      worker.terminate();
      reject(new PreparationFailed(event.message || "The preparation worker failed to start."));
    };
  });
  worker.postMessage(request);
  return { battle, cancel: () => worker.terminate() };
}
