import { useCallback, useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { ObservationView } from "@web/battle/sim/observation";
import type { Order } from "@web/battle/sim/protocol";
import { AckLog } from "../AckLog";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// Two brew-ups, one order each: blue's tank kills red's tank (tracked, a
// turret to throw) and red's jeep (wheeled, none). Each wreck stays; the
// debris each throws lies wider than the wreck for a while, then sinks away.
// Everyone holds fire until a demo orders it.
const SEED = 9;

const COOK_OFF_CAMERA: Camera3DParams = {
  target: [440, 393, 0],
  distance: 60,
  pitch: 0.8,
  yaw: -2.2,
  ...gameCamera.lens,
};

const target =
  (kind: string) =>
  (o: ObservationView): Order | null => {
    const unit = o.identified.find((e) => e.kind === kind);
    return unit
      ? { kind: "attack", units: [0], target: { kind: "identified", id: unit.id } }
      : null;
  };

/** Reference commands, exactly as a player would send them. */
const DEMOS: Record<string, (o: ObservationView) => Order | null> = {
  "Destroy the red tank": target("test_tank"),
  "Destroy the red jeep": target("test_jeep"),
};

export default function CookOff() {
  return (
    <SavedEncounter fixture="cook-off" encounter="cook-off">
      {(battle) => <CookOffLab battle={battle} />}
    </SavedEncounter>
  );
}

function CookOffLab({ battle }: { battle: SavedBattle }) {
  const session = useBattleSession({ ...battle, seed: SEED });
  const { meshes, sim, control } = session;
  const worldFeed = useFeed(meshes);
  const overlayFeed = useFeed(useMemo(() => undefined, []));
  const { observation } = sim;

  const runDemo = useCallback(
    async (name: string) => {
      const o = sim.latest.current;
      const order = o && DEMOS[name](o);
      if (!order) return null;
      if ("units" in order) control.setSelected(order.units);
      return control.issue(order);
    },
    [sim.latest, control],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = { ...session.probes, demo: (name: string) => runDemo(name) };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="cook-off"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        buildings={session.buildingsFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={COOK_OFF_CAMERA}
        onPick={session.onPick}
        onRightPress={session.onRightPress}
        onCursor={session.onCursor}
        pointerMarks={session.pointerPaint.feed}
        onBox={session.onBox}
        onReady={session.onReady}
        onFrame={session.placePanels}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-occludes-readouts data-testid="cook-off-panel">
        <strong>Cook-offs</strong>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
        <div className="lab-row">
          {Object.keys(DEMOS).map((name) => (
            <button key={name} type="button" onClick={() => void runDemo(name)}>
              {name}
            </button>
          ))}
          <button type="button" onClick={sim.restart}>
            Reset
          </button>
        </div>
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}
