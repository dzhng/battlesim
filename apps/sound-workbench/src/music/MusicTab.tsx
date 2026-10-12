import { useCallback, useEffect, useRef, useState } from "react";
import { isGenerated } from "../../../../packages/battle-audio/src/catalog";
import type { Mood } from "../../../../packages/battle-audio/src/music/musicCatalog";
import {
  BattleMusic,
  type BattleMood,
} from "../../../../packages/battle-audio/src/music/battleMusic";
import type { VoiceRequest } from "../../../../packages/battle-audio/src/voices/chatter";
import { loadPresentation } from "../../../../web/src/battle/sim/presentation";
import type { Auditioner } from "../audition";
import type { GenerationJob, MusicAPI, Snapshot } from "../protocol";

const JOB_POLL_MS = 1000;
const MOOD_POLL_MS = 1000;
const TITLES: Record<Mood, string> = { menu: "Menu", tension: "Tension", combat: "Combat" };
/** The battle score first, the menu bed after it. */
const SHOWN: readonly Mood[] = ["tension", "combat", "menu"];
type Player = Awaited<ReturnType<Auditioner["music"]>>;

/**
 * Audition the score. The mood switch drives the battle's own score (Rust's, through
 * `BattleMusic`) on a clock this tab owns: Combat hears an own under-fire bark, Tension
 * skips the calm period. The score's choice plays through the real `MusicPlayer`,
 * crossfade included, and a combat mood left alone falls back to tension after the
 * calm period, as in battle.
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
    /** The battle's score; a track played by hand ends it, and the next switch opens a fresh one. */
    score: BattleMusic | null;
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
    const [player] = await Promise.all([audition.music(catalog.music.tracks), loadPresentation()]);
    return (session.current = { player, score: null, skipped: 0, mood: null });
  };
  const moodOf = (track: string): BattleMood =>
    catalog.music.moods.combat.includes(track) ? "combat" : "tension";
  /** The score's next commands at the tab's clock: a start crossfades to that track. */
  const hear = (barks: VoiceRequest[]) => {
    const current = session.current;
    if (!current) return;
    current.score ??= new BattleMusic(
      direction,
      { tension: catalog.music.moods.tension, combat: catalog.music.moods.combat },
      { play: (track) => play(track, moodOf(track)), stop: () => {}, duck: () => {} },
      Date.now(),
    );
    current.score.update(now(), barks, false);
  };
  const play = (track: string | null, mood: BattleMood | null) => {
    const current = session.current;
    if (!current || !track) return;
    current.mood = mood;
    current.player.crossfadeTo(track, direction.crossfade_s);
    setPlaying((before) => ({ track, mood, from: before?.track ?? null }));
  };
  const switchTo = (mood: BattleMood) =>
    open()
      .then((current) => {
        if (mood === "combat") {
          const trigger = direction.combat_triggers[0] as VoiceRequest["trigger"];
          const at = now();
          hear([
            { tier: "own_bark", speaker: "workbench", set: "", trigger, intensity: "stressed", at },
          ]);
        } else {
          // Tension returns once the calm period has passed without a bark: skip it.
          current.skipped += direction.calm_s + 1;
          hear([]);
        }
      })
      .catch(report);
  const playTrack = (track: string) =>
    open()
      .then((current) => {
        current.score?.dispose();
        current.score = null;
        play(track, null);
      })
      .catch(report);

  useEffect(() => {
    if (!playing) return;
    const timer = setInterval(() => {
      if (session.current?.mood) hear([]);
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
