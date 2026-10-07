// /lab/lean: the lean-out firefight on the street test map, watched: the
// map's saved encounter `lean` (`fixtures/maps/street/encounters/lean.json`).
// A blue squad at rest just inside the west wood trades fire with a red
// squad in the open 45 m east; soldiers too tough to fall, so the fight
// holds. Men lean out from their trees, fire and tuck back in.
import { useMemo } from "react";
import game from "@fixtures/game.json";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { durableSoldiers } from "../scenarios";
import { SavedEncounter } from "../savedMaps";
import { BattleView } from "../BattleView";
import { gameCamera } from "../gameCamera";
import { BattleClock } from "../battleStatus";

export default function Lean() {
  const { rules } = useSessionCatalog();
  const durable = useMemo(() => durableSoldiers(rules), [rules]);
  return (
    <SavedEncounter fixture="lean" encounter="lean" rules={durable}>
      {(battle) => (
        <BattleView
          fixture="lean"
          scenario={battle.scenario}
          seed={game.seed}
          camera={gameCamera.opening()}
          status={({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />}
        />
      )}
    </SavedEncounter>
  );
}
