// The top bar of a battle: the clock, and a skirmish's scores and
// objectives.
import { shippedRules } from "@web/shippedRules";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "@web/battle/present/icons";
import type { ObservationView, SkirmishView } from "@web/battle/sim/observationView";
import type { SideName } from "@web/battle/sim/protocol";

/** A tick as the battle's clock reads it: minutes and seconds. */
export function clock(tick: number) {
  const s = Math.floor(tick / shippedRules.tick_hz);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** The battle's clock, from the published tick. */
export function BattleClock({ tick }: { tick: number }) {
  return (
    <span className="hud-clock" data-testid="clock">
      {clock(tick)}
    </span>
  );
}

/** A finished match's result as `viewer`'s side reads it: VICTORY or
 *  DEFEAT, or, for a client watching a battle it doesn't play in (null), the
 *  side that won. */
export function outcome(result: NonNullable<SkirmishView["result"]>, viewer: SideName | null) {
  if (result === "draw") return "DRAW";
  if (viewer === null) return `${result.toUpperCase()} WINS`;
  return result === viewer ? "VICTORY" : "DEFEAT";
}

/** Match authority publishes both teams' progress; territory funds victory
 *  only. `viewer` is the side of the human the client plays, null when it
 *  watches; a watching client reads the match as Blue, whose observation it
 *  sees. Scores are published Blue then Red. */
export function SkirmishStatus({
  match,
  viewer,
}: {
  match: NonNullable<ObservationView["skirmish"]>;
  viewer: SideName | null;
}) {
  const ours: SideName = viewer ?? "blue";
  const [own, enemy] = ours === "blue" ? match.scores : [match.scores[1], match.scores[0]];
  return (
    <>
      {match.phase === "preparation" ? (
        <span className="hud-objective">
          PREPARATION · {Math.ceil(match.preparationRemainingS)} s
        </span>
      ) : (
        <>
          <span className="hud-score" aria-label="Your victory score">
            {Math.floor(own)} / 1,000
          </span>
          <span className="hud-score hud-score-enemy" aria-label="Enemy victory score">
            {Math.floor(enemy)} / 1,000
          </span>
        </>
      )}
      {match.result && (
        <span className="hud-objective" role="status">
          {outcome(match.result, viewer)}
        </span>
      )}
      <div className="hud-objective-flags" aria-label="Objectives">
        {match.objectives.map((objective, index) => (
          <span
            key={objective.id}
            data-owner={objective.owner ?? "neutral"}
            data-contested={objective.contested}
            aria-label={`Objective ${index + 1}: ${objective.contested ? "contested" : objective.owner === ours ? "yours" : objective.owner ? "enemy" : "neutral"}`}
          >
            <Icon path={hudIcon("objective")} />
            {index + 1}
            {objective.contested
              ? " · CONTESTED"
              : objective.capturing
                ? ` · ${Math.floor(objective.captureProgress * 100)}%`
                : ""}
          </span>
        ))}
      </div>
    </>
  );
}
