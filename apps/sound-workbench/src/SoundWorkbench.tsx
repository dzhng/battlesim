import { Link } from "react-router";
import {
  useCallback,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import {
  validateSoundCatalog,
  type SoundCatalog,
} from "../../../packages/battle-audio/src/catalog";
import { SoundAuditioner, type Auditioner, type LiveMix } from "./audition";
import { AuditionSession, type AuditionSnapshot } from "./auditionSession";
import {
  musicAPI,
  workbenchAPI,
  type MusicAPI,
  type LookSound,
  type Snapshot,
  type WorkbenchAPI,
} from "./protocol";
import {
  initialSoundWorkbenchState,
  soundWorkbenchBusy,
  soundWorkbenchCanSave,
  soundWorkbenchDirty,
  soundWorkbenchReducer,
} from "./soundWorkbenchState";
import { Library, type Selection } from "./Library";
import { WeaponLooks } from "./WeaponLooks";
import { Movement } from "./Movement";
import { VoicesTab } from "./voices/VoicesTab";
import { MusicTab } from "./music/MusicTab";
import { ChatterPreview } from "./voices/ChatterPreview";
import { voiceAPI, type VoiceAPI } from "./voices/protocol";
import "./sound-workbench.css";

function changedSettings(
  before: unknown,
  after: unknown,
  path = "",
): { path: string; before: string; after: string }[] {
  if (JSON.stringify(before) === JSON.stringify(after)) return [];
  const object = (value: unknown) =>
    value !== null && typeof value === "object" && !Array.isArray(value);
  if ((object(before) || before === undefined) && (object(after) || after === undefined)) {
    const a = (before ?? {}) as Record<string, unknown>;
    const b = (after ?? {}) as Record<string, unknown>;
    return [...new Set([...Object.keys(a), ...Object.keys(b)])].flatMap((key) =>
      changedSettings(a[key], b[key], path ? `${path}.${key}` : key),
    );
  }
  const display = (value: unknown) =>
    typeof value === "string" ? value : (JSON.stringify(value) ?? "not set");
  return [{ path, before: display(before), after: display(after) }];
}

const playingKey = ({ phase, key }: AuditionSnapshot) => (phase === "playing" ? (key ?? "") : "");

export function SoundWorkbench({
  api = workbenchAPI,
  voices = voiceAPI,
  music = musicAPI,
  audition: suppliedAudition,
}: { api?: WorkbenchAPI; voices?: VoiceAPI; music?: MusicAPI; audition?: Auditioner } = {}) {
  const audition = useMemo(() => suppliedAudition ?? new SoundAuditioner(), [suppliedAudition]);
  const [state, dispatch] = useReducer(soundWorkbenchReducer, initialSoundWorkbenchState);
  const { snapshot, draft, lookSounds, review, outcome } = state;
  const [tab, setTab] = useState<"library" | "movement" | "looks" | "voices" | "music" | "chatter">(
    "library",
  );
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Selection | null>(null);
  const [auditionMessage, setAuditionMessage] = useState("");
  const session = useMemo(() => new AuditionSession(audition), [audition]);
  const auditionState = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  const { mix, stops } = auditionState;
  const playing = playingKey(auditionState);
  const loadingAudio = auditionState.phase === "loading";
  const nextOperation = useRef(0);
  const stopAudio = useCallback(() => {
    session.stop();
  }, [session]);
  const busy = soundWorkbenchBusy(state);
  const message =
    outcome?.kind === "error"
      ? outcome.message
      : outcome?.kind === "saved"
        ? "Saved. Reload or open a battle page to use this generation."
        : outcome?.kind === "reloaded"
          ? "Reloaded saved sources. Draft discarded."
          : auditionMessage;

  const startOperation = useCallback(
    (kind: "load" | "reload" | "preview" | "save", draftJSON?: string) => {
      const id = ++nextOperation.current;
      dispatch({ type: "operation-started", operation: { id, kind, draftJSON } });
      setAuditionMessage("");
      return id;
    },
    [],
  );

  const loadSnapshot = useCallback(
    async (
      kind: "load" | "reload" | "save",
      load: () => Promise<Snapshot>,
      active: () => boolean = () => true,
    ) => {
      const id = startOperation(kind);
      if (id === null) return;
      try {
        const value = await load();
        if (!active()) return;
        dispatch({ type: "snapshot-received", id, snapshot: value });
        if (nextOperation.current === id) stopAudio();
      } catch (error) {
        if (!active()) return;
        dispatch({
          type: "operation-failed",
          id,
          message: error instanceof Error ? error.message : String(error),
        });
      }
    },
    [startOperation, stopAudio],
  );

  useEffect(() => {
    let active = true;
    void loadSnapshot("load", api.snapshot, () => active);
    return () => {
      active = false;
      stopAudio();
    };
  }, [api, loadSnapshot, stopAudio]);

  useEffect(() => {
    if (!snapshot) return;
    setSelected((selection) => {
      if (
        selection &&
        (selection.kind === "clip"
          ? snapshot.catalog.clips[selection.id]
          : snapshot.catalog.sounds[selection.id])
      )
        return selection;
      const clip = Object.keys(snapshot.catalog.clips)[0];
      const sound = Object.keys(snapshot.catalog.sounds)[0];
      return clip ? { kind: "clip", id: clip } : sound ? { kind: "sound", id: sound } : null;
    });
  }, [snapshot]);

  const edit = (change: (catalog: SoundCatalog) => void) => {
    if (!draft) return;
    const next = structuredClone(draft);
    change(next);
    dispatch({ type: "catalog-edited", draft: next });
    setAuditionMessage("");
    stopAudio();
  };
  const editLooks = (change: (sounds: Record<string, LookSound>) => void) => {
    const next = structuredClone(lookSounds);
    change(next);
    if (!draft) return;
    dispatch({ type: "look-sounds-edited", lookSounds: next });
    setAuditionMessage("");
    stopAudio();
  };
  // One audition at a time: a later one, or a stop, supersedes an earlier
  // still preparing, which then never sounds or reports.
  const begin = async <T,>(
    key: string,
    open: (catalog: SoundCatalog) => Promise<T>,
    mixOf: (value: T) => LiveMix | null = () => null,
  ) => {
    if (!draft) return null;
    dispatch({ type: "outcome-cleared" });
    setAuditionMessage("");
    try {
      return await session.start(key, () => open(draft), mixOf);
    } catch (error) {
      setAuditionMessage(error instanceof Error ? error.message : String(error));
      return null;
    }
  };
  const play = async (kind: Selection["kind"], id: string, variant = 0, gain = 1) => {
    await begin(id, (catalog) => audition.play(catalog, kind, id, variant, gain));
  };
  const live = async (key: string, sounds: readonly string[]) => {
    return begin(
      key,
      (catalog) => audition.live(catalog, sounds),
      (value) => value,
    );
  };
  const previewChanges = async () => {
    if (!snapshot || !draft || busy) return;
    const captured = {
      revision: snapshot.revision,
      catalog: structuredClone(draft),
      lookSounds: structuredClone(lookSounds),
    };
    const id = startOperation("preview", JSON.stringify(captured));
    if (id === null) return;
    try {
      validateSoundCatalog(captured.catalog);
      const value = await api.preview(captured);
      dispatch({ type: "preview-received", id, value });
    } catch (error) {
      dispatch({
        type: "operation-failed",
        id,
        message: error instanceof Error ? error.message : String(error),
      });
    }
  };
  const saveReview = async () => {
    if (!soundWorkbenchCanSave(state) || !review) return;
    await loadSnapshot("save", () =>
      api.save({
        candidateId: review.value.candidateId,
        revision: review.value.revision,
      }),
    );
  };
  const reload = async () => {
    if (!snapshot || busy) return;
    await loadSnapshot("reload", api.snapshot);
  };
  const select = (selection: Selection) => {
    setSelected(selection);
    stopAudio();
  };
  const clone = (id: string) => {
    if (!draft) return;
    let name = `custom_${id}`;
    let index = 2;
    while (draft.sounds[name]) name = `custom_${id}_${index++}`;
    edit((c) => {
      c.sounds[name] = {
        ...structuredClone(c.sounds[id]),
        label: `${c.sounds[id].label} · custom`,
      };
    });
    setSelected({ kind: "sound", id: name });
  };
  const dirty = soundWorkbenchDirty(state);

  return (
    <main className="sound-workbench" data-sound-workbench>
      <header className="sw-header">
        <div>
          <Link to="/">Developer menu</Link>
          <h1>Sound workbench</h1>
          <p>Audition the library and edit each weapon look’s sound.</p>
        </div>
        {tab !== "voices" && tab !== "music" && tab !== "chatter" && (
          <div className="sw-publication">
            <span>{dirty ? "UNSAVED DRAFT" : "SAVED GENERATION"}</span>
            <button disabled={!draft || busy} onClick={() => void previewChanges()}>
              {busy ? "Working…" : "Preview changes"}
            </button>
            <button disabled={!snapshot || busy} onClick={() => void reload()}>
              Discard draft and reload
            </button>
          </div>
        )}
      </header>
      <nav className="sw-tabs" aria-label="Workbench views">
        {(
          [
            ["library", "Library & recipes"],
            ["movement", "Movement"],
            ["looks", "Weapon looks"],
            ["voices", "Voices"],
            ["music", "Music"],
            ["chatter", "Chatter preview"],
          ] as const
        ).map(([value, label]) => (
          <button
            key={value}
            aria-pressed={tab === value}
            onClick={() => {
              setTab(value);
              setSearch("");
              stopAudio();
            }}
          >
            {label}
          </button>
        ))}
        <button className="sw-stop" onClick={stopAudio}>
          Stop audition
        </button>
      </nav>
      {message && (
        <p role="status" className="sw-message">
          {message}
        </p>
      )}
      {loadingAudio && (
        <p role="status" className="sw-message">
          Preparing audition…
        </p>
      )}
      {playing && (
        <p className="sw-playing">
          Audition · {draft?.sounds[playing]?.label ?? draft?.clips[playing]?.label ?? playing}
        </p>
      )}
      {tab === "voices" ? (
        <VoicesTab api={voices} audition={audition} />
      ) : tab === "music" && snapshot ? (
        <MusicTab
          snapshot={snapshot}
          api={music}
          audition={audition}
          stop={stopAudio}
          stops={stops}
          reload={() => {
            if (!busy) void loadSnapshot("load", api.snapshot);
          }}
        />
      ) : tab === "chatter" ? (
        <ChatterPreview api={voices} audition={audition} />
      ) : !snapshot || !draft ? (
        <p>{message ? "The saved library could not be loaded." : "Loading saved library…"}</p>
      ) : (
        <>
          <fieldset className="sw-editable" disabled={busy}>
            {tab === "library" && (
              <Library
                snapshot={snapshot}
                lookSounds={lookSounds}
                draft={draft}
                edit={edit}
                play={play}
                search={search}
                setSearch={setSearch}
                selected={selected}
                select={select}
                clone={clone}
              />
            )}
            {tab === "movement" && (
              <Movement
                snapshot={snapshot}
                draft={draft}
                edit={edit}
                play={play}
                playing={playing}
                mix={mix}
                live={live}
                stop={stopAudio}
              />
            )}
            {tab === "looks" && (
              <WeaponLooks
                snapshot={snapshot}
                draft={draft}
                sounds={lookSounds}
                edit={editLooks}
                play={play}
                stop={stopAudio}
              />
            )}
          </fieldset>
          {review && (
            <section className="sw-review">
              <h2>Reviewed JSON</h2>
              <p>
                {review.value.files.length
                  ? "Save this exact candidate to the repository. Running battles keep their current sound generation."
                  : "No source changes. Saving preserves the original bytes."}
              </p>
              {review.value.files.map((file) => (
                <div key={file.path}>
                  <section aria-label="Changed sound settings" className="sw-changes">
                    <h3>Changed settings</h3>
                    {changedSettings(JSON.parse(file.before), JSON.parse(file.after)).map(
                      (change) => (
                        <div key={change.path} className="sw-change">
                          <code>{change.path}</code>
                          <div>
                            <span>{change.before}</span>
                            <b aria-label="changes to"> → </b>
                            <strong>{change.after}</strong>
                          </div>
                        </div>
                      ),
                    )}
                  </section>
                  <details>
                    <summary>{file.path} · exact before / after</summary>
                    <div className="sw-diff">
                      <div>
                        <h3>Before</h3>
                        <pre>{file.before}</pre>
                      </div>
                      <div>
                        <h3>After</h3>
                        <pre>{file.after}</pre>
                      </div>
                    </div>
                  </details>
                </div>
              ))}
              <button disabled={busy} onClick={() => void saveReview()}>
                Save reviewed JSON
              </button>
              <button onClick={() => dispatch({ type: "review-closed" })}>Close review</button>
            </section>
          )}
        </>
      )}
    </main>
  );
}
export default SoundWorkbench;
