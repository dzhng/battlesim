import { useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// The test helicopter hovering at cruise height over open ground, the test
// jeep on the road below it for scale: its rotors turn, its shadow lies on
// the ground below and off to the side, and nothing else moves.
const SEED = 6;

// From the south-east at about the game's tactical pitch: the airframe over
// the field, its shadow on the ground beside it, the jeep on the road behind.
const AIR_HOVER_CAMERA: Camera3DParams = {
  target: [142, 90, 8],
  distance: 58,
  pitch: 0.72,
  yaw: -1.25,
  ...gameCamera.lens,
};

export default function AirHover() {
  return (
    <SavedEncounter fixture="air-hover" encounter="hover">
      {(battle) => <AirHoverLab battle={battle} />}
    </SavedEncounter>
  );
}

function AirHoverLab({ battle }: { battle: SavedBattle }) {
  const session = useBattleSession({ ...battle, seed: SEED });
  const { meshes, sim } = session;
  const worldFeed = useFeed(meshes);
  const overlayFeed = useFeed(useMemo(() => undefined, []));
  const { observation } = sim;

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="air-hover"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        buildings={session.buildingsFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={AIR_HOVER_CAMERA}
        onPick={session.onPick}
        onRightPress={session.onRightPress}
        onCursor={session.onCursor}
        pointerMarks={session.pointerPaint.feed}
        onBox={session.onBox}
        onReady={session.onReady}
        onFrame={session.placePanels}
        diagnostics={session.probes}
      />
      <aside className="hud-panel lab-panel" data-occludes-readouts data-testid="air-hover-panel">
        <strong>Air hover</strong>
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
