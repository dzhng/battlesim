/** The coach over an authored scenario's battle played by hand: the step
 *  showing now (`useCoach`), the beats the player's side was told, and the
 *  camera tour a step flies. `CoachLayer` draws it; the battle view passes
 *  the tour to the viewport and the step's marks to the overlay. */
import { useEffect, useMemo, useRef } from "react";
import type { CoachDocument, CoachStep, CoachView } from "@web/battle/coach/coach";
import { useCoach } from "@web/battle/coach/useCoach";
import type { ScenarioView } from "@web/battle/sim/observationView";
import { sampleReel, type Reel } from "./capture/capture";
import type { ViewportPilot } from "./viewportRender";

export interface ScenarioCoach {
  doc: CoachDocument;
  view: CoachView | null;
  /** The beats the player's side was told, for which reactions are due. */
  told: ReadonlySet<string>;
  /** The battle's run: anything that changes on a restart. */
  run: unknown;
  /** The player's Continue on a step. */
  dismiss(step: string): void;
  /** The coach's camera, for the viewport's life. */
  tour: ViewportPilot;
}

/** The coach `doc` over the battle whose scenario progress is `scenario`;
 *  null without one. */
export function useScenarioCoach(
  doc: CoachDocument | undefined,
  scenario: ScenarioView | null,
  run: unknown,
): ScenarioCoach | null {
  const { view, dismiss } = useCoach(doc ?? null, scenario, run);
  const told = useMemo(() => new Set(scenario?.beats.map((b) => b.beat)), [scenario]);
  const tour = useCoachTour(view?.step ?? null);
  return doc ? { doc, view, told, run, dismiss, tour } : null;
}

/** The coach's camera: one pilot for the viewport's life (it is read once),
 *  which flies a step's tour from the first frame after the step appears and
 *  gives the camera back to the player where the tour ends. A tour never
 *  dips, so the reel's veil is never drawn. */
function useCoachTour(step: CoachStep | null): ViewportPilot {
  const flying = useRef<{ reel: Reel; start: number | null } | null>(null);
  useEffect(() => {
    flying.current = step?.camera ? { reel: step.camera, start: null } : null;
  }, [step]);
  return useMemo<ViewportPilot>(
    () => ({
      frameRate: true,
      pose(now) {
        const tour = flying.current;
        if (!tour) return null;
        tour.start ??= now;
        const { pose, done } = sampleReel(tour.reel, (now - tour.start) / 1000);
        if (done) flying.current = null;
        return done ? null : pose;
      },
    }),
    [],
  );
}
