import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { useLabOverlay } from "../labOverlay";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// The helicopters' closing scene (D14), the air map's `closing` encounter:
// the AH-64E flies in from the west edge, pops up over the village roofs and
// kills the T-72 waiting behind them with rockets and its chin gun; ordered
// on east, holding its fire, it is shot down by the BMP-2M the tower hid,
// and its wreck comes down in the wood and flattens the trees there. Drawn
// as the battle draws it: drop line and ground ring, blue's fog.
const SEED = 6;

// From the south, high and wide: the village, the tower and the wood in one
// frame.
const CAMERA: Camera3DParams = {
  target: [300, 470, 10],
  distance: 330,
  pitch: 0.55,
  yaw: -Math.PI / 2,
  ...gameCamera.lens,
};

export default function AirClosing() {
  return (
    <SavedEncounter fixture="air-closing" encounter="closing">
      {(battle) => <AirClosingLab battle={battle} />}
    </SavedEncounter>
  );
}

function AirClosingLab({ battle }: { battle: SavedBattle }) {
  const session = useBattleSession({ ...battle, seed: SEED });
  const { meshes, sim } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  const { overlayFeed, onFrame } = useLabOverlay(session, CAMERA);

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="air-closing"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        buildings={session.buildingsFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={CAMERA}
        onPick={session.onPick}
        onRightPress={session.onRightPress}
        onCursor={session.onCursor}
        pointerMarks={session.pointerPaint.feed}
        onBox={session.onBox}
        onReady={session.onReady}
        onFrame={onFrame}
        diagnostics={session.probes}
      />
      <aside className="hud-panel lab-panel" data-occludes-readouts data-testid="air-closing-panel">
        <strong>Closing scene</strong>
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
