/** Main-thread side of battle preparation: one worker per request. */
import type { SimConnection } from "../sim/client";
import type { SimReply } from "../sim/protocol";
import type { ScenarioBundlePreparationReport } from "./scenarioBundle";
export { scenarioBundleCatalog } from "./scenarioBundle";
import type {
  PrepareDiagnostic,
  PreparedBattle,
  PreparationMessage,
  PrepareReply,
  PrepareStage,
  BundlePrepareStage,
  RefusalStage,
  PrepareScenarioBundleMessage,
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

/** The shared battle view only needs the prepared scenario and its live
 * worker connection. Bundle admission has a smaller report, so it uses this
 * narrow seam instead of fabricating ordinary map-preparation metadata. */
export interface PreparedConnectionSession {
  scenario: string;
  connect: SimConnection;
}

export interface Preparation {
  readonly battle: Promise<PreparedSession>;
  /** Close the owned worker. A cancelled pending request never settles,
   * so its answer cannot start a stale battle. */
  cancel(): void;
}

/** Own the worker through preparation and connection handoff. The promise owns
 * settlement; installed handlers own the protocol phase. Cancellation stays
 * separate because even an already-queued answer must become silent. */
function prepareWorker<T>(
  message: PreparationMessage,
  onStage: (stage: PrepareStage) => void,
  admit: (
    reply: Extract<PrepareReply, { type: "prepared" | "bundle-prepared" }>,
    connect: SimConnection,
  ) => T,
  reportToConsole: boolean,
): { battle: Promise<T>; cancel(): void } {
  const worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  let cancelled = false;
  const cancel = () => {
    cancelled = true;
    worker.terminate();
  };
  const connect: SimConnection = (receive, fail) => {
    worker.onmessage = (event: MessageEvent<SimReply>) => {
      if (!cancelled) receive(event.data);
    };
    worker.onerror = (event) => {
      if (!cancelled) fail(event.message || "The battle worker failed.");
    };
    return {
      send: (request, transfer) => worker.postMessage(request, { transfer: transfer ?? [] }),
      close: cancel,
    };
  };
  const battle = new Promise<T>((resolve, reject) => {
    worker.onmessage = (event: MessageEvent<PrepareReply>) => {
      if (cancelled) return;
      const reply = event.data;
      if (reply.type === "stage") return onStage(reply.stage);
      if (reply.type === "prepared" || reply.type === "bundle-prepared") {
        try {
          resolve(admit(reply, connect));
        } catch (error) {
          worker.terminate();
          reject(error);
        }
        return;
      }
      worker.terminate();
      if (reply.type === "refused") {
        if (reportToConsole)
          console.warn("battle preparation refused", {
            stage: reply.stage,
            diagnostics: reply.diagnostics,
          });
        reject(
          new PreparationFailed(
            reply.diagnostics.map((d) => d.message).join("; "),
            reply.diagnostics,
            reply.stage,
          ),
        );
      } else {
        if (reportToConsole)
          console.warn("battle preparation worker failed", { message: reply.message });
        reject(new PreparationFailed(reply.message));
      }
    };
    worker.onerror = (event) => {
      if (cancelled) return;
      worker.terminate();
      const message = event.message || "The preparation worker failed to start.";
      if (reportToConsole) console.warn("battle preparation worker error", { message });
      reject(new PreparationFailed(message));
    };
  });
  worker.postMessage(message);
  return { battle, cancel };
}

export function prepareBattle(
  message: PreparationMessage,
  onStage: (stage: PrepareStage) => void,
): Preparation {
  return prepareWorker(
    message,
    onStage,
    (reply, connect) => {
      if (reply.type !== "prepared")
        throw new PreparationFailed(
          "The worker returned an uploaded bundle to the ordinary battle path.",
        );
      return { ...reply.battle, connect };
    },
    true,
  );
}

export interface PreparedScenarioBundleSession {
  canonical: string;
  report: ScenarioBundlePreparationReport;
  session: PreparedConnectionSession;
}

export interface ScenarioBundlePreparation {
  readonly battle: Promise<PreparedScenarioBundleSession>;
  cancel(): void;
}

/** Start normal worker authority from an explicit downloaded bundle. */
export function prepareUploadedScenarioBundle(
  bundle: string,
  onStage: (stage: PrepareStage | BundlePrepareStage) => void = () => {},
  activeHuman?: string | null,
): ScenarioBundlePreparation {
  const message: PrepareScenarioBundleMessage = {
    type: "prepare-bundle",
    bundle,
    active_human: activeHuman,
  };
  return prepareWorker(
    message,
    onStage,
    (reply, connect) => {
      if (reply.type !== "bundle-prepared")
        throw new PreparationFailed("The worker returned an ordinary battle instead of a bundle.");
      return {
        canonical: reply.battle.canonical,
        report: reply.battle.report,
        session: { scenario: reply.battle.scenario, connect },
      };
    },
    false,
  );
}
