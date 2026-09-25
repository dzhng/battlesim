import { useCallback, useEffect, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import { buildDeploymentOverlay } from "@packages/battle-renderer/src/deploymentOverlay";
import { buildOrderOverlay } from "@packages/battle-renderer/src/orderOverlay";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import type { OwnUnitView } from "@web/battle/sim/observation";
import type { Order } from "@web/battle/sim/protocol";
import village from "@fixtures/village.json";
import deploymentMap from "@fixtures/deployment-lab.json";
import { AckLine } from "../AckLine";
import { LabViewport, type LabPick } from "../LabViewport";
import { labScenario } from "../scenarios";
import { sideInstances } from "../sideInstances";
import { useSimSession } from "../useSimSession";
import { groundUnderRay, useStaticWorld } from "../useStaticWorld";

// One supply truck on a road. It sets up where it stands (a stopped supply
// unit deploys); every button sends a real command through the one path.
const SCENARIO = labScenario(deploymentMap, [
  { side: "blue", kind: "supply", position: [100, 100] },
]);
const SEED = 12;
/** Deploying and packing both take this long (village.json `service`). */
const DURATION_S = village.service.deploy_and_pack_s;

export const DEPLOYMENT_CAMERA: Camera3DParams = {
  target: [102, 110, 0],
  distance: 46,
  pitch: 0.95,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
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
  const world = useStaticWorld(deploymentMap);
  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface"),
    [world],
  );
  const sim = useSimSession({ scenario: SCENARIO, seed: SEED });
  const { observation } = sim;
  const control = useUnitControl(sim.client, observation);
  const instanceUnits = useRef<(number | null)[]>([]);
  const selectedRef = useRef(control.selected);
  selectedRef.current = control.selected;

  // The truck starts selected, so the buttons act on it at once.
  const { setSelected } = control;
  const firstSupply = observation?.own.find((u) => u.kind === "supply")?.id ?? null;
  const autoSelected = useRef<typeof sim.client>(null);
  useEffect(() => {
    if (firstSupply === null || autoSelected.current === sim.client) return;
    autoSelected.current = sim.client;
    setSelected([firstSupply]);
  }, [firstSupply, sim.client, setSelected]);

  const surfaceZ = useCallback(
    (x: number, y: number) => world?.view.surface_at(x, y)[0] ?? 0,
    [world],
  );

  const frameInstances = useCallback(
    (now: number): SceneInstance[] | null => {
      const poses = sim.interpolator.current?.sample(now);
      if (!poses || !observation) return null;
      const drawn = sideInstances("blue", poses, observation, selectedRef.current);
      instanceUnits.current = drawn.owners;
      return drawn.instances;
    },
    [observation, sim.interpolator],
  );

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const orders = buildOrderOverlay(observation.own, surfaceZ);
    const rings = buildDeploymentOverlay(
      observation.own.flatMap((u) =>
        u.deployment ? [{ position: u.position, yaw: u.yaw, ...u.deployment }] : [],
      ),
      surfaceZ,
    );
    return { opaque: concatMeshes([orders.opaque, rings]), translucent: orders.translucent };
  }, [world, observation, surfaceZ]);

  const onPick = useCallback(
    (pick: LabPick) => {
      const ground = world && pick.button === "right" ? groundUnderRay(world.view, pick.ray) : null;
      control.onPointer({
        ...pick,
        unit: pick.instance >= 0 ? (instanceUnits.current[pick.instance] ?? null) : null,
        ground: ground && [ground[0], ground[1]],
      });
    },
    [world, control],
  );

  const runDemo = useCallback(
    (name: string) => {
      if (!control.selected.length) return null;
      const demo = DEMOS[name];
      return control.issue(demo.order(control.selected), demo.queued ?? false);
    },
    [control],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    tick: () => sim.latest.current?.tick ?? 0,
    observation: () => sim.latest.current,
    acks: () => control.acks,
    selected: () => control.selected,
    select: (ids: number[]) => control.setSelected(ids),
    command: (order: Order, queued = false) => control.issue(order, queued),
    demo: (name: string) => runDemo(name),
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
  };

  if (!meshes) return null;
  const has = control.selected.length > 0;
  return (
    <>
      <LabViewport
        fixture="deployment"
        world={meshes}
        overlay={overlay}
        instances={[]}
        frameInstances={frameInstances}
        initialCamera={DEPLOYMENT_CAMERA}
        onPick={onPick}
        onReady={sim.onViewportReady}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="deployment-panel">
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
            <DeploymentReadout key={u.id} unit={u} />
          ))}
        </div>
        <div className="lab-hint">Commands, newest first</div>
        <ul className="lab-log" data-testid="ack-log">
          {control.acks.length === 0 && <li>None yet</li>}
          {control.acks.map((a) => (
            <AckLine key={a.seq} entry={a} />
          ))}
        </ul>
      </aside>
    </>
  );
}

/** The one progress value and its target, read straight from the observation. */
function DeploymentReadout({ unit }: { unit: OwnUnitView }) {
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
          <div className="lab-bar">
            <span>setup</span>
            {/* Filled like the ground ring's arc, in its colour. */}
            <div
              className="lab-track"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={1}
              aria-valuenow={d.progress}
            >
              <div
                className={`lab-fill lab-fill-${phase(d.progress, d.target)}`}
                style={{ width: `${d.progress * 100}%` }}
              />
            </div>
            <span data-testid="deploy-seconds">
              {/* Rounded down, so unfinished setup never reads complete. */}
              {(Math.floor(d.progress * DURATION_S * 10) / 10).toFixed(1)}/{DURATION_S.toFixed(1)} s
            </span>
          </div>
          <div data-testid="deploy-direction">
            {/* Coloured like the ground ring's arc. */}
            <span className={`lab-deploy lab-deploy-${phase(d.progress, d.target)}`}>
              {describeDirection(d.progress, d.target)}
            </span>
          </div>
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
  if (target === "deployed") return progress >= 1 ? "deployed" : "deploying → deployed";
  return progress <= 0 ? "packed" : "packing → packed";
}

function phase(progress: number, target: string): "deploying" | "deployed" | "packing" | "packed" {
  if (target === "deployed") return progress >= 1 ? "deployed" : "deploying";
  return progress <= 0 ? "packed" : "packing";
}
