// The top bar of a battle with an objective: how the hold stands and the
// clock. One readout for every such battle, the village's and a prepared one.
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
