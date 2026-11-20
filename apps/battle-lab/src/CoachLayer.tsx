/** The coach's own layer over the battle: its card and its rings round the
 *  HUD controls the step points at. It sits over the match's result, so its
 *  closing words outlast the battle, and under the pause menu, which it
 *  waits behind. */
import { CoachHighlights } from "@web/battle/coach/CoachHighlights";
import { CoachPanel } from "@web/battle/coach/CoachPanel";
import type { ScenarioCoach } from "./useScenarioCoach";

export function CoachLayer({
  coach,
  paused,
  ended,
  speak,
}: {
  coach: ScenarioCoach;
  paused: boolean;
  ended: boolean;
  /** Plays a narration line; its length in seconds, or null for none. */
  speak?: (speaker: string, line: string) => Promise<number | null>;
}) {
  return (
    <div className="hud-coach-layer" inert={paused} data-ended={ended || undefined}>
      <CoachPanel
        doc={coach.doc}
        view={coach.view}
        told={coach.told}
        run={coach.run}
        onContinue={coach.dismiss}
        speak={speak}
      />
      <CoachHighlights anchors={coach.view?.step.highlight ?? []} />
    </div>
  );
}
