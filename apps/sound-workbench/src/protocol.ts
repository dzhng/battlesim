import type { AudioPresentation } from "../../../packages/battle-audio/src/audioPresentation";
import type { SoundCatalog } from "../../../packages/battle-audio/src/catalog";
import type { Replacement } from "../../fixture-publication/publication";

export interface Snapshot {
  revision: string;
  catalog: SoundCatalog;
  /** `game.json`'s weapon rows by name, each with the row it `extends`: a
   *  row without a firing choice of its own takes its nearest ancestor's. */
  weapons: Record<string, { extends?: string }>;
  /** The synthesized baseline shots (`presentation.audio.shots`). */
  firing: AudioPresentation["shots"];
  /** Each vehicle class's loops (`presentation.audio.vehicles`), read only here. */
  vehicles: AudioPresentation["vehicles"];
  /** A soldier's footstep and stride (`presentation.audio.footsteps`), read only here. */
  footsteps: AudioPresentation["footsteps"];
  /** A soldier's running pace (`presentation.pose.gait.run_mps`). */
  runMps: number;
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
