/** The lobby's map preview (D14, D39): each lobby change shows a fresh
 *  admitted preparation through `admitBattle`, and keeps only what Deploy and
 *  the picture need: the admitted request, its map, its sites and the SVG.
 *  A newer request cancels the older one, and a finished preview closes its
 *  worker; Deploy prepares the kept request again and gets the same battle. */
import {
  admitBattle,
  type Admission,
  type AdmissionAttempt,
  type AdmissionEdges,
  type AdmissionInputs,
  type AdmissionPolicy,
} from "./admission";
import type { PrepareBattleRequest, PrepareDocuments, PreparedPicture } from "./protocol";

export interface MapPreview extends PreparedPicture {
  /** The admitted request, as the simulation's check wrote it back. */
  request: PrepareBattleRequest;
}

/** Where the preview is. Loading holds from the moment a request is shown
 *  (an XL 5v5 takes seconds: D53); retrying means an earlier map was refused
 *  for a reason another seed can fix, and `attempt` is the candidate now
 *  preparing. */
export type PreviewState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "retrying"; attempt: number }
  | { status: "ready"; preview: MapPreview }
  | { status: "failed"; message: string; attempts: readonly AdmissionAttempt[] };

export interface MapPreviews {
  readonly state: PreviewState;
  /** Preview the lobby's candidate requests, cancelling any older preview. */
  show(candidate: AdmissionInputs["candidate"]): void;
  /** Cancel the running preview; nothing more is reported. */
  dispose(): void;
}

export function mapPreviews(
  inputs: {
    documents: PrepareDocuments;
    policy: AdmissionPolicy;
    onChange(state: PreviewState): void;
  },
  edges: Partial<AdmissionEdges> = {},
): MapPreviews {
  let state: PreviewState = { status: "idle" };
  let running: Admission | null = null;
  const set = (next: PreviewState) => {
    state = next;
    inputs.onChange(next);
  };
  const stop = () => {
    running?.cancel();
    running = null;
  };
  return {
    get state() {
      return state;
    },
    show(candidate) {
      stop();
      set({ status: "loading" });
      let attempt = 0;
      const admission = admitBattle(
        {
          candidate,
          documents: inputs.documents,
          policy: inputs.policy,
          preview: true,
          onRequest: () => {
            // The first candidate is requested before `admission` is assigned.
            if (++attempt > 1 && running === admission) set({ status: "retrying", attempt });
          },
        },
        () => {},
        edges,
      );
      running = admission;
      admission.battle.then(
        (battle) => {
          if (running !== admission) return;
          stop();
          if (!battle.preview)
            return set({
              status: "failed",
              message: "The preparation returned no map picture.",
              attempts: admission.attempts,
            });
          set({ status: "ready", preview: { request: battle.report.request, ...battle.preview } });
        },
        (error: unknown) => {
          if (running !== admission) return;
          stop();
          set({
            status: "failed",
            message: error instanceof Error ? error.message : String(error),
            attempts: admission.attempts,
          });
        },
      );
    },
    dispose: stop,
  };
}
