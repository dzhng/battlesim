import { BattleView } from "../BattleView";
import { SavedEncounter } from "../savedMaps";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

/** A reachable squad beside a building and a vehicle enclosed by walls. */
export default function CursorOrders() {
  return (
    <SavedEncounter fixture="cursor-orders" encounter="cursor-orders">
      {(battle) => (
        <BattleView
          fixture="cursor-orders"
          scenario={battle.scenario}
          seed={4}
          camera={{
            ...gameCamera.lens,
            target: [346, 252, 0],
            distance: 130,
            pitch: 0.95,
            yaw: -0.75,
          }}
          status={({ sim }) => (
            <TickStatus tick={sim.observation?.tick} status={sim.status.status} />
          )}
        />
      )}
    </SavedEncounter>
  );
}
