import { useCallback, useEffect, useRef, useState } from "react";
import { mulberry32 } from "math/random";
import { isGenerated } from "../../../../packages/battle-audio/src/catalog";
import type { Mood } from "../../../../packages/battle-audio/src/music/musicCatalog";
import {
  MusicDirector,
  type BattleMood,
} from "../../../../packages/battle-audio/src/music/musicDirector";
import type { Auditioner } from "../audition";
import type { GenerationJob, MusicAPI, Snapshot } from "../protocol";

const JOB_POLL_MS = 1000;
const MOOD_POLL_MS = 1000;
const TITLES: Record<Mood, string> = { menu: "Menu", tension: "Tension", combat: "Combat" };
/** The battle score first, the menu bed after it. */
const SHOWN: readonly Mood[] = ["tension", "combat", "menu"];
type Player = Awaited<ReturnType<Auditioner["music"]>>;

/**
 * Audition the score. The mood switch drives the battle's own `MusicDirector` on a
 * clock this tab owns: Combat notes an under-fire bark, Tension skips the calm period.
 * The director's choice plays through the real `MusicPlayer`, crossfade included, and
 * a combat mood left alone falls back to tension after the calm period, as in battle.
 */
export function MusicTab({
  snapshot,
  api,
  audition,
  reload,
  stop,
  stops,
}: {
  snapshot: Snapshot;
  api: MusicAPI;
  audition: Auditioner;
  reload: () => void;
  /** The workbench's audition stop. */
  stop: () => void;
  /** Counts the workbench's audition stops; each one ends the player. */
  stops: number;
}) {
  const { catalog, music: direction } = snapshot;
  const [playing, setPlaying] = useState<{
    track: string;
    mood: BattleMood | null;
    /** The track it crossfaded from. */
    from: string | null;
  } | null>(null);
  const [jobs, setJobs] = useState<GenerationJob[]>([]);
  const [message, setMessage] = useState("");
  const session = useRef<{
    player: Player;
    director: MusicDirector;
    /** Seconds the switch has skipped ahead of the wall clock. */
    skipped: number;
    mood: BattleMood | null;
  } | null>(null);
  const report = useCallback((error: unknown) => {
    setMessage(error instanceof Error ? error.message : String(error));
  }, []);
  const now = () => performance.now() / 1000 + (session.current?.skipped ?? 0);

  /** The audition's player, opened on the first click so opening the tab stays silent. */
  const open = async () => {
    if (session.current) return session.current;
    const player = await audition.music(catalog.music.tracks);
    const rng = mulberry32.create(Date.now() >>> 0);
    const director = new MusicDirector(
      direction,
      { tension: catalog.music.moods.tension, combat: catalog.music.moods.combat },
      () => mulberry32.sample(rng),
    );
    return (session.current = { player, director, skipped: 0, mood: null });
  };
  const play = (track: string | null, mood: BattleMood | null) => {
    const current = session.current;
    if (!current || !track) return;
    current.mood = mood;
    current.player.crossfadeTo(track, direction.crossfade_s);
    setPlaying((before) => ({ track, mood, from: before?.track ?? null }));
  };
  /** Follows the director: a changed mood crossfades to that mood's next track. */
  const follow = () => {
    const current = session.current;
    if (!current) return;
    const mood = current.director.mood(now());
    if (mood !== current.mood) play(current.director.next(mood), mood);
  };
  const switchTo = (mood: BattleMood) =>
    open()
      .then((current) => {
        if (mood === "combat") current.director.note(direction.combat_triggers[0], now());
        // Tension returns once the calm period has passed without a bark: skip it.
        else current.skipped += direction.calm_s + 1;
        follow();
      })
      .catch(report);
  const playTrack = (track: string) =>
    open()
      .then(() => play(track, null))
      .catch(report);

  useEffect(() => {
    if (!playing) return;
    const timer = setInterval(() => {
      if (session.current?.mood) follow();
    }, MOOD_POLL_MS);
    return () => clearInterval(timer);
  });
  // The audition owns the player: a stop anywhere in the workbench ends this session.
  useEffect(() => {
    session.current = null;
    setPlaying(null);
  }, [stops]);

  const regenerate = (track: string) =>
    api
      .regenerate(track)
      .then(({ job }) => setJobs((list) => [...list.filter((other) => other.id !== job.id), job]))
      .catch(report);
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
            if (ended) reload();
            return next;
          });
        })
        .catch(report);
    }, JOB_POLL_MS);
    return () => clearInterval(timer);
  }, [api, running, reload, report]);

  return (
    <section className="sw-music">
      <div className="sw-mood-switch" role="group" aria-label="Battle mood">
        <span className="sw-eyebrow">BATTLE MOOD</span>
        {(["tension", "combat"] as const).map((mood) => (
          <button
            key={mood}
            aria-label={`${TITLES[mood]} mood`}
            aria-pressed={playing?.mood === mood}
            disabled={!catalog.music.moods[mood].length}
            onClick={() => void switchTo(mood)}
          >
            {TITLES[mood]}
          </button>
        ))}
        <button disabled={!playing} onClick={stop}>
          Stop
        </button>
        <p>
          {playing ? (
            <>
              Playing <strong>{playing.track}</strong>
              {playing.mood && ` · ${TITLES[playing.mood]}`}
              {playing.from &&
                playing.from !== playing.track &&
                ` · crossfaded from ${playing.from} over ${direction.crossfade_s} s`}
            </>
          ) : (
            "Silent until a mood or track is chosen."
          )}
          <small>
            As in battle: Combat is an own {direction.combat_triggers.join(", ")} bark; left alone,
            it returns to Tension after {direction.calm_s} s. Tension skips that wait.
          </small>
        </p>
      </div>
      {message && (
        <p role="status" className="sw-message">
          {message}
        </p>
      )}
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
      {SHOWN.map((mood) => (
        <section key={mood} className="sw-music-mood" aria-label={TITLES[mood]}>
          <h2>
            {TITLES[mood]}{" "}
            <small>
              {catalog.music.moods[mood].length}{" "}
              {catalog.music.moods[mood].length === 1 ? "track" : "tracks"}
            </small>
          </h2>
          <ul>
            {catalog.music.moods[mood].map((id) => {
              const track = catalog.music.tracks[id];
              const source = catalog.sources[track.source];
              const generated = isGenerated(source) ? source : null;
              return (
                <li key={id} className={playing?.track === id ? "sw-music-playing" : undefined}>
                  <div className="sw-music-track">
                    <strong>{id}</strong>
                    <span className="sw-count">
                      {generated
                        ? `${generated.length_s} s · ${generated.model} · ${generated.date}`
                        : "recording"}{" "}
                      · gain {track.gain}
                    </span>
                    <button aria-pressed={playing?.track === id} onClick={() => void playTrack(id)}>
                      {playing?.track === id ? "Playing" : "Play"}
                    </button>
                    {generated ? (
                      <button
                        disabled={running}
                        title="Buys a new generation of this track and replaces it"
                        onClick={() => void regenerate(id)}
                      >
                        Regenerate
                      </button>
                    ) : (
                      <span />
                    )}
                  </div>
                  {generated && (
                    <details>
                      <summary>Prompt</summary>
                      <p>{generated.prompt}</p>
                    </details>
                  )}
                </li>
              );
            })}
          </ul>
        </section>
      ))}
    </section>
  );
}
