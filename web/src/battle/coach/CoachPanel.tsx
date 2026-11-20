/** The coach's card in the battle, a radio transmission from its speaker:
 *  their portrait, name and post, the step showing now (`useCoach`, its keys
 *  filled in), and Continue where the step waits on the player. As the step
 *  changes its lines play (`linesBetween`, `useCoachVoice`), and the signal
 *  bar runs while one speaks. */
import { useEffect, useRef, useState } from "react";
import {
  cardParts,
  linesBetween,
  type CoachDocument,
  type CoachStep,
  type CoachView,
} from "./coach";

/** Plays a narration line; its length in seconds, or null for none. */
type Speak = (speaker: string, line: string) => Promise<number | null>;

export function CoachPanel({
  doc,
  view,
  told,
  run,
  onContinue,
  speak,
}: {
  doc: CoachDocument;
  view: CoachView | null;
  /** The beats the player's side was told, for which reactions are due. */
  told: ReadonlySet<string>;
  /** The battle's run: a restart silences what the last one queued. */
  run: unknown;
  onContinue: (step: string) => void;
  speak?: Speak;
}) {
  const step = view?.step ?? null;
  const speaking = useCoachVoice(doc.speaker.id, step, told, run, speak);
  if (!view || !step) return null;
  return (
    <aside
      className="hud-panel hud-coach"
      aria-label="Coach"
      data-step={step.id}
      data-occludes-readouts
    >
      <span className="hud-coach-portrait">
        <img src={doc.speaker.portrait} alt="" />
      </span>
      <div className="hud-coach-body">
        <header className="hud-coach-head">
          <span className="hud-coach-name">
            {doc.speaker.name} · {doc.speaker.post}
          </span>
          <span className="hud-coach-chapter">{view.chapter.title}</span>
          <span className="hud-coach-signal" data-speaking={speaking || undefined} aria-hidden>
            <i />
            <i />
            <i />
            <i />
            <i />
          </span>
        </header>
        <p className="hud-coach-card">
          {cardParts(step.card).map((part, i) =>
            typeof part === "string" ? (
              part
            ) : (
              <span key={i} className="hud-coach-keys" data-keys={part.name}>
                {part.keys.map((key) => (
                  <kbd key={key}>{key}</kbd>
                ))}
              </span>
            ),
          )}
        </p>
        {step.until === "continue" && (
          <button
            type="button"
            className="hud-menu-item hud-coach-continue"
            onClick={() => onContinue(step.id)}
          >
            Continue
          </button>
        )}
      </div>
    </aside>
  );
}

/** A line waiting to be spoken. A step's own line (`gives`) gives way to the
 *  next step's; a reaction is heard to its end. */
interface Line {
  line: string;
  gives: boolean;
}

/** The coach's voice, one speaker for a run of the battle: each step change
 *  queues its lines (`linesBetween`), which play in order, each after the
 *  last ends. A step change drops the step lines still waiting, and cuts a
 *  step's line short when the next has something to say. A new run, or the
 *  panel leaving, silences it. True while a line plays. */
function useCoachVoice(
  speaker: string,
  step: CoachStep | null,
  told: ReadonlySet<string>,
  run: unknown,
  speak: Speak | undefined,
): boolean {
  const [speaking, setSpeaking] = useState(false);
  // The latest player and beats, read as a line starts or the step changes.
  const voice = useRef(speak);
  const toldRef = useRef(told);
  useEffect(() => {
    voice.current = speak;
    toldRef.current = told;
  });
  const queue = useRef<{ add(lines: Line[]): void } | null>(null);
  useEffect(() => {
    const waiting: Line[] = [];
    let playing: Line | null = null;
    let quiet: ReturnType<typeof setTimeout> | undefined;
    let stopped = false;
    // Each line said carries its turn: a line cut short ends nothing.
    let turn = 0;
    const next = () => {
      clearTimeout(quiet);
      setSpeaking(false);
      const say = voice.current;
      playing = waiting.shift() ?? null;
      if (!playing) return;
      if (!say) return next();
      const mine = ++turn;
      void say(speaker, playing.line).then((seconds) => {
        if (stopped || mine !== turn) return;
        if (seconds === null) return next();
        setSpeaking(true);
        quiet = setTimeout(next, seconds * 1000);
      });
    };
    queue.current = {
      add(lines) {
        const kept = waiting.filter((line) => !line.gives);
        waiting.splice(0, waiting.length, ...kept, ...lines);
        if (!playing || (playing.gives && lines.length)) next();
      },
    };
    return () => {
      stopped = true;
      clearTimeout(quiet);
      setSpeaking(false);
      queue.current = null;
    };
  }, [speaker, run]);
  const left = useRef<CoachStep | null>(null);
  useEffect(() => {
    const lines = linesBetween(left.current, step, toldRef.current);
    left.current = step;
    queue.current?.add(lines.map((line) => ({ line, gives: line === step?.line && !step.reacts })));
  }, [step]);
  return speaking;
}
