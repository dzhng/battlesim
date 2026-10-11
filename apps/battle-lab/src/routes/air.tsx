import { useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// The first helicopter checkpoint: the test helicopter flies one move order
// over a village at cruise height, pops over its roofs, goes round the tower
// in its way and over a little wood. A red rifle squad by the village plinks
// at it, a tank far to the south never brings its gun to bear, and the gun
// jeep beside the tank, told to fire once the helicopter hovers at the end of
// its route and to hold again after one round, hurts it.
const SEED = 7;

// From the south-west over the village: the route from the first houses to
// the wood past the tower, the rifle squad in front of the houses.
const AIR_CAMERA: Camera3DParams = {
  target: [300, 440, 12],
  distance: 330,
  pitch: 0.55,
  yaw: -1.9,
  ...gameCamera.lens,
};

export default function Air() {
  return (
    <SavedEncounter fixture="air" encounter="flight">
      {(battle) => <AirLab battle={battle} />}
    </SavedEncounter>
  );
}

function AirLab({ battle }: { battle: SavedBattle }) {
  const session = useBattleSession({ ...battle, seed: SEED });
  const { meshes, sim } = session;
  const worldFeed = useFeed(meshes);
  const overlayFeed = useFeed(useMemo(() => undefined, []));
  const { observation } = sim;

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="air"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        buildings={session.buildingsFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={AIR_CAMERA}
        onPick={session.onPick}
        onRightPress={session.onRightPress}
        onCursor={session.onCursor}
        pointerMarks={session.pointerPaint.feed}
        onBox={session.onBox}
        onReady={session.onReady}
        onFrame={session.placePanels}
        diagnostics={session.probes}
      />
      <aside className="hud-panel lab-panel" data-occludes-readouts data-testid="air-panel">
        <strong>Air</strong>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
        <div className="lab-row">
          <button type="button" onClick={sim.restart}>
            Reset
          </button>
        </div>
      </aside>
    </>
  );
}
