/** The step a scenario's coach shows now, from its published progress and
 *  what the player did since the open chapter opened (UI signals and
 *  dismissals). Nothing else accumulates, and a new run of the battle (a
 *  chapter restart) forgets that too, so the coach re-derives from the
 *  publication. */
import { useEffect, useState } from "react";
import { onUiSignal, type UiSignal } from "../input/uiSignals";
import type { ScenarioView } from "../sim/observationView";
import { coachView, type CoachDocument, type CoachView } from "./coach";

export interface Coach {
  view: CoachView | null;
  /** The player's Continue on a step. */
  dismiss(step: string): void;
}

export function useCoach(
  doc: CoachDocument | null,
  scenario: ScenarioView | null,
  /** The battle's run: anything that changes on a restart. */
  run: unknown,
): Coach {
  const chapter = scenario?.chapter ?? null;
  const [heard, setHeard] = useState<Heard<UiSignal>>({ run, chapter, items: new Set() });
  const [dismissed, setDismissed] = useState<Heard<string>>({
    run,
    chapter: null,
    items: new Set(),
  });
  // A new chapter starts with nothing heard: its lessons are its own.
  const signals = ofRun(heard, run, chapter);
  useEffect(
    () =>
      onUiSignal((signal) =>
        setHeard((current) => {
          const now = ofRun(current, run, chapter);
          return now.has(signal) ? current : { run, chapter, items: new Set(now).add(signal) };
        }),
      ),
    [run, chapter],
  );
  return {
    view: doc && coachView(doc, { scenario, signals, dismissed: ofRun(dismissed, run, null) }),
    dismiss: (step) =>
      setDismissed((current) => ({
        run,
        chapter: null,
        items: new Set(ofRun(current, run, null)).add(step),
      })),
  };
}

/** What the coach heard, in one run of the battle and one chapter. */
interface Heard<T> {
  run: unknown;
  chapter: string | null;
  items: Set<T>;
}
const ofRun = <T>(heard: Heard<T>, run: unknown, chapter: string | null): Set<T> =>
  heard.run === run && heard.chapter === chapter ? heard.items : new Set<T>();
