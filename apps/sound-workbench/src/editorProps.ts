import type { SoundCatalog } from "../../../packages/battle-audio/src/catalog";
import type { Snapshot } from "./protocol";
export interface EditorProps {
  draft: SoundCatalog;
  snapshot: Snapshot;
  edit(change: (catalog: SoundCatalog) => void): void;
  play(kind: "clip" | "sound", id: string, variant?: number, gain?: number): Promise<void>;
}
