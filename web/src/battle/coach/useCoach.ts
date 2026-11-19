/** The step a scenario's coach shows now, from its published progress and
 *  what the player did since the open chapter opened (UI signals and
 *  dismissals). Nothing else accumulates, so a chapter restart re-derives
 *  it. */
import { useEffect, useState } from "react";
import { onUiSignal, type UiSignal } from "../input/uiSignals";
import type { ScenarioView } from "../sim/observationView";
import { coachView, type CoachDocument, type CoachView } from "./coach";

export interface Coach {
  view: CoachView | null;
  /** The player's Continue on a step. */
  dismiss(step: string): void;
}

export function useCoach(doc: CoachDocument | null, scenario: ScenarioView | null): Coach {
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
  return {
    view: doc && coachView(doc, { scenario, signals, dismissed }),
    dismiss: (step) => setDismissed((current) => new Set(current).add(step)),
  };
}
