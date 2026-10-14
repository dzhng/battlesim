import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ALL_TRIGGERS,
  backgroundChoices,
  takeId,
  voiceTriggers,
  voiceUrl,
  type Intensity,
  type Trigger,
  type VoiceCatalog,
  type VoiceTake,
} from "../../../../packages/battle-audio/src/voices/voiceCatalog";
import type { Auditioner } from "../audition";
import type { GenerationJob } from "../protocol";
import {
  candidateUrl,
  type TakeView,
  type VoiceAPI,
  type VoiceScope,
  type VoiceSnapshot,
} from "./protocol";

const INTENSITIES: readonly Intensity[] = ["steady", "stressed"];
const TAKES = [0, 1] as const;
const JOB_POLL_MS = 1000;

const parse = (id: string): VoiceTake => {
  const [set, voice, trigger, intensity, take] = id.split("/");
  return {
    set,
    voice,
    trigger: trigger as Trigger,
    intensity: intensity as Intensity,
    take: Number(take),
  };
};
/** Every take of a set in grid order: voice, trigger, intensity, take. */
function setTakes(catalog: VoiceCatalog, set: string): string[] {
  return catalog.sets[set].voices.flatMap((voice) =>
    voiceTriggers(voice).flatMap((trigger) =>
      INTENSITIES.flatMap((intensity) =>
        TAKES.map((take) => takeId({ set, voice: voice.id, trigger, intensity, take })),
      ),
    ),
  );
}
/**
 * The review order for a set: every flagged take awaiting a verdict, then one
 * unreviewed take per voice and trigger, as a spot check of the rest.
 */
