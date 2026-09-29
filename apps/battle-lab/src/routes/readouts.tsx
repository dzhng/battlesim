import { useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import { CommandBar, ReadoutLayer, SelectionCard } from "@web/battle/present/readouts";
import readoutsMap from "@fixtures/readouts-lab.json";
import { AckLog } from "../AckLog";
import { orderLayer, tracerLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario } from "../scenarios";
import { useFeed } from "../feed";
import { villageCamera } from "../villageCamera";

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
  ...villageCamera.lens,
};

export default function Readouts() {
  const session = useBattleSession({ map: readoutsMap, scenario: SCENARIO, seed: SEED });
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const orders = orderLayer(observation, control.selected, session.revealed, surfaceZ);
    const tracers = tracerLayer(observation);
    return combineWorldMeshes([orders, tracers]);
  }, [world, observation, surfaceZ, control.selected, session.revealed]);
  const overlayFeed = useFeed(overlay);

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = { ...session.probes, mode: () => control.mode };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="readouts"
        world={worldFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        instances={[]}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={READOUTS_CAMERA}
        onPick={session.onPick}
        onBox={session.onBox}
        onReady={session.onReady}
        onFrame={(project, camera) => session.placePanels(project, camera.distance)}
        diagnostics={diagnostics}
      />
      <ReadoutLayer
        own={observation?.own ?? []}
        rules={session.rules}
        selected={control.selected}
        handle={session.readouts}
      />
      <aside className="hud-panel lab-panel" data-occludes-readouts data-testid="readouts-panel">
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
          onToggleDeployment={control.toggleDeployment}
          onExit={control.exitBuilding}
        />
        <div className="lab-legend">
          Rings: <span className="lab-swatch lab-swatch-aim" /> aim ·{" "}
          <span className="lab-swatch lab-swatch-reload" /> reload (dashed) · number: rounds left ·
          the mark after a weapon: why it cannot fire
        </div>
        <SelectionCard
          units={control.selectedUnits}
          own={observation?.own ?? []}
          rules={session.rules}
        />
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}
