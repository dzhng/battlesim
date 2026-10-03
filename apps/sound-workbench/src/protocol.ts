import type { AudioPresentation } from "../../../packages/battle-audio/src/audioPresentation";
import type { SoundCatalog } from "../../../packages/battle-audio/src/catalog";
import type { Replacement } from "../../fixture-publication/publication";

export interface SoundUnit {
  id: string;
  name: string;
  mounts: { name: string; weapons: string[]; count: number }[];
}
export interface Snapshot {
  revision: string;
  catalog: SoundCatalog;
  units: SoundUnit[];
  firing: AudioPresentation["shots"];
  materials: string[];
  rounds: string[];
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
