/** The coach's card in the battle, a radio transmission from its speaker:
 *  their portrait, name and post, the step showing now (`useCoach`, its keys
 *  filled in), and Continue where the step waits on the player. As the step
 *  changes its lines play, one after another (`linesBetween`), and the
 *  signal bar runs while one speaks. */
import { useEffect, useRef, useState } from "react";
import {
  cardParts,
  linesBetween,
  type CoachDocument,
  type CoachStep,
  type CoachView,
} from "./coach";

export function CoachPanel({
  doc,
  view,
  told,
  onContinue,
  speak,
}: {
  doc: CoachDocument;
  view: CoachView | null;
  /** The beats the player's side was told, for which reactions are due. */
  told: ReadonlySet<string>;
  onContinue: (step: string) => void;
  /** Plays a narration line; its length in seconds, or null for none. */
  speak?: (speaker: string, line: string) => Promise<number | null>;
}) {
  const step = view?.step ?? null;
  const [speaking, setSpeaking] = useState(false);
  // The latest player, so a new one each render never replays the line.
  const voice = useRef(speak);
  useEffect(() => {
    voice.current = speak;
  });
  const canSpeak = speak !== undefined;
  // The step left behind, and the beats told then, read as the step changes.
  const left = useRef<CoachStep | null>(null);
  const toldRef = useRef(told);
  useEffect(() => {
    toldRef.current = told;
  });
  // Each step change speaks once (or as sound starts): the left step's
  // reaction, then the new step's line, each after the last ends.
  useEffect(() => {
    const say = voice.current;
    const lines = linesBetween(left.current, step, toldRef.current);
    left.current = step;
    if (!say || lines.length === 0) return;
    let live = true;
    let quiet: ReturnType<typeof setTimeout> | undefined;
    const next = (rest: string[]) => {
      const [line, ...after] = rest;
      if (!line || !live) return;
      void say(doc.speaker.id, line).then((seconds) => {
        if (!live || seconds === null) return next(after);
        setSpeaking(true);
        quiet = setTimeout(() => {
          setSpeaking(false);
          next(after);
        }, seconds * 1000);
      });
    };
    next(lines);
    return () => {
      live = false;
      clearTimeout(quiet);
      setSpeaking(false);
    };
  }, [step, canSpeak, doc.speaker.id]);
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
              <span key={i} className="hud-coach-keys">
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
