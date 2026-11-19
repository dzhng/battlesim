/** The coach's card in the battle: the step showing now, its keys filled
 *  in, and Continue where the step waits on the player. It holds what the
 *  coach heard since the open chapter opened (UI signals) and what the
 *  player dismissed; everything else it reads from the scenario's published
 *  progress, so a chapter restart re-derives it. */
import { useEffect, useState } from "react";
import { onUiSignal, type UiSignal } from "../input/uiSignals";
import type { ScenarioView } from "../sim/observationView";
import { cardText, coachView, type CoachDocument } from "./coach";

export function CoachPanel({
  doc,
  scenario,
}: {
  doc: CoachDocument;
  scenario: ScenarioView | null;
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
  if (!view) return null;
  const { step } = view;
  return (
    <aside className="hud-panel hud-coach" aria-label="Coach" data-occludes-readouts>
      <span className="hud-coach-chapter">{view.chapter.title}</span>
      <p className="hud-coach-card">{cardText(step.card)}</p>
      {step.until === "continue" && (
        <button
          type="button"
          className="hud-menu-item hud-coach-continue"
          onClick={() => setDismissed((current) => new Set(current).add(step.id))}
        >
          Continue
        </button>
      )}
    </aside>
  );
}
