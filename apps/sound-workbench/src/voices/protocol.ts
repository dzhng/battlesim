import type {
  Intensity,
  Trigger,
  VoiceCatalog,
} from "../../../../packages/battle-audio/src/voices/voiceCatalog";
import type { Match } from "../../../../packages/battle-audio/tools/generate/transcript";
import type { GenerationJob, Review, Save } from "../protocol";

/** One locked take as the review grid shows it. */
export interface TakeView {
  text: string;
  /** The served bytes; `voiceUrl` builds the take's address from it. */
  sha256: string;
  duration_s: number;
  transcript: string;
  match: Match;
  /** The standing verdict: one given to other bytes reads as unreviewed. */
  verdict: "ok" | "bad" | null;
}
/** A set's takes by review state. `flagged` counts flagged takes still awaiting a verdict. */
export interface SetCounts {
  /** Every take the set's voices speak: voices × their triggers × 2 intensities × 2 takes. */
  expected: number;
  /** Takes in the lock. */
  takes: number;
  unreviewed: number;
  flagged: number;
  ok: number;
  bad: number;
}
/** A voice's Voice Design candidates, by `<set>/<voice>`. */
export interface DesignView {
  /** The role's audition text every candidate speaks. */
  text: string;
  candidates: { transcript: string | null; score: number | null; duration_s: number }[];
  /** The candidate saved as the voice (1-based), if any. */
  picked: number | null;
}
export interface VoiceSnapshot {
  /** The `voices.json` generation a draft edits. Verdicts never change it. */
  revision: string;
  catalog: VoiceCatalog;
  /** Locked takes by take id. */
  takes: Record<string, TakeView>;
  counts: Record<string, SetCounts>;
  designs: Record<string, DesignView>;
}
export interface VoiceDraft {
  revision: string;
  catalog: VoiceCatalog;
}
export interface VerdictRequest {
  verdict: "ok" | "bad";
  /** Each take with the served sha it was heard as. */
  takes: { take: string; sha256: string }[];
}
export interface VerdictResult {
  /** The judged takes as the lock now holds them. */
  takes: Record<string, TakeView>;
  /** The judged sets' counts. */
  counts: Record<string, SetCounts>;
  /** Takes refused because their bytes changed since they were heard. */
  changed: string[];
}
/** `generate voices`' selection; `sets` is required so a click never spends on the whole library. */
export interface VoiceScope {
  sets: string[];
  voices?: string[];
  triggers?: Trigger[];
  intensities?: Intensity[];
  take?: number;
  only?: "missing" | "bad" | "flagged";
}
export interface VoiceAPI {
  snapshot(): Promise<VoiceSnapshot>;
  preview(draft: VoiceDraft): Promise<Review>;
  save(review: Save): Promise<VoiceSnapshot>;
  verdict(request: VerdictRequest): Promise<VerdictResult>;
  generate(scope: VoiceScope): Promise<{ job: GenerationJob }>;
  /** Designs new candidates for `<set>/<voice>`. */
  design(voice: string): Promise<{ job: GenerationJob }>;
  /** Saves candidate 1–3 as the voice; voices.json gains its voice_id. */
  /** Saves a design candidate as the voice; `replace` swaps one already picked. */
  pick(voice: string, candidate: number, replace: boolean): Promise<VoiceSnapshot>;
  jobs(): Promise<{ jobs: GenerationJob[] }>;
}
/** A design candidate rendered through its set's profile. */
export const candidateUrl = (voice: string, candidate: number) =>
  `/__sound-workbench/voices/candidate/${voice}/${candidate}`;

async function request<T>(endpoint: string, body?: unknown): Promise<T> {
  const response = await fetch(
    `/__sound-workbench/voices/${endpoint}`,
    body === undefined
      ? undefined
      : {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
        },
  );
  const result = await response.json();
  if (!response.ok) throw new Error(result.error ?? "Voice workbench request failed");
  return result;
}
export const voiceAPI: VoiceAPI = {
  snapshot: () => request("snapshot"),
  preview: (draft) => request("preview", draft),
  save: (review) => request("save", review),
  verdict: (body) => request("verdict", body),
  generate: (scope) => request("generate", { scope }),
  design: (voice) => request("design", { voice }),
  pick: (voice, candidate, replace) => request("pick", { voice, candidate, replace }),
  jobs: () => request("jobs"),
};
