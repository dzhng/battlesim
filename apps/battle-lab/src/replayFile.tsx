// A saved battle: what rebuilds its scenario, and every accepted command
// (`sim::battle::Replay`, which pins the scenario's and the rules' digests
// and the battle seed, and refuses to play on any other). The village's file
// names its variant; a prepared battle's holds its whole preparation request,
// whose generation request pins the generator, presets and catalogue, so
// another build refuses it instead of replaying on another map.
import { useState } from "react";
import type { PrepareBattleRequest } from "@web/battle/prepare/protocol";

export interface VillageReplayFile {
  variant: string;
  replay: string;
}
export interface PreparedReplayFile {
  request: PrepareBattleRequest;
  replay: string;
}
export type ReplayFile = VillageReplayFile | PreparedReplayFile;

export const isPreparedReplay = (file: ReplayFile): file is PreparedReplayFile => "request" in file;

/** The viewer of a prepared battle's saved replay. */
export const PREPARED_REPLAY_ROUTE = "/battle?replay=saved";
/** The viewer that plays `file` (the village's when there is none). */
export const replayRoute = (file: ReplayFile | null) =>
  file && isPreparedReplay(file) ? PREPARED_REPLAY_ROUTE : "/replay/village";

const LAST_REPLAY_KEY = "village-last-replay";

/** A replay file's text, parsed; throws when it is not one. */
export function parseReplayFile(text: string): ReplayFile {
  const file = JSON.parse(text) as Partial<VillageReplayFile & PreparedReplayFile> | null;
  const named =
    typeof file?.variant === "string" || (typeof file?.request === "object" && !!file.request);
  if (!file || typeof file.replay !== "string" || !named) throw new Error("not a saved battle");
  return file as ReplayFile;
}

/** The battle saved last on this browser, if any. */
export function readSavedReplay(): ReplayFile | null {
  try {
    const raw = localStorage.getItem(LAST_REPLAY_KEY);
    return raw ? parseReplayFile(raw) : null;
  } catch {
    return null;
  }
}

function remember(text: string) {
  try {
    localStorage.setItem(LAST_REPLAY_KEY, text);
  } catch {
    // storage unavailable: the download still works
  }
}

/** Keep `file` as the last saved battle and download it as `name`. */
export function saveReplay(file: ReplayFile, name: string) {
  const text = JSON.stringify(file);
  remember(text);
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  // Revoking at once can cancel the download in some browsers.
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

/** Load a saved battle from a file. One this viewer `plays` is handed to
 *  `onLoad`; the other kind is kept as the last saved battle and opened in
 *  its own viewer. */
export function ReplayImport<File extends ReplayFile>({
  plays,
  onLoad,
}: {
  plays: (file: ReplayFile) => file is File;
  onLoad: (file: File) => void;
}) {
  const [error, setError] = useState<string | null>(null);
  return (
    <label className="hud-menu-file">
      Load a saved battle{" "}
      <input
        type="file"
        accept="application/json"
        data-testid="replay-file"
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (!f) return;
          void f.text().then((text) => {
            try {
              const file = parseReplayFile(text);
              setError(null);
              if (plays(file)) return onLoad(file);
              const viewer = replayRoute(file);
              if (viewer === window.location.pathname + window.location.search)
                throw new Error("not a saved battle this viewer can play");
              remember(text);
              window.location.assign(viewer);
            } catch (err) {
              setError((err as Error).message);
            }
          });
        }}
      />
      {error && <span className="hud-error"> {error}</span>}
    </label>
  );
}
