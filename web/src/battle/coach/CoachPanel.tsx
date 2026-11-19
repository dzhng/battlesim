/** The coach's card in the battle, a radio transmission from its speaker:
 *  their portrait, name and post, the step showing now (`useCoach`, its keys
 *  filled in), and Continue where the step waits on the player. A step's
 *  line plays when it appears, and the signal bar runs while it speaks. */
import { useEffect, useRef, useState } from "react";
import { cardParts, type CoachDocument, type CoachView } from "./coach";

export function CoachPanel({
  doc,
  view,
  onContinue,
  speak,
}: {
  doc: CoachDocument;
  view: CoachView | null;
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
  // A step's line plays once, when the step appears (or sound starts).
  useEffect(() => {
    const say = voice.current;
    if (!step?.line || !say) return;
    let live = true;
    let quiet: ReturnType<typeof setTimeout> | undefined;
    void say(doc.speaker.id, step.line).then((seconds) => {
      if (!live || seconds === null) return;
      setSpeaking(true);
      quiet = setTimeout(() => setSpeaking(false), seconds * 1000);
    });
    return () => {
      live = false;
      clearTimeout(quiet);
      setSpeaking(false);
    };
  }, [step?.id, step?.line, canSpeak, doc.speaker.id]);
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
