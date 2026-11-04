import { audioMix } from "@web/audioMix";
import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import type { AudioMix } from "../../../../packages/battle-audio/src/audioMix";
import {
  Chatter,
  type DropReason,
  type VoiceTier,
} from "../../../../packages/battle-audio/src/voices/chatter";
import { loadPresentation } from "../../../../web/src/battle/sim/presentation";
import { VoiceBank } from "../../../../packages/battle-audio/src/voices/voiceBank";
import { voicePack } from "../../../../packages/battle-audio/src/voices/voiceCatalog";
import type { Auditioner } from "../audition";
import type { VoiceAPI } from "./protocol";
import { CHATTER_SCRIPT, LISTENER, SCRIPT_S, SCRIPT_SETS, SCRIPT_SPEAKERS } from "./chatterScript";

const RULES = (audioMix as unknown as AudioMix).voices;
/** The preview's take seed: fixed, so a replay is the same performance. */
const SEED = 1;
const TICK_MS = 40;
/** Seconds the preview runs past the script, so the last lines finish. */
const TAIL_S = 3;

interface Event {
  speaker: string;
  tier: VoiceTier;
  trigger: string;
  /** Script seconds. */
  at: number;
  /** Started: how long it sounds, the voice and take. */
  started?: { duration: number; voice: string; take: number };
  dropped?: DropReason;
}
const REASON: Record<DropReason, string> = { late: "late", group: "group", missing: "no take" };
/** Drops of one reason closer than this, in seconds, share one mark with their count. */
const CROWDED_S = 0.4;

/** A lane's drops, with drops of one reason close together merged into one mark. */
function dropMarks(events: readonly Event[]) {
  const marks: { at: number; reason: DropReason; triggers: string[] }[] = [];
  for (const event of events) {
    if (!event.dropped) continue;
    const last = marks.at(-1);
    if (last && last.reason === event.dropped && event.at - last.at < CROWDED_S)
      last.triggers.push(event.trigger);
    else marks.push({ at: event.at, reason: event.dropped, triggers: [event.trigger] });
  }
  return marks;
}

const TIER: Record<VoiceTier, string> = {
  response: "response",
  own_bark: "own bark",
  other_bark: "seen enemy",
};

/**
 * Plays the scripted firefight through the battle's own line arbitration
 * (`Chatter`, in Rust) and `VoiceBank`, with every locked take not judged bad, and draws which lines
 * started and which were dropped.
 */