function reviewQueue(catalog: VoiceCatalog, takes: Record<string, TakeView>, set: string) {
  const ids = setTakes(catalog, set);
  const waiting = (id: string) => takes[id] && takes[id].verdict === null;
  const flagged = ids.filter((id) => waiting(id) && takes[id].match.flagged);
  const spot = new Map<string, string>();
  for (const id of ids) {
    const { voice, trigger } = parse(id);
    const cell = `${voice}/${trigger}`;
    if (!spot.has(cell) && waiting(id) && !takes[id].match.flagged) spot.set(cell, id);
  }
  return [...flagged, ...spot.values()];
}
const status = (take: TakeView | undefined) => (take ? (take.verdict ?? "unreviewed") : "missing");
const editable = (target: EventTarget | null) =>
  target instanceof HTMLElement &&
  (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

/**
 * Review voice takes one set at a time. The queue is keyboard-driven; verdicts
 * are recorded by the generation tool against the bytes that were heard.
 */
export function VoicesTab({ api, audition }: { api: VoiceAPI; audition: Auditioner }) {
  const [snapshot, setSnapshot] = useState<VoiceSnapshot | null>(null);
  const [set, setSet] = useState("");
  const [queue, setQueue] = useState<string[]>([]);
  const [current, setCurrent] = useState("");
  const [editing, setEditing] = useState<{ id: string; text: string } | null>(null);
  const [jobs, setJobs] = useState<GenerationJob[]>([]);
  const [message, setMessage] = useState("");
  const latest = useRef<VoiceSnapshot | null>(null);
  latest.current = snapshot;
  // Edits to voices.json run one after another, each on the latest saved generation.
  const edits = useRef<Promise<unknown>>(Promise.resolve());

  const report = useCallback((error: unknown) => {
    setMessage(error instanceof Error ? error.message : String(error));
  }, []);
  const reload = useCallback(async () => {
    try {
      setSnapshot(await api.snapshot());
    } catch (error) {
      report(error);
    }
  }, [api, report]);
  // The queue is fixed when a set opens, so judging a take never reshuffles it.
  const openSet = useCallback((id: string, value: VoiceSnapshot) => {
    const next = reviewQueue(value.catalog, value.takes, id);
    setSet(id);
    setQueue(next);
    setCurrent(next[0] ?? "");
    setEditing(null);
  }, []);
  useEffect(() => {
    let active = true;
    api.snapshot().then((value) => {
      if (!active) return;
      setSnapshot(value);
      const sets = Object.keys(value.catalog.sets).sort();
      openSet(sets.find((id) => value.counts[id]?.takes) ?? sets[0] ?? "", value);
    }, report);
    return () => {
      active = false;
    };
  }, [api, openSet, report]);

  const play = useCallback(
    (ids: readonly string[]) => {
      const takes = latest.current?.takes ?? {};
      const urls = ids.filter((id) => takes[id]).map((id) => voiceUrl(parse(id), takes[id].sha256));
      if (urls.length) audition.voice(urls).catch(report);
    },
    [audition, report],
  );
  const go = useCallback(
    (id: string | undefined) => {
      if (!id) return;
      setCurrent(id);
      setEditing(null);
      play([id]);
    },
    [play],
  );
  const step = useCallback(
    (by: 1 | -1) => {
      const at = queue.indexOf(current);
      go(at < 0 ? queue[0] : queue[at + by]);
    },
    [queue, current, go],
  );

  const judge = useCallback(
    async (verdict: "ok" | "bad", ids: readonly string[]) => {
      const takes = latest.current?.takes ?? {};
      const judged = ids.filter((id) => takes[id]);
      if (!judged.length) return;
      setSnapshot(
        (value) =>
          value && {
            ...value,
            takes: {
              ...value.takes,
              ...Object.fromEntries(judged.map((id) => [id, { ...value.takes[id], verdict }])),
            },
          },
      );
      try {
        const result = await api.verdict({
          verdict,
          takes: judged.map((id) => ({ take: id, sha256: takes[id].sha256 })),
        });
        setSnapshot(
          (value) =>
            value && {
              ...value,
              takes: { ...value.takes, ...result.takes },
              counts: { ...value.counts, ...result.counts },
            },
        );
        if (result.changed.length)
          setMessage(`Changed since you heard it, so not judged: ${result.changed.join(", ")}`);
      } catch (error) {
        report(error);
        await reload();
      }
    },
    [api, reload, report],
  );

  /** Saves a change to voices.json through review and publication, on the latest generation. */
  const publish = useCallback(
    (change: (catalog: VoiceCatalog) => void) => {
      const run = edits.current.then(async () => {
        const value = latest.current!;
        const catalog = structuredClone(value.catalog);
        change(catalog);
        const review = await api.preview({ revision: value.revision, catalog });
        const saved = await api.save({
          candidateId: review.candidateId,
          revision: review.revision,
        });
        latest.current = saved;
        setSnapshot(saved);
      });
      edits.current = run.catch(() => {});
      return run;
    },
    [api],
  );
  const generate = useCallback(
    async (scope: VoiceScope) => {
      const { job } = await api.generate(scope);
      setJobs((list) => [...list.filter((other) => other.id !== job.id), job]);
    },
    [api],
  );
  const regenerate = useCallback(
    async (id: string) => {
      const take = parse(id);
      try {
        await judge("bad", [id]);
        await publish((catalog) => {
          catalog.retakes[id] = (catalog.retakes[id] ?? 0) + 1;
        });
        await generate({
          sets: [take.set],
          voices: [take.voice],
          triggers: [take.trigger],
          intensities: [take.intensity],
          take: take.take,
        });
      } catch (error) {
        report(error);
      }
    },
    [judge, publish, generate, report],
  );

  const pick = useCallback(
    async (voice: string, candidate: number, replace: boolean) => {
      try {
        const saved = await api.pick(voice, candidate, replace);
        latest.current = saved;
        setSnapshot(saved);
        // A swap queues the voice's new takes and the set's pack: follow them.
        if (replace) setJobs((await api.jobs()).jobs);
      } catch (error) {
        report(error);
      }
    },
    [api, report],
  );
  /** Re-mixes the set onto another background; the tool re-renders and re-packs it. */
  const background = useCallback(
    async (set: string, processing: string) => {
      try {
        const saved = await api.background(latest.current!.revision, set, processing);
        latest.current = saved;
        setSnapshot(saved);
        setJobs((await api.jobs()).jobs);
      } catch (error) {
        report(error);
      }
    },
    [api, report],
  );
  /** None of these: bump the voice's design retake, then design new candidates. */
  const redesign = useCallback(
    async (voice: string) => {
      try {
        await publish((catalog) => {
          const key = `${voice}/design`;
          catalog.retakes[key] = (catalog.retakes[key] ?? 0) + 1;
        });
        const { job } = await api.design(voice);
        setJobs((list) => [...list.filter((other) => other.id !== job.id), job]);
      } catch (error) {
        report(error);
      }
    },
    [api, publish, report],
  );

  // While a job runs, follow its progress; when one ends, show what it made.
  const running = jobs.some((job) => job.status === "queued" || job.status === "running");
  useEffect(() => {
    if (!running) return;
    const timer = setInterval(() => {
      api
        .jobs()
        .then(({ jobs: next }) => {
          setJobs((before) => {
            const ended = next.some(
              (job) =>
                (job.status === "done" || job.status === "failed") &&
                before.some((old) => old.id === job.id && old.status !== job.status),
            );
            if (ended) void reload();
            return next;
          });
        })
        .catch(report);
    }, JOB_POLL_MS);
    return () => clearInterval(timer);
  }, [api, running, reload, report]);

  const keys = useRef<(event: KeyboardEvent) => void>(() => {});
  keys.current = (event) => {
    if (editable(event.target) || event.metaKey || event.ctrlKey || event.altKey) return;
    const key = event.key.toLowerCase();
    // Judging moves on through the queue; a take opened from the grid stays open.
    const next = () => (queue.includes(current) ? queue[queue.indexOf(current) + 1] : undefined);
    if (key === " ") {
      event.preventDefault();
      play([current]);
    } else if (key === "j") step(1);
    else if (key === "k") step(-1);
    else if ((key === "a" || key === "x") && current) {
      const after = next();
      void judge(key === "a" ? "ok" : "bad", [current]);
      go(after);
    } else if (key === "r" && current && latest.current?.takes[current]) {
      const after = next();
      void regenerate(current);
      go(after);
    } else if (key === "e" && current) {
      event.preventDefault();
      const take = parse(current);
      const catalog = latest.current!.catalog;
      const role = catalog.sets[take.set].role;
      setEditing({
        id: current,
        text: catalog.roles[role].lines[take.trigger][take.intensity][take.take],
      });
    }
  };
  useEffect(() => {
    const listener = (event: KeyboardEvent) => keys.current(event);
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);

  const sets = useMemo(
    () => (snapshot ? Object.keys(snapshot.catalog.sets).sort() : []),
    [snapshot],
  );
  if (!snapshot) return <p>{message || "Loading voice takes…"}</p>;
  const catalog = snapshot.catalog;
  const spec = catalog.sets[set];
  if (!spec) return <p>No voice sets.</p>;
  const ids = setTakes(catalog, set);
  const unflagged = ids.filter(
    (id) => snapshot.takes[id]?.verdict === null && !snapshot.takes[id].match.flagged,
  );
  const missing = ids.filter(
    (id) =>
      !snapshot.takes[id] && spec.voices.find((voice) => voice.id === parse(id).voice)?.voice_id,
  ).length;
  const shown = current ? parse(current) : null;
  const shownTake = current ? snapshot.takes[current] : undefined;
  const sharing = Object.values(catalog.sets).filter((other) => other.role === spec.role).length;
  const position = queue.indexOf(current);

  return (
    <div className="sw-layout sw-voices">
      <nav className="sw-browser" aria-label="Voice sets">
        {sets.map((id) => {
          const count = snapshot.counts[id];
          return (
            <button
              key={id}
              className="sw-library-row sw-voice-set"
              aria-pressed={id === set}
              data-empty={count.takes ? undefined : "true"}
              onClick={() => openSet(id, snapshot)}
            >
              <span className="sw-set-name">{id}</span>
              <small className="sw-set-counts">
                <span>
                  {count.takes}/{count.expected} takes
                </span>
                {count.unreviewed > 0 && <span>{count.unreviewed} unreviewed</span>}
                {count.flagged > 0 && <span data-tone="flagged">{count.flagged} flagged</span>}
                {count.bad > 0 && <span data-tone="bad">{count.bad} bad</span>}
              </small>
            </button>
          );
        })}
      </nav>
      <section className="sw-inspector">
        <header className="sw-voice-header">
          <div>
            <h2>{set}</h2>
            <p>
              {spec.country.toUpperCase()} · {spec.role} · “{spec.callsign}” ·{" "}
              {backgroundChoices(spec.processing).length > 1 ? (
                <select
                  aria-label="Background"
                  title="The bed this set's lines are mixed over"
                  value={spec.processing}
                  disabled={running}
                  onChange={(event) => {
                    const next = event.target.value;
                    if (
                      window.confirm(
                        `Mix ${set} over ${next}? Its takes are re-mixed (no new speech) and stay out of the pack until reviewed.`,
                      )
                    )
                      void background(set, next);
                  }}
                >
                  {backgroundChoices(spec.processing).map((profile) => (
                    <option key={profile} value={profile}>
                      {profile}
                    </option>
                  ))}
                </select>
              ) : (
                spec.processing
              )}
            </p>
          </div>
          <div className="sw-actions">
            <button
              title={`${unflagged.length} unreviewed takes that passed speech-to-text`}
              disabled={!unflagged.length}
              onClick={() => void judge("ok", unflagged)}
            >
              Accept unflagged in set
            </button>
            <button
              title="Takes of voices with a designed voice that the lock lacks"
              disabled={!missing || running}
              onClick={() => {
                if (
                  window.confirm(
                    `Generate ${missing} missing takes of ${set}? This spends credits.`,
                  )
                )
                  void generate({ sets: [set], only: "missing" }).catch(report);
              }}
            >
              Generate {missing} missing takes
            </button>
          </div>
        </header>
        {message && (
          <p role="status" className="sw-message">
            {message}
          </p>
        )}
        <section className="sw-review-take" aria-label="Current take">
          {shown ? (
            <>
              <div className="sw-review-line">
                <span className="sw-count">
                  {position >= 0 ? `QUEUE ${position + 1} / ${queue.length}` : "FROM GRID"}
                </span>
                <strong>
                  {shown.voice} · {shown.trigger} · {shown.intensity} take {shown.take + 1}
                </strong>
                <span className="sw-chip-state" data-verdict={status(shownTake)}>
                  {status(shownTake)}
                  {shownTake?.match.flagged ? " · flagged" : ""}
                </span>
              </div>
              {shownTake ? (
                <dl className="sw-take-facts">
                  <dt>Line</dt>
                  <dd>{shownTake.text}</dd>
                  <dt>Heard</dt>
                  <dd>{shownTake.transcript}</dd>
                  <dt>Check</dt>
                  <dd>
                    speech-to-text match {shownTake.match.score.toFixed(2)} ·{" "}
                    {shownTake.duration_s.toFixed(1)} s
                    {shownTake.match.missing.length
                      ? ` · missing ${shownTake.match.missing.join(", ")}`
                      : ""}
                    {shownTake.match.overlong ? " · overlong" : ""}
                  </dd>
                </dl>
              ) : (
                <p>Not generated yet.</p>
              )}
              {editing?.id === current && (
                <div className="sw-line-editor">
                  <label>
                    Line text
                    <textarea
                      aria-label="Line text"
                      autoFocus
                      value={editing.text}
                      onChange={(event) => setEditing({ id: current, text: event.target.value })}
                    />
                  </label>
                  <small>
                    Role {spec.role}, {shown.trigger} {shown.intensity} {shown.take + 1}: shared by{" "}
                    {sharing} {sharing === 1 ? "set" : "sets"}. {"{callsign}"} names the set.
                  </small>
                  <div className="sw-actions">
                    {(["Save text", "Save and regenerate"] as const).map((label) => (
                      <button
                        key={label}
                        onClick={() => {
                          const { id, text } = editing;
                          const take = parse(id);
                          setEditing(null);
                          void publish((draft) => {
                            draft.roles[spec.role].lines[take.trigger][take.intensity][take.take] =
                              text;
                          })
                            .then(() =>
                              label === "Save and regenerate"
                                ? generate({
                                    sets: [take.set],
                                    voices: [take.voice],
                                    triggers: [take.trigger],
                                    intensities: [take.intensity],
                                    take: take.take,
                                  })
                                : undefined,
                            )
                            .catch(report);
                        }}
                      >
                        {label}
                      </button>
                    ))}
                    <button onClick={() => setEditing(null)}>Cancel</button>
                  </div>
                </div>
              )}
            </>
          ) : (
            <p>Nothing waits for review in this set.</p>
          )}
          <p className="sw-keys">
            <kbd>Space</kbd> replay <kbd>J</kbd>/<kbd>K</kbd> next/previous <kbd>A</kbd> accept{" "}
            <kbd>X</kbd> bad <kbd>R</kbd> bad + regenerate <kbd>E</kbd> edit line
          </p>
        </section>
        {spec.voices
          .filter((voice) => voice.voice_id === null || snapshot.designs[`${set}/${voice.id}`])
          .map((voice) => {
            const id = `${set}/${voice.id}`;
            const design = snapshot.designs[id];
            // A picked voice keeps its candidates: the one in use, and the others to swap in.
            const picked = voice.voice_id !== null;
            return (
              <section
                key={id}
                className="sw-review-take sw-design"
                aria-label={`Candidates for ${voice.id}`}
              >
                <div className="sw-review-line">
                  <span className="sw-count">{picked ? "VOICE" : "DESIGN"}</span>
                  <strong>
                    {voice.id} <small>{voice.speaks}</small>
                  </strong>
                  {!picked && (
                    <button disabled={running} onClick={() => void redesign(id)}>
                      {design ? "None of these" : "Design"}
                    </button>
                  )}
                </div>
                <p className="sw-voice-description">{voice.description}</p>
                {design ? (
                  <ol className="sw-candidates">
                    {design.candidates.map((candidate, index) => (
                      <li key={index} title={`heard: ${candidate.transcript ?? "not transcribed"}`}>
                        <button
                          aria-label={`Play candidate ${index + 1}`}
                          onClick={() =>
                            audition.voice([candidateUrl(id, index + 1)]).catch(report)
                          }
                        >
                          ▶ {index + 1}
                        </button>
                        <span>
                          match {candidate.score?.toFixed(2) ?? "–"} ·{" "}
                          {candidate.duration_s.toFixed(1)} s
                        </span>
                        {!picked ? (
                          <button
                            aria-label={`Pick candidate ${index + 1}`}
                            onClick={() => void pick(id, index + 1, false)}
                          >
                            Pick
                          </button>
                        ) : design.picked === index + 1 ? (
                          <strong className="sw-in-use">In use</strong>
                        ) : (
                          <button
                            aria-label={`Use candidate ${index + 1}`}
                            onClick={() => {
                              const replace = `Use candidate ${index + 1} as ${id}? Its takes are made again with this voice and stay out of the pack until reviewed.`;
                              if (globalThis.confirm?.(replace) ?? true)
                                void pick(id, index + 1, true);
                            }}
                          >
                            Use this
                          </button>
                        )}
                      </li>
                    ))}
                  </ol>
                ) : (
                  <p>No candidates yet.</p>
                )}
              </section>
            );
          })}
        {jobs.length > 0 && (
          <section className="sw-jobs" aria-label="Generation jobs">
            {jobs
              .slice(-3)
              .reverse()
              .map((job) => (
                <details key={job.id} open={job.status === "running"}>
                  <summary>
                    generate {job.args.join(" ")} · {job.status}
                  </summary>
                  <pre>{job.lines.slice(-12).join("\n")}</pre>
                </details>
              ))}
          </section>
        )}
        <div className="sw-voice-grid-scroll">
          <table className="sw-voice-grid">
            <thead>
              <tr>
                <th scope="col">Trigger</th>
                {spec.voices.map((voice) => (
                  <th scope="col" key={voice.id}>
                    <div className="sw-voice-head">
                      <span>
                        {voice.id} <small>{voice.speaks}</small>
                      </span>
                      <button
                        aria-label={`Play row ${voice.id}`}
                        onClick={() =>
                          play(
                            voiceTriggers(voice).flatMap((trigger) => {
                              const cell = INTENSITIES.flatMap((intensity) =>
                                TAKES.map((take) =>
                                  takeId({ set, voice: voice.id, trigger, intensity, take }),
                                ),
                              ).filter((id) => snapshot.takes[id]);
                              return cell.slice(0, 1);
                            }),
                          )
                        }
                      >
                        ▶ Row
                      </button>
                    </div>
                    <small className="sw-voice-description" title={voice.description}>
                      {voice.voice_id ? voice.description : "waiting for a designed voice"}
                    </small>
                    <span className="sw-chip-legend" aria-hidden="true">
                      <span>steady</span>
                      <span>stressed</span>
                    </span>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {ALL_TRIGGERS.map((trigger) => (
                <tr key={trigger}>
                  <th scope="row">{trigger}</th>
                  {spec.voices.map((voice) => (
                    <td key={voice.id}>
                      {voiceTriggers(voice).includes(trigger) ? (
                        <div className="sw-chips">
                          {INTENSITIES.map((intensity) => (
                            <span
                              key={intensity}
                              className="sw-chip-pair"
                              data-intensity={intensity}
                            >
                              {TAKES.map((index) => {
                                const id = takeId({
                                  set,
                                  voice: voice.id,
                                  trigger,
                                  intensity,
                                  take: index,
                                });
                                const take = snapshot.takes[id];
                                const state = status(take);
                                return (
                                  <button
                                    key={index}
                                    className="sw-chip"
                                    aria-label={`${voice.id} ${trigger} ${intensity} ${index + 1}, ${state}${take?.match.flagged ? ", flagged" : ""}`}
                                    title={
                                      take
                                        ? `${take.text}\nheard: ${take.transcript}`
                                        : "not generated"
                                    }
                                    data-verdict={state}
                                    data-flagged={take?.match.flagged ? "true" : undefined}
                                    data-current={id === current ? "true" : undefined}
                                    onClick={() => go(id)}
                                  >
                                    {take ? index + 1 : ""}
                                  </button>
                                );
                              })}
                            </span>
                          ))}
                        </div>
                      ) : (
                        <span className="sw-muted">—</span>
                      )}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}
