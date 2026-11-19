/** The coach's card in the battle, a radio transmission from its speaker:
 *  their portrait, name and post, the step showing now (its keys filled in),
 *  and Continue where the step waits on the player. A step's line plays when
 *  it appears, and the signal bar runs while it speaks. The card holds what
 *  the coach heard since the open chapter opened (UI signals) and what the
 *  player dismissed; everything else it reads from the scenario's published
 *  progress, so a chapter restart re-derives it. */
import { useEffect, useRef, useState } from "react";
import { onUiSignal, type UiSignal } from "../input/uiSignals";
import type { ScenarioView } from "../sim/observationView";
import { cardParts, coachView, type CoachDocument } from "./coach";

export function CoachPanel({
  doc,
  scenario,
  speak,
}: {
  doc: CoachDocument;
  scenario: ScenarioView | null;
  /** Plays a narration line; its length in seconds, or null for none. */
  speak?: (speaker: string, line: string) => Promise<number | null>;
}) {
  const chapter = scenario?.chapter ?? null;
  const [heard, setHeard] = useState<{ chapter: string | null; signals: Set<UiSignal> }>({
    chapter,
    signals: new Set(),
  });
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());
  // A new chapter starts with nothing heard: its lessons are its own.
  const signals = heard.chapter === chapter ? heard.signals : new Set<UiSignal>();
  useEffect(
    () =>
      onUiSignal((signal) =>
        setHeard((current) =>
          current.chapter === chapter && current.signals.has(signal)
            ? current
            : {
                chapter,
                signals: new Set(current.chapter === chapter ? current.signals : []).add(signal),
              },
        ),
      ),
    [chapter],
  );
  const view = coachView(doc, { scenario, signals, dismissed });
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
    <aside className="hud-panel hud-coach" aria-label="Coach" data-occludes-readouts>
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
            onClick={() => setDismissed((current) => new Set(current).add(step.id))}
          >
            Continue
          </button>
        )}
      </div>
    </aside>
  );
}
