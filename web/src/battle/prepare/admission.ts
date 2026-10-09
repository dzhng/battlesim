/** Ordinary unpinned Play selects an admitted preparation. Exact requests
 * still go straight to prepareBattle; this wrapper owns candidate workers. */
import type defaults from "@fixtures/generated-battle.json";
import { newSeed } from "../../maps/source";
import { prepareBattle, PreparationFailed, type Preparation, type PreparedSession } from "./client";
import type {
  PrepareBattleRequest,
  PrepareDocuments,
  PrepareStage,
  RefusalStage,
} from "./protocol";

export type AdmissionPolicy = Pick<
  typeof defaults.admission,
  "max_generated_attempts" | "generated_deadline_ms"
>;
export interface AdmissionInputs {
  candidate(seed: string): PrepareBattleRequest;
  documents: PrepareDocuments;
  policy: AdmissionPolicy;
  onRequest?(request: PrepareBattleRequest): void;
}
export interface AdmissionAttempt {
  request: PrepareBattleRequest;
  outcome: "refused" | "timeout" | "error" | "admitted";
  failure?: Pick<PreparationFailed, "message" | "stage" | "diagnostics">;
}
export interface Admission extends Preparation {
  readonly attempts: readonly AdmissionAttempt[];
}
interface AdmissionEdges {
  seed(): string;
  prepare: typeof prepareBattle;
  schedule(run: () => void, ms: number): () => void;
}
const edges: AdmissionEdges = {
  seed: newSeed,
  prepare: prepareBattle,
  schedule: (run, ms) => {
    const timer = setTimeout(run, ms);
    return () => clearTimeout(timer);
  },
};

/** What another map seed can change: the generator running out of room, or
 * the battle refusing the skirmish sites that map laid out. Invalid inputs
 * and runtime faults are the same on every seed. */
const SEED_DEPENDENT: Record<RefusalStage, readonly string[]> = {
  request: [],
  map: ["generation_failed", "complexity_limit"],
  encounter: ["invalid_skirmish_sites"],
};

function canRetry(error: unknown): boolean {
  if (!(error instanceof PreparationFailed) || !error.stage) return false;
  const codes = SEED_DEPENDENT[error.stage];
  return error.diagnostics.length > 0 && error.diagnostics.every((d) => codes.includes(d.code));
}

function evidence(error: unknown): Pick<PreparationFailed, "message" | "stage" | "diagnostics"> {
  const failure =
    error instanceof PreparationFailed
      ? error
      : new PreparationFailed(error instanceof Error ? error.message : String(error));
  return { message: failure.message, stage: failure.stage, diagnostics: failure.diagnostics };
}

export function admitBattle(
  input: AdmissionInputs,
  onStage: (stage: PrepareStage) => void,
  supplied: Partial<AdmissionEdges> = {},
): Admission {
  const run = { ...edges, ...supplied };
  const attempts: AdmissionAttempt[] = [];
  const seeds = new Set<string>();
  let cancelled = false;
  let current: Preparation | null = null;
  let expired = false;
  let expire!: () => void;
  const deadline = new Promise<null>((resolve) => {
    expire = () => resolve(null);
  });
  const clearDeadline = run.schedule(() => {
    expired = true;
    current?.cancel();
    expire();
  }, input.policy.generated_deadline_ms);

  const stage = (value: PrepareStage) => {
    if (!cancelled) onStage(value);
  };
  const select = async (): Promise<PreparedSession> => {
    let failure: unknown = new PreparationFailed("No generated battlefield was admitted.");
    for (let index = 0; index < input.policy.max_generated_attempts && !expired; index++) {
      // A random collision must not turn selection into an unbounded draw loop.
      let seed = run.seed();
      while (seeds.has(seed)) seed = ((BigInt(seed) + 1n) % (1n << 64n)).toString();
      seeds.add(seed);
      const request = input.candidate(seed);
      input.onRequest?.(request);
      current = null;
      try {
        current = run.prepare({ type: "prepare", request, documents: input.documents }, stage);
        const battle = await Promise.race([current.battle, deadline]);
        if (cancelled) return new Promise(() => {});
        if (battle === null) {
          attempts.push({ request, outcome: "timeout" });
          failure = new PreparationFailed("Map generation timed out.");
          break;
        }
        attempts.push({ request, outcome: "admitted" });
        clearDeadline();
        return battle;
      } catch (error) {
        if (cancelled) return new Promise(() => {});
        current?.cancel();
        attempts.push({
          request,
          outcome: error instanceof PreparationFailed && error.stage ? "refused" : "error",
          failure: evidence(error),
        });
        failure = error;
        if (!canRetry(error)) break;
      }
    }
    clearDeadline();
    if (cancelled) return new Promise(() => {});
    throw failure;
  };
  return {
    battle: select().finally(clearDeadline),
    attempts,
    cancel: () => {
      cancelled = true;
      clearDeadline();
      current?.cancel();
    },
  };
}
