import { useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import {
  CommandBar,
  ReadoutLayer,
  SelectionPanel,
  type ReadoutLayerHandle,
} from "@web/battle/present/readouts";
import readoutsMap from "@fixtures/readouts-lab.json";
import { AckLog } from "../AckLog";
import { orderLayer, tracerLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario } from "../scenarios";

// A tank (cannon with AP/HE, and an HMG), an AT team, a rifle squad and a
// supply truck setting up, facing a red tank: every kind of timer runs at once.
const SCENARIO = labScenario(readoutsMap, [
  { side: "blue", kind: "tank", position: [200, 220] },
  { side: "blue", kind: "at", position: [215, 270] },
  { side: "blue", kind: "rifle", position: [185, 170], condition: { spent: { grenade: 3 } } },
  { side: "blue", kind: "supply", position: [140, 230] },
  { side: "red", kind: "tank", position: [470, 230], yaw: Math.PI, engagement: "return_fire_only" },
]);
const SEED = 14;

const READOUTS_CAMERA: Camera3DParams = {
  target: [215, 230, 0],
  distance: 400,
  pitch: 0.95,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

export default function Readouts() {
  const session = useBattleSession({ map: readoutsMap, scenario: SCENARIO, seed: SEED });
  const { world, meshes, sim, control, surfaceZ } = session;
  const { observation } = sim;
  const readouts = useRef<ReadoutLayerHandle>(null);

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const orders = orderLayer(observation, control.selected, surfaceZ);
    const tracers = tracerLayer(observation);
    return {
      opaque: concatMeshes([orders.opaque, tracers.opaque]),
      translucent: concatMeshes([orders.translucent, tracers.translucent]),
    };
  }, [world, observation, surfaceZ, control.selected]);

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = { ...session.probes, mode: () => control.mode };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="readouts"
        world={meshes}
        overlay={overlay}
        fog={observation?.fog ?? null}
        instances={[]}
        frameInstances={session.frameInstances}
        initialCamera={READOUTS_CAMERA}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        onFrame={(project, camera) =>
          readouts.current?.place(project, camera.distance, session.drawnAt.current)
        }
        diagnostics={diagnostics}
      />
      <ReadoutLayer
        observation={observation}
        selected={control.selected}
        handle={readouts}
        groundZ={surfaceZ}
      />
      <aside className="lab-panel" data-testid="readouts-panel">
        <strong>Weapon readouts</strong>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
        <CommandBar
          mode={control.mode}
          setMode={control.setMode}
          selected={control.selectedUnits}
          onStop={control.stop}
          onTogglePolicy={control.togglePolicy}
          onDeploy={control.setDeployment}
          onExit={control.exitBuilding}
        />
        <div className="lab-legend">
          Rings: <span className="lab-swatch lab-swatch-aim" /> aim ·{" "}
          <span className="lab-swatch lab-swatch-reload" /> reload (dashed) · number: rounds left (∞
          unlimited) · ⌖ guiding · lower badge: why it cannot fire · square: set-up (▲ setting up, ▼
          packing, ✓ set up)
        </div>
        <SelectionPanel units={control.selectedUnits} />
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}
