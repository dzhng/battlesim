import { useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// The air map's crash: the test helicopter, one hit from death, flies east
// past a gun jeep, which shoots it down: it falls on in a spinning arc and
// leaves its wreck where it strikes the ground.
const SEED = 6;

// From the south, across the road: the whole arc, from where it is hit to
// where its wreck lies, broadside on.
const CAMERA: Camera3DParams = {
  target: [84, 150, 8],
  distance: 80,
  pitch: 0.5,
  yaw: -Math.PI / 2,
  ...gameCamera.lens,
};

export default function AirCrash() {
  return (
    <SavedEncounter fixture="air-crash" encounter="crash">
      {(battle) => <AirCrashLab battle={battle} />}
    </SavedEncounter>
  );
}

function AirCrashLab({ battle }: { battle: SavedBattle }) {
  const session = useBattleSession({ ...battle, seed: SEED });
  const { meshes, sim } = session;
  const worldFeed = useFeed(meshes);
  const overlayFeed = useFeed(useMemo(() => undefined, []));
  const { observation } = sim;

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="air-crash"
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
        onFrame={session.placePanels}
        diagnostics={session.probes}
      />
      <aside className="hud-panel lab-panel" data-occludes-readouts data-testid="air-crash-panel">
        <strong>Air crash</strong>
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
