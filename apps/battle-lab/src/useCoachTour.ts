import { useEffect, useMemo, useRef } from "react";
import type { CoachStep } from "@web/battle/coach/coach";
import { sampleReel, type Reel } from "./capture/capture";
import type { ViewportPilot } from "./viewportRender";

/** The coach's camera: one pilot for the viewport's life (it is read once),
 *  which flies a step's tour from the first frame after the step appears and
 *  gives the camera back to the player where the tour ends. A tour never
 *  dips, so the reel's veil is never drawn. */
export function useCoachTour(step: CoachStep | null): ViewportPilot {
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
