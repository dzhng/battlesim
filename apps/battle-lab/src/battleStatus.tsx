// The top bar of a battle: the clock, and a skirmish's score meter and
// objectives.
import { useSessionCatalog } from "@web/battle/catalog/context";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "@web/battle/present/icons";
import {
  captureShare,
  objectiveStanding,
  type ObjectiveStanding,
} from "@web/battle/present/objectiveMarkers";
import type { ObservationView, SkirmishView } from "@web/battle/sim/observationView";
import type { SideName } from "@web/battle/sim/protocol";

/** A tick as the battle's clock reads it: minutes and seconds. */
export function clock(tick: number, tickHz: number) {
  const s = Math.floor(tick / tickHz);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** The battle's clock, from the published tick. */
export function BattleClock({ tick }: { tick: number }) {
  const { rules } = useSessionCatalog();
  return (
    <span className="hud-clock" data-testid="clock">
      {clock(tick, rules.tick_hz)}
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

/** How the score strip names an objective's standing to the player. */
const STANDING_LABEL: Record<ObjectiveStanding, string> = {
  own: "yours",
  enemy: "enemy",
  neutral: "neutral",
};

/** How many cells each side's half of the score meter is cut into. */
const SCORE_CELLS = 20;

/** One side's half of the score meter: cells lit for what it has scored,
 *  filling from the outer end toward the finish at the centre. The cell it is
 *  filling blinks while the side is scoring. */
function ScoreMeter({
  label,
  standing,
  score,
  target,
  scoring,
}: {
  label: string;
  standing: "own" | "enemy";
  score: number;
  target: number;
  scoring: boolean;
}) {
  const filled = Math.min(1, score / target) * SCORE_CELLS;
  const whole = Math.floor(filled);
  const points = Math.floor(score);
  return (
    <div
      className="hud-score-half"
      data-owner={standing}
      role="meter"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={target}
      aria-valuenow={points}
      title={`${points} / ${target.toLocaleString("en-US")}`}
    >
      {Array.from({ length: SCORE_CELLS }, (_, cell) => (
        <i
          key={cell}
          data-lit={cell < whole ? "full" : cell < filled ? "part" : undefined}
          data-filling={scoring && cell === whole ? true : undefined}
        />
      ))}
    </div>
  );
}

/** Match authority publishes both teams' progress; territory funds victory
 *  only. `viewer` is the side of the human the client plays, null when it
 *  watches; a watching client reads the match as Blue, whose observation it
 *  sees. Scores are published Blue then Red. A side scores while it holds a
 *  flag no enemy stands on. The preparation countdown stands where the score
 *  meter will, which has nothing to show until the battle starts. */
export function SkirmishStatus({
  match,
  viewer,
  target,
}: {
  target: number;
  match: NonNullable<ObservationView["skirmish"]>;
  viewer: SideName | null;
}) {
  const ours: SideName = viewer ?? "blue";
  const [own, enemy] = ours === "blue" ? match.scores : [match.scores[1], match.scores[0]];
  const scoring = (standing: ObjectiveStanding) =>
    match.result === null &&
    match.objectives.some(
      (objective) =>
        !objective.contested && objectiveStanding(objective.owner, viewer) === standing,
    );
  return (
    <div className="hud-match">
      {match.result && (
        <span className="hud-objective" role="status">
          {outcome(match.result, viewer)}
        </span>
      )}
      {match.phase === "preparation" ? (
        <span className="hud-objective">
          PREPARATION · {Math.ceil(match.preparationRemainingS)} s
        </span>
      ) : (
        <div className="hud-score">
          <ScoreMeter
            label="Your victory score"
            standing="own"
            score={own}
            target={target}
            scoring={scoring("own")}
          />
          <span className="hud-score-finish" />
          <ScoreMeter
            label="Enemy victory score"
            standing="enemy"
            score={enemy}
            target={target}
            scoring={scoring("enemy")}
          />
        </div>
      )}
      <div className="hud-objective-flags" aria-label="Objectives">
        {match.objectives.map((objective, index) => {
          const standing = objectiveStanding(objective.owner, viewer);
          return (
            <span
              key={objective.id}
              data-owner={standing}
              data-contested={objective.contested}
              aria-label={`Objective ${index + 1}: ${objective.contested ? "contested" : STANDING_LABEL[standing]}`}
            >
              <Icon path={hudIcon("objective")} />
              {index + 1}
              {captureShare(objective)}
            </span>
          );
        })}
      </div>
    </div>
  );
}
