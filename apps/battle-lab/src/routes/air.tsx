import { useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// The helicopter labs on the `air` test map, one saved encounter each:
// - hover: the test helicopter hovering at cruise height over open ground,
//   the test jeep on the road below it for scale: its rotors turn, its shadow
//   lies on the ground below and off to the side, and nothing else moves.
// - crash: the test helicopter, one hit from death, flies east past a gun
//   jeep, which shoots it down: it falls on in a spinning arc and leaves its
//   wreck where it strikes the ground.
const SEED = 6;

interface AirLabSpec {
  fixture: "air-hover" | "air-crash";
  encounter: string;
  title: string;
  camera: Camera3DParams;
}

export const AIR_LABS = {
  // From the south-east at about the game's tactical pitch: the airframe over
  // the field, its shadow on the ground beside it, the jeep on the road behind.
  hover: {
    fixture: "air-hover",
    encounter: "hover",
    title: "Air hover",
    camera: { target: [142, 90, 8], distance: 58, pitch: 0.72, yaw: -1.25, ...gameCamera.lens },
  },
  // From the south, across the road: the whole arc, from where it is hit to
  // where its wreck lies, broadside on.
  crash: {
    fixture: "air-crash",
    encounter: "crash",
    title: "Air crash",
    camera: {
      target: [84, 150, 8],
      distance: 80,
      pitch: 0.5,
      yaw: -Math.PI / 2,
      ...gameCamera.lens,
    },
  },
} satisfies Record<string, AirLabSpec>;

export default function AirHover() {
  return <AirLab spec={AIR_LABS.hover} />;
}

export function AirLab({ spec }: { spec: AirLabSpec }) {
  return (
    <SavedEncounter fixture={spec.fixture} encounter={spec.encounter}>
      {(battle) => <AirLabView battle={battle} spec={spec} />}
    </SavedEncounter>
  );
}

function AirLabView({ battle, spec }: { battle: SavedBattle; spec: AirLabSpec }) {
  const session = useBattleSession({ ...battle, seed: SEED });
  const { meshes, sim } = session;
  const worldFeed = useFeed(meshes);
  const overlayFeed = useFeed(useMemo(() => undefined, []));
  const { observation } = sim;

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture={spec.fixture}
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        buildings={session.buildingsFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={spec.camera}
        onPick={session.onPick}
        onRightPress={session.onRightPress}
        onCursor={session.onCursor}
        pointerMarks={session.pointerPaint.feed}
        onBox={session.onBox}
        onReady={session.onReady}
        onFrame={session.placePanels}
        diagnostics={session.probes}
      />
      <aside
        className="hud-panel lab-panel"
        data-occludes-readouts
        data-testid={`${spec.fixture}-panel`}
      >
        <strong>{spec.title}</strong>
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
