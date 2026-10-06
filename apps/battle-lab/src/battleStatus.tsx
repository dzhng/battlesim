// The top bar of a battle with an objective: how the hold stands and the
// clock. One readout for every such battle, the village's and a prepared one.
import { hudIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "@web/battle/present/icons";
import game from "@fixtures/game.json";
import type { ObservationView } from "@web/battle/sim/observation";
import type { BattleSession } from "./useBattleSession";

/** The battle's clock, from the published tick. */
export function BattleClock({ tick }: { tick: number }) {
  const s = tick / game.tick_hz;
  return (
    <span className="hud-clock" data-testid="clock">
      {Math.floor(s / 60)}:{String(Math.floor(s % 60)).padStart(2, "0")}
    </span>
  );
}

/** The objective's readout and the clock, as a battle view's `status`: the
 *  hold's count (of `holdS` seconds) while the battle runs, its result once
 *  it is decided, a win called `captured` ("VILLAGE CAPTURED"). */
export const objectiveStatus =
  (holdS: number, captured: string) =>
  ({ sim }: BattleSession) => (
    <ObjectiveStatus
      encounter={sim.observation?.encounter ?? null}
      tick={sim.observation?.tick ?? 0}
      holdS={holdS}
      captured={captured}
    />
  );

function ObjectiveStatus({
  encounter: enc,
  tick,
  holdS,
  captured,
}: {
  encounter: ObservationView["encounter"];
  tick: number;
  holdS: number;
  captured: string;
}) {
  const result: Record<string, string> = {
    captured,
    defeated: "DEFEATED",
    inconclusive: "INCONCLUSIVE: PLAY ON",
  };
  return (
    <>
      <span className="hud-objective" data-testid="encounter">
        {!enc
          ? "—"
          : enc.result === "running"
            ? `HOLD ${enc.heldS.toFixed(0)}/${holdS} s`
            : result[enc.result]}
      </span>
      <BattleClock tick={tick} />
    </>
  );
}

/** Match authority publishes both teams' progress; territory funds victory only. */
export function SkirmishStatus({ match }: { match: NonNullable<ObservationView["skirmish"]> }) {
  return (
    <>
      {match.phase === "preparation" ? (
        <span className="hud-objective">
          PREPARATION · {Math.ceil(match.preparationRemainingS)} s
        </span>
      ) : (
        <>
          <span className="hud-score" aria-label="Your victory score">
            {Math.floor(match.scores[0])} / 1,000
          </span>
          <span className="hud-score hud-score-enemy" aria-label="Enemy victory score">
            {Math.floor(match.scores[1])} / 1,000
          </span>
        </>
      )}
      {match.result && (
        <span className="hud-objective" role="status">
          {match.result === "draw" ? "DRAW" : match.result === "blue" ? "VICTORY" : "DEFEAT"}
        </span>
      )}
      <div className="hud-objective-flags" aria-label="Objectives">
        {match.objectives.map((objective, index) => (
          <span
            key={objective.id}
            data-owner={objective.owner ?? "neutral"}
            data-contested={objective.contested}
            aria-label={`Objective ${index + 1}: ${objective.contested ? "contested" : objective.owner === "blue" ? "yours" : objective.owner === "red" ? "enemy" : "neutral"}`}
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
