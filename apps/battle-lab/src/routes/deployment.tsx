import { useCallback, useEffect, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { ReadoutLayer } from "@web/battle/present/readouts";
import type { OwnUnitView } from "@web/battle/sim/observation";
import type { Order } from "@web/battle/sim/protocol";
import deploymentMap from "@fixtures/deployment-lab.json";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { AckLog } from "../AckLog";
import { orderLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario } from "../scenarios";
import { useFeed } from "../feed";
import { villageCamera } from "../villageCamera";

// One supply truck on a road. It sets up where it stands (a stopped supply
// unit deploys); every button sends a real command through the one path. Its
// progress is its info panel's row, as in the battle.
const SCENARIO = labScenario(deploymentMap, [
  { side: "blue", kind: "supply", position: [100, 100] },
]);
const SEED = 12;

const DEPLOYMENT_CAMERA: Camera3DParams = {
  target: [102, 110, 0],
  distance: 46,
  pitch: 0.95,
  yaw: -1.57,
  ...villageCamera.lens,
};

/** Reference commands, exactly as a player would send them. */
const DEMOS: Record<string, { order: (units: number[]) => Order; queued?: boolean }> = {
  "Move east": {
    order: (units) => ({
      kind: "move",
      units,
      gesture: 9201,
      goal: [140, 100],
      route: "shortest",
    }),
  },
  "Queue move west": {
    order: (units) => ({
      kind: "move",
      units,
      gesture: 9202,
      goal: [80, 100],
      route: "shortest",
    }),
    queued: true,
  },
};

export default function Deployment() {
  const session = useBattleSession({ map: deploymentMap, scenario: SCENARIO, seed: SEED });
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;

  // The truck starts selected, so the buttons act on it at once.
  const { setSelected } = control;
  const firstSupply = observation?.own.find((u) => u.kind === "supply")?.id ?? null;
  const autoSelected = useRef<typeof sim.client>(null);
  useEffect(() => {
    if (firstSupply === null || autoSelected.current === sim.client) return;
    autoSelected.current = sim.client;
    setSelected([firstSupply]);
  }, [firstSupply, sim.client, setSelected]);

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    return orderLayer(
      observation,
      observation.own.map((u) => u.id),
      session.revealed,
      surfaceZ,
    );
  }, [world, observation, session.revealed, surfaceZ]);
  const overlayFeed = useFeed(overlay);

  const runDemo = useCallback(
    (name: string) => {
      if (!control.selected.length) return null;
      const demo = DEMOS[name];
      return control.issue(demo.order(control.selected), demo.queued ?? false);
    },
    [control],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = { ...session.probes, demo: (name: string) => runDemo(name) };

  if (!meshes) return null;
  const has = control.selected.length > 0;
  return (
    <>
      <LabViewport
        fixture="deployment"
        world={worldFeed}
        overlay={overlayFeed}
        instances={[]}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={DEPLOYMENT_CAMERA}
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
      <aside className="lab-panel" data-occludes-readouts data-testid="deployment-panel">
        <strong>Deployment</strong>
        <div className="lab-hint">
          Click: select · Right‑click: move (Shift queues) · S: stop · buttons act on the selection
        </div>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
        <div className="lab-row">
          <button type="button" onClick={() => control.setDeployment(true)} disabled={!has}>
            Deploy
          </button>
          <button type="button" onClick={() => control.setDeployment(false)} disabled={!has}>
            Pack
          </button>
          <button type="button" onClick={control.stop} disabled={!has}>
            Stop
          </button>
          {Object.keys(DEMOS).map((name) => (
            <button key={name} type="button" onClick={() => void runDemo(name)} disabled={!has}>
              {name}
            </button>
          ))}
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
        </div>
        <div data-testid="deployment-readout" className="lab-actions">
          {control.selectedUnits.length === 0 && <div className="lab-hint">No unit selected</div>}
          {control.selectedUnits.map((u) => (
            <DeploymentReadout
              key={u.id}
              unit={u}
              seconds={UNITS.type(u.kind).capabilities.deploy?.seconds ?? 0}
            />
          ))}
        </div>
        <AckLog acks={control.acks} />
      </aside>
    </>
  );
}

/** The one progress value and its target, read straight from the observation. */
/** Deploying and packing both take `seconds` (the scenario's service rules). */
function DeploymentReadout({ unit, seconds }: { unit: OwnUnitView; seconds: number }) {
  const d = unit.deployment;
  const orders = unit.goal
    ? `move to (${unit.goal.map((v) => v.toFixed(0)).join(", ")})${
        unit.queue.length ? ` + ${unit.queue.length} queued` : ""
      }`
    : "none";
  return (
    <div className="lab-unit">
      <div>
        <strong>
          {unit.kind} #{unit.id}
        </strong>{" "}
        · movement: {unit.state.replace("_", " ")} · orders: {orders}
      </div>
      {d ? (
        <>
          {/* The fill always means how deployed the unit is, whichever way it moves. */}
          <div className="lab-bar lab-bar-deploy">
            <div
              className="lab-track"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={1}
              aria-valuenow={d.progress}
            >
              <div className="lab-fill" style={{ width: `${d.progress * 100}%` }} />
            </div>
            <span data-testid="deploy-seconds">
              {/* Rounded down, so unfinished setup never reads complete. */}
              {`deployed ${(Math.floor(d.progress * seconds * 10) / 10).toFixed(1)}/${seconds.toFixed(1)} s`}
            </span>
          </div>
          <div data-testid="deploy-direction">{describeDirection(d.progress, d.target)}</div>
          <div data-testid="service-ready">
            service: {d.progress >= 1 ? "ready (fully deployed)" : "not ready until fully deployed"}
          </div>
        </>
      ) : (
        <div className="lab-hint">does not deploy</div>
      )}
    </div>
  );
}

function describeDirection(progress: number, target: string): string {
  if (target === "deployed") return progress >= 1 ? "✓ fully deployed" : "↻ deploying";
  return progress <= 0 ? "packed" : "↺ packing";
}
