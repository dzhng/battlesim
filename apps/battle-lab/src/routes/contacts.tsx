import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { BattleView } from "../BattleView";
import { SavedEncounter } from "../savedMaps";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// Scripted evidence and movement, observed through the player's presentation.
const SEED = 6;
const CONTACTS_CAMERA: Camera3DParams = {
  target: [720, 420, 0],
  distance: 520,
  pitch: 0.95,
  yaw: -1.57,
  ...gameCamera.lens,
};

export default function Contacts() {
  return (
    <SavedEncounter fixture="contacts" encounter="contacts">
      {(battle) => (
        <BattleView
          fixture="contacts"
          scenario={battle.scenario}
          seed={SEED}
          camera={CONTACTS_CAMERA}
          status={({ sim }) => (
            <>
              <strong>Contacts and sound</strong>
              <TickStatus tick={sim.observation?.tick} status={sim.status.status} />
            </>
          )}
        />
      )}
    </SavedEncounter>
  );
}
