import { useLabLoading } from "./LabLoading";
// A saved battle and every accepted command. Prepared battles retain the
// exact scenario bytes (compiled map, encounter and resolved rules), so
// fixture edits cannot change playback. The simulation checks the engine,
// scenario and rules digests; replay retention across builds is unsupported.
import { useNavigate } from "react-router";
import { usePageVisitActive } from "./navigation";
import { useEffect, useRef, useState } from "react";
import type { PreparedBattle } from "@web/battle/prepare/protocol";
import { LoadingScreen } from "./LoadingScreen";

/** A prepared battle and every command it accepted. */
export interface ReplayFile {
  battle: PreparedBattle;
  replay: string;
}

/** The viewer of a saved replay. */
export const REPLAY_ROUTE = "/battle?replay=saved";

const LAST_REPLAY_KEY = "last-replay";

/** One saved battle, replaced in a transaction. Compiled maps exceed the
 *  localStorage quota; IndexedDB keeps them across the viewer's navigation. */
function storedReplay(text?: string): Promise<string | undefined> {
  return new Promise((resolve, reject) => {
    const opened = indexedDB.open("battle-replay", 1);
    opened.onupgradeneeded = () => opened.result.createObjectStore("replay");
    opened.onerror = () => reject(opened.error);
    opened.onsuccess = () => {
      const db = opened.result;
      const transaction = db.transaction("replay", text === undefined ? "readonly" : "readwrite");
      const store = transaction.objectStore("replay");
      const request =
        text === undefined ? store.get(LAST_REPLAY_KEY) : store.put(text, LAST_REPLAY_KEY);
      transaction.oncomplete = () => {
        db.close();
        resolve(text ?? request.result);
      };
      transaction.onabort = transaction.onerror = () => {
        db.close();
        reject(transaction.error ?? request.error);
      };
    };
  });
}

/** A replay file's text, parsed; throws when it is not one. */
export function parseReplayFile(text: string): ReplayFile {
  const file = JSON.parse(text) as Partial<ReplayFile> | null;
  const named = typeof file?.battle?.scenario === "string" && !!file.battle.report?.request;
  if (!file || typeof file.replay !== "string" || !named) throw new Error("not a saved battle");
  return file as ReplayFile;
}

/** The battle saved last on this browser, if any. */
export async function readSavedReplay(): Promise<ReplayFile | null> {
  try {
    const raw = await storedReplay();
    return raw ? parseReplayFile(raw) : null;
  } catch {
    return null;
  }
}

/** Persist an imported replay before a navigation can discard its page state. */
export async function rememberReplay(text: string): Promise<void> {
  await storedReplay(text);
}

/** Reading storage never overwrites a file imported while that read was pending. */
export function useSavedReplay() {
  const [loaded, setLoaded] = useState<{ file: ReplayFile | null | undefined; n: number }>({
    file: undefined,
    n: 0,
  });
  useEffect(() => {
    let live = true;
    void readSavedReplay().then((file) => {
      if (live) setLoaded((l) => (l.n === 0 ? { file, n: 0 } : l));
    });
    return () => {
      live = false;
    };
  }, []);
  const setFile = (file: ReplayFile) => setLoaded((l) => ({ file, n: l.n + 1 }));
  return [loaded, setFile] as const;
}

/** A pending storage read is neither a missing replay nor a battle to start. */
export function ReplayLoading() {
  return (
    <LoadingScreen
      title="Loading replay"
      subject="SAVED BATTLE"
      stages={[{ id: "read", label: "Reading saved battle" }]}
      current="read"
    />
  );
}

/** Keep `file` as the last saved battle and download it as `name`. */
export async function saveReplay(file: ReplayFile, name: string) {
  const text = JSON.stringify(file);
  try {
    await rememberReplay(text);
  } catch {
    /* Storage failure must not prevent a download. */
  }
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  // Revoking at once can cancel the download in some browsers.
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

/** Load a saved battle from a file and keep it as the last saved battle.
 *  The viewer hands it to `onLoad`; elsewhere (the menu) it is opened in the
 *  viewer. */
export function ReplayImport({ onLoad }: { onLoad?: (file: ReplayFile) => void }) {
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();
  const visitActive = usePageVisitActive();
  const generation = useRef(0);
  useEffect(
    () => () => {
      generation.current++;
    },
    [],
  );
  useLabLoading("renderer", true);
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
          const request = ++generation.current;
          const active = () => request === generation.current && visitActive();
          void (async () => {
            try {
              const text = await f.text();
              if (!active()) return;
              const file = parseReplayFile(text);
              await rememberReplay(text);
              if (!active()) return;
              setError(null);
              if (onLoad) return onLoad(file);
              void navigate(REPLAY_ROUTE);
            } catch (err) {
              if (active()) setError(err instanceof Error ? err.message : String(err));
            }
          })();
        }}
      />
      {error && <span className="hud-error"> {error}</span>}
    </label>
  );
}
