import { Link } from "react-router";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  validateSoundCatalog,
  type SoundCatalog,
} from "../../../packages/battle-audio/src/catalog";
import { SoundAuditioner, type Auditioner, type LiveMix } from "./audition";
import { workbenchAPI, type Review, type Snapshot, type WorkbenchAPI } from "./protocol";
import { Library, type Selection } from "./Library";
import { GlobalAssignments } from "./Assignments";
import { Movement } from "./Movement";
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

export function SoundWorkbench({
  api = workbenchAPI,
  audition: suppliedAudition,
}: { api?: WorkbenchAPI; audition?: Auditioner } = {}) {
  const audition = useMemo(() => suppliedAudition ?? new SoundAuditioner(), [suppliedAudition]);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [draft, setDraft] = useState<SoundCatalog | null>(null);
  const [review, setReview] = useState<Review | null>(null);
  const [tab, setTab] = useState<"library" | "movement" | "effects">("library");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Selection | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [playing, setPlaying] = useState("");
  const [mix, setMix] = useState<LiveMix | null>(null);
  const [loadingAudio, setLoadingAudio] = useState(false);
  const auditionGeneration = useRef(0);
  const stopAudio = useCallback(() => {
    auditionGeneration.current++;
    audition.stop();
    setPlaying("");
    setMix(null);
    setLoadingAudio(false);
  }, [audition]);
  const accept = useCallback(
    (value: Snapshot) => {
      stopAudio();
      setSnapshot(value);
      setDraft(structuredClone(value.catalog));
      setReview(null);
      setSelected((selection) => {
        if (
          selection &&
          (selection.kind === "clip"
            ? value.catalog.clips[selection.id]
            : value.catalog.sounds[selection.id])
        )
          return selection;
        const clip = Object.keys(value.catalog.clips)[0];
        const sound = Object.keys(value.catalog.sounds)[0];
        return clip ? { kind: "clip", id: clip } : sound ? { kind: "sound", id: sound } : null;
      });
    },
    [stopAudio],
  );

  useEffect(() => {
    let active = true;
    void api
      .snapshot()
      .then((value) => {
        if (active) accept(value);
      })
      .catch((error) => {
        if (active) setMessage(String(error.message));
      });
    return () => {
      active = false;
      stopAudio();
    };
  }, [api, accept, stopAudio]);

  const edit = (change: (catalog: SoundCatalog) => void) => {
    if (!draft) return;
    const next = structuredClone(draft);
    change(next);
    setDraft(next);
    setReview(null);
    setMessage("");
    stopAudio();
  };
  // One audition at a time: a later one, or a stop, supersedes an earlier
  // still preparing, which then never sounds or reports.
  const begin = async <T,>(key: string, open: (catalog: SoundCatalog) => Promise<T>) => {
    if (!draft) return null;
    const generation = ++auditionGeneration.current;
    setLoadingAudio(true);
    setPlaying("");
    setMix(null);
    setMessage("");
    try {
      const opened = await open(draft);
      if (generation !== auditionGeneration.current) return null;
      setPlaying(key);
      return opened;
    } catch (error) {
      if (generation === auditionGeneration.current)
        setMessage(error instanceof Error ? error.message : String(error));
      return null;
    } finally {
      if (generation === auditionGeneration.current) setLoadingAudio(false);
    }
  };
  const play = async (kind: Selection["kind"], id: string, variant = 0, gain = 1) => {
    await begin(id, (catalog) => audition.play(catalog, kind, id, variant, gain));
  };
  const live = async (key: string, sounds: readonly string[]) => {
    const opened = await begin(key, (catalog) => audition.live(catalog, sounds));
    if (opened) setMix(opened);
    return opened;
  };
  const action = async (run: () => Promise<void>) => {
    setBusy(true);
    setMessage("");
    try {
      await run();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
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
  const dirty = snapshot && draft && JSON.stringify(snapshot.catalog) !== JSON.stringify(draft);

  return (
    <main className="sound-workbench" data-sound-workbench>
      <header className="sw-header">
        <div>
          <Link to="/">Developer menu</Link>
          <h1>Sound workbench</h1>
          <p>Audition the library. Choose the sound of each unit and weapon mount.</p>
        </div>
        <div className="sw-publication">
          <span>{dirty ? "UNSAVED DRAFT" : "SAVED GENERATION"}</span>
          <button
            disabled={!draft || busy}
            onClick={() =>
              void action(async () => {
                validateSoundCatalog(draft);
                setReview(await api.preview({ revision: snapshot!.revision, catalog: draft! }));
              })
            }
          >
            {busy ? "Working…" : "Preview changes"}
          </button>
          <button
            disabled={!snapshot || busy}
            onClick={() =>
              void action(async () => {
                accept(await api.snapshot());
                setMessage("Reloaded saved sources. Draft discarded.");
              })
            }
          >
            Discard draft and reload
          </button>
        </div>
      </header>
      <nav className="sw-tabs" aria-label="Workbench views">
        {(
          [
            ["library", "Library & recipes"],
            ["movement", "Movement"],
            ["effects", "Defaults & effects"],
          ] as const
        ).map(([value, label]) => (
          <button
            key={value}
            aria-pressed={tab === value}
            onClick={() => {
              setTab(value);
              setSearch("");
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
      {!snapshot || !draft ? (
        <p>{message ? "The saved library could not be loaded." : "Loading saved library…"}</p>
      ) : (
        <>
          <fieldset className="sw-editable" disabled={busy}>
            {tab === "library" && (
              <Library
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
            {tab === "effects" && (
              <GlobalAssignments snapshot={snapshot} draft={draft} edit={edit} play={play} />
            )}
          </fieldset>
          {review && (
            <section className="sw-review">
              <h2>Reviewed JSON</h2>
              <p>
                {review.files.length
                  ? "Save this exact candidate to the repository. Running battles keep their current sound generation."
                  : "No source changes. Saving preserves the original bytes."}
              </p>
              {review.files.map((file) => (
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
              <button
                disabled={busy}
                onClick={() =>
                  void action(async () => {
                    accept(
                      await api.save({
                        candidateId: review.candidateId,
                        revision: review.revision,
                      }),
                    );
                    setMessage("Saved. Reload or open a battle page to use this generation.");
                  })
                }
              >
                Save reviewed JSON
              </button>
              <button onClick={() => setReview(null)}>Close review</button>
            </section>
          )}
        </>
      )}
    </main>
  );
}
export default SoundWorkbench;