export function ChatterPreview({ api, audition }: { api: VoiceAPI; audition: Auditioner }) {
  const [events, setEvents] = useState<Event[]>([]);
  const [clock, setClock] = useState<number | null>(null);
  const [status, setStatus] = useState("");
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);
  const halt = useCallback(() => {
    if (timer.current) clearInterval(timer.current);
    timer.current = null;
  }, []);
  useEffect(() => halt, [halt]);

  const play = async () => {
    halt();
    setEvents([]);
    setClock(null);
    setStatus("Loading takes…");
    try {
      const snapshot = await api.snapshot();
      const packs = (set: string) =>
        Promise.resolve(
          voicePack(snapshot.catalog, set, (id) => {
            const take = snapshot.takes[id];
            return take && take.verdict !== "bad" ? take : null;
          }),
        );
      await loadPresentation();
      const chatter = new Chatter(RULES, SEED);
      const opened = await audition.lines(async (context, signal) => {
        const bank = new VoiceBank(context, fetch, packs, { listener: chatter });
        await bank.load(SCRIPT_SETS, signal);
        return bank;
      });
      if (!opened) return setStatus("");
      const { prepared: bank, sink, current } = opened;
      setStatus(
        bank.errors.size ? `Some takes failed to load: ${[...bank.errors.keys()].join(", ")}` : "",
      );
      let next = 0;
      timer.current = setInterval(() => {
        const now = sink.now();
        if (!current() || now > SCRIPT_S + TAIL_S) {
          halt();
          if (current()) setClock(null);
          return;
        }
        while (next < CHATTER_SCRIPT.length && CHATTER_SCRIPT[next].t <= now) {
          const { t, ...request } = CHATTER_SCRIPT[next++];
          chatter.request({ ...request, at: t }, now);
        }
        const { commands, started, dropped } = chatter.update(now, LISTENER);
        for (const line of commands)
          bank.play(
            line.sound,
            line.variant,
            line.at,
            RULES.wait_s,
            () => sink.now(),
            (buffer, at) => sink.play(buffer, at),
          );
        if (started.length || dropped.length)
          setEvents((list) => [
            ...list,
            ...started.map(({ request, voice, take, duration, at }) => ({
              speaker: request.speaker,
              tier: request.tier,
              trigger: request.trigger,
              at,
              started: { duration, voice, take },
            })),
            ...dropped.map(({ request, reason, at }) => ({
              speaker: request.speaker,
              tier: request.tier,
              trigger: request.trigger,
              at,
              dropped: reason,
            })),
          ]);
        setClock(now);
      }, TICK_MS);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  };

  const span = SCRIPT_S + TAIL_S;
  const x = (seconds: number) => `${(100 * seconds) / span}%`;
  const startedCount = events.filter((event) => event.started).length;
  const droppedBy = (reason: DropReason) =>
    events.filter((event) => event.dropped === reason).length;

  return (
    <section className="sw-chatter" aria-label="Chatter preview">
      <header className="sw-voice-header">
        <div>
          <h2>Chatter preview</h2>
          <p>
            A scripted 30 s firefight through the battle's own line arbitration: up to {RULES.lines}{" "}
            lines at once, answers before own shouts before a seen enemy's, nearest first, dropped
            after {RULES.wait_s} s of waiting, one answer per group. Plays every take not judged
            bad.
          </p>
        </div>
        <div className="sw-actions">
          <button onClick={() => void play()}>▶ Play preview</button>
        </div>
      </header>
      {status && (
        <p role="status" className="sw-message">
          {status}
        </p>
      )}
      <div className="sw-chatter-summary" aria-live="polite">
        <span className="sw-chatter-key">
          <i className="sw-key-ask" /> asked
          <i className="sw-key-line" /> heard (take length) · ✕<sup>2</sup> at once
        </span>
        <span>
          <b>{startedCount}</b> started
        </span>
        {(["late", "group", "missing"] as const).map((reason) => (
          <span key={reason} data-reason={reason}>
            <i className="sw-drop-mark">✕</i> <b>{droppedBy(reason)}</b> dropped · {REASON[reason]}
          </span>
        ))}
        <span className="sw-chatter-legend">
          {(Object.keys(TIER) as VoiceTier[]).map((tier) => (
            <span key={tier} data-tier={tier}>
              {TIER[tier]}
            </span>
          ))}
        </span>
      </div>
      <div
        className="sw-lanes"
        role="table"
        aria-label="Lines by speaker"
        style={{ "--sw-grid": x(5) } as CSSProperties}
      >
        <div className="sw-lane sw-lane-axis" role="row">
          <span className="sw-lane-name" role="rowheader">
            <small>speaker · voice set</small>
          </span>
          <div className="sw-lane-track">
            {Array.from({ length: Math.floor(span / 5) + 1 }, (_, i) => (
              <span key={i} className="sw-lane-tick" style={{ left: x(i * 5) }}>
                {i * 5} s
              </span>
            ))}
          </div>
        </div>
        {SCRIPT_SPEAKERS.map((speaker) => {
          const script = CHATTER_SCRIPT.find((entry) => entry.speaker === speaker)!;
          return (
            <div key={speaker} className="sw-lane" role="row">
              <span className="sw-lane-name" role="rowheader">
                {speaker}
                <small>{script.set}</small>
              </span>
              <div className="sw-lane-track" role="cell">
                {CHATTER_SCRIPT.filter((entry) => entry.speaker === speaker).map((entry, i) => (
                  <span
                    key={`asked-${i}`}
                    className="sw-lane-ask"
                    data-tier={entry.tier}
                    style={{ left: x(entry.t) }}
                    title={`asks ${entry.trigger} at ${entry.t.toFixed(1)} s`}
                  />
                ))}
                {events
                  .filter((event) => event.speaker === speaker && event.started)
                  .map((event, i) => (
                    <span
                      key={i}
                      className="sw-lane-line"
                      data-tier={event.tier}
                      style={{ left: x(event.at), width: x(event.started!.duration) }}
                      title={`${event.trigger} · ${event.started!.voice} take ${event.started!.take + 1} · ${event.at.toFixed(1)} s`}
                    >
                      {event.trigger}
                    </span>
                  ))}
                {dropMarks(events.filter((event) => event.speaker === speaker)).map((mark, i) => (
                  <span
                    key={`drop-${i}`}
                    className="sw-lane-drop"
                    data-reason={mark.reason}
                    style={{ left: x(mark.at) }}
                    title={`${mark.triggers.join(", ")} dropped: ${REASON[mark.reason]} · ${mark.at.toFixed(1)} s`}
                  >
                    ✕{mark.triggers.length > 1 && <sup>{mark.triggers.length}</sup>}
                  </span>
                ))}
                {clock !== null && <span className="sw-lane-playhead" style={{ left: x(clock) }} />}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
