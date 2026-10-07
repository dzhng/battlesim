// /lab/street: a battle on the street test map, through the production
// controls and readouts: the map's saved encounter `attack`
// (`fixtures/maps/street/encounters/attack.json`), blue at its start west of
// the street, red holding the street under the encounter's defender policy.
// `?watch` plays `advance` instead, where blue's start is ordered onto the
// street at once, so the battle plays itself under a free camera; `?seed=`
// sets the battle's seed. The browser scenes judge the battle view here; their
// `remount()` probe mounts a fresh battle view (a new device, worker and
// sound), so they can check that one releases the last.
import { useState } from "react";
import game from "@fixtures/game.json";
import { SavedEncounter } from "../savedMaps";
import { BattleView } from "../BattleView";
import { gameCamera } from "../gameCamera";
import { BattleClock } from "../battleStatus";

/** The battle's seed: the fixture's, or `?seed=` for testing. */
function urlSeed(): number {
  const seed = Number(new URLSearchParams(window.location.search).get("seed"));
  return Number.isInteger(seed) && seed > 0 ? seed : game.seed;
}

export default function Street() {
  const [seed] = useState(urlSeed);
  const [watch] = useState(() => new URLSearchParams(window.location.search).has("watch"));
  const [mount, setMount] = useState(0);
  return (
    <SavedEncounter fixture="street" encounter={watch ? "advance" : "attack"}>
      {(battle) => (
        <BattleView
          key={mount}
          fixture="street"
          scenario={battle.scenario}
          seed={seed}
          camera={gameCamera.opening()}
          status={({ sim }) => <BattleClock tick={sim.observation?.tick ?? 0} />}
          diagnostics={() => ({ mount: () => mount, remount: () => setMount(mount + 1) })}
        />
      )}
    </SavedEncounter>
  );
}
