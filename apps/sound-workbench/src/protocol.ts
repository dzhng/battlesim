import type { AudioMix } from "../../../packages/battle-audio/src/audioMix";
import type { SoundCatalog } from "../../../packages/battle-audio/src/catalog";
import type { Replacement } from "../../fixture-publication/publication";

export interface Snapshot {
  revision: string;
  catalog: SoundCatalog;
  /** `rules.json`'s weapon rows by name, each with the row it `extends`: a
   *  row without a firing choice of its own takes its nearest ancestor's. */
  weapons: Record<string, { extends?: string }>;
  /** The synthesized baseline shots (`presentation.audio.shots`). */
  firing: AudioMix["shots"];
  /** Each vehicle class's loops (`presentation.audio.vehicles`), read only here. */
  vehicles: AudioMix["vehicles"];
  /** A soldier's footstep and stride (`presentation.audio.footsteps`), read only here. */
  footsteps: AudioMix["footsteps"];
  /** A soldier's running pace (`presentation.pose.gait.run_mps`). */
  runMps: number;
  /** Battle music's mood direction and ducking (`presentation.audio.music`), read only here. */
  music: AudioMix["music"];
  materials: string[];
}
export interface Draft {
  revision: string;
  catalog: SoundCatalog;
}
export interface Review {
  candidateId: string;
  revision: string;
  files: Replacement[];
}
export interface Save {
  candidateId: string;
  revision: string;
}
/** A generation-tool command the workbench queued, voices or music. */
export interface GenerationJob {
  id: string;
  /** The generation tool's arguments. */
  args: string[];
  status: "queued" | "running" | "done" | "failed";
  /** The tool's latest progress lines. */
  lines: string[];
}
export interface WorkbenchAPI {
  snapshot(): Promise<Snapshot>;
  preview(draft: Draft): Promise<Review>;
  save(review: Save): Promise<Snapshot>;
}

async function request<T>(endpoint: string, body?: unknown): Promise<T> {
  const response = await fetch(
    `/__sound-workbench/${endpoint}`,
    body === undefined
      ? undefined
      : {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
        },
  );
  const result = await response.json();
  if (!response.ok) throw new Error(result.error ?? "Sound workbench request failed");
  return result;
}
export const workbenchAPI: WorkbenchAPI = {
  snapshot: () => request("snapshot"),
  preview: (draft) => request("preview", draft),
  save: (review) => request("save", review),
};

/** The Music tab's generation: a battle track bought again, and the tool's queue. */
export interface MusicAPI {
  regenerate(track: string): Promise<{ job: GenerationJob }>;
  jobs(): Promise<{ jobs: GenerationJob[] }>;
}
export const musicAPI: MusicAPI = {
  regenerate: (track) => request("music/regenerate", { track }),
  jobs: () => request("music/jobs"),
};
