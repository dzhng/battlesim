import { useCallback, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import { buildOrderOverlay } from "@packages/battle-renderer/src/orderOverlay";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { proxyForUnit, SIDE_COLORS } from "@packages/battle-renderer/src/unitProxies";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import type { OwnUnitView } from "@web/battle/sim/observation";
import type { Order } from "@web/battle/sim/protocol";
import movementMap from "@fixtures/movement-lab.json";
import { AckLine } from "../AckLine";
import { LabViewport, type LabBox, type LabPick } from "../LabViewport";
import { labScenario } from "../scenarios";
import { useSimSession } from "../useSimSession";
import { groundUnderRay, useStaticWorld } from "../useStaticWorld";

// A wall drops across the main road at tick 150: units only learn of it by
// coming close, then route around it.
const SCENARIO = labScenario(
  movementMap,
  [
    { side: "blue", kind: "tank", position: [40, 190] },
    { side: "blue", kind: "tank", position: [40, 215] },
    { side: "blue", kind: "rifle", position: [70, 200] },
    { side: "blue", kind: "rifle", position: [70, 225] },
    { side: "blue", kind: "recon", position: [75, 175] },
    { side: "blue", kind: "supply", position: [20, 240] },
  ],
  [
    {
      tick: 150,
      add_prop: { kind: "wall", center: [160, 60], yaw: 0, half_extents: [0.5, 10, 2] },
    },
  ],
);
const SEED = 4;

export const MOVEMENT_CAMERA: Camera3DParams = {
  target: [120, 185, 0],
  distance: 390,
  pitch: 0.95,
  yaw: -1.5,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

/** Reference commands the lab can send, exactly as a player would. */
const DEMOS: Record<string, (own: OwnUnitView[]) => { order: Order; queued?: boolean }[]> = {
  "Shortest vs fastest": (own) => {
    const tanks = own.filter((u) => u.kind === "tank").map((u) => u.id);
    return [
      {
        order: {
          kind: "move",
          units: [tanks[0]],
          gesture: 9001,
          goal: [275, 185],
          route: "shortest",
        },
      },
      {
        order: {
          kind: "move",
          units: [tanks[1]],
          gesture: 9002,
          goal: [300, 215],
          route: "fastest",
        },
      },
    ];
  },
  "Group east": (own) => [
    {
      order: {
        kind: "move",
        units: own.map((u) => u.id),
        gesture: 9003,
        goal: [440, 150],
        route: "fastest",
      },
    },
  ],
  "Infantry through the gap": (own) => [
    {
      order: {
        kind: "move",
        units: own.filter((u) => u.kind === "rifle").map((u) => u.id),
        gesture: 9004,
        goal: [215, 325],
        route: "shortest",
      },
    },
  ],
  "Onto the cliff top": (own) => [
    {
      order: {
        kind: "move",
        units: [own.find((u) => u.kind === "supply")!.id],
        gesture: 9005,
        goal: [455, 345],
        route: "shortest",
      },
    },
  ],
};

export default function Movement() {
  const world = useStaticWorld(movementMap);
  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface"),
    [world],
  );
  const sim = useSimSession(SCENARIO, SEED);
  const { observation } = sim;
  const control = useUnitControl(sim.client, observation);
  // Which unit each drawn instance belongs to (squads draw one per soldier).
  const instanceUnits = useRef<number[]>([]);
  const selectedRef = useRef(control.selected);
  selectedRef.current = control.selected;

  const surfaceZ = useCallback(
    (x: number, y: number) => world?.view.surface_at(x, y)[0] ?? 0,
    [world],
  );

  const frameInstances = useCallback(
    (now: number): SceneInstance[] | null => {
      const poses = sim.interpolator.current?.sample(now);
      if (!poses || !observation) return null;
      const kinds = new Map(observation.own.map((u) => [u.id, u.kind]));
      const out: SceneInstance[] = [];
      const owners: number[] = [];
      for (const pose of poses) {
        const kind = kinds.get(pose.id);
        if (!kind) continue;
        const highlight = selectedRef.current.includes(pose.id);
        const color = SIDE_COLORS.blue;
        const bodies = pose.members.length ? pose.members : [pose.position];
        for (const p of bodies) {
          out.push({
            kind: proxyForUnit(kind),
            x: p[0],
            y: p[1],
            z: p[2],
            yaw: pose.yaw,
            color,
            highlight,
          });
          owners.push(pose.id);
        }
      }
      instanceUnits.current = owners;
      return out;
    },
    [observation, sim.interpolator],
  );

  const overlay = useMemo(
    () =>
      world &&
      buildOrderOverlay(
        (observation?.own ?? []).filter((u) => control.selected.includes(u.id)),
        surfaceZ,
      ),
    [world, observation, control.selected, surfaceZ],
  );

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

  const onBox = useCallback(
    (box: LabBox) =>
      control.selectInRect((u) => {
        const p = box.project(u.position[0], u.position[1], u.position[2] + 1);
        return !!p && p[0] >= box.x0 && p[0] <= box.x1 && p[1] >= box.y0 && p[1] <= box.y1;
      }, box.shift),
    [control],
  );

  const runDemo = useCallback(
    async (name: string) => {
      if (!observation) return;
      const steps = DEMOS[name](observation.own);
      control.setSelected([
        ...new Set(steps.flatMap((s) => ("units" in s.order ? s.order.units : []))),
      ]);
      for (const step of steps) await control.issue(step.order, step.queued ?? false);
    },
    [observation, control],
  );

  const diagnostics = useMemo(
    () => ({
      tick: () => observation?.tick ?? 0,
      observation: () => observation,
      acks: () => control.acks,
      selected: () => control.selected,
      select: (ids: number[]) => control.setSelected(ids),
      command: (order: Order, queued = false) => control.issue(order, queued),
      demo: (name: string) => runDemo(name),
      pause: () => sim.client?.pause(),
      resume: () => sim.client?.resume(),
      advance: (n: number) => sim.client!.advance(n),
    }),
    [observation, control, runDemo, sim.client],
  );

  if (!meshes || !overlay) return null;
  const tick = observation?.tick ?? 0;
  return (
    <>
      <LabViewport
        fixture="movement"
        world={meshes}
        overlay={overlay}
        instances={[]}
        frameInstances={frameInstances}
        initialCamera={MOVEMENT_CAMERA}
        onPick={onPick}
        onBox={onBox}
        onReady={sim.onViewportReady}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="movement-panel">
        <strong>Movement</strong>
        <div className="lab-hint">
          Click/drag: select · Right‑click: move · Double right‑click: fast move · Shift: queue · S:
          stop · Arrows/screen edge: pan
        </div>
        <div>
          Tick {tick} · {sim.status}
          {tick < 150 ? ` · road wall appears at tick 150` : " · road wall is down"}
        </div>
        <div className="lab-row">
          {Object.keys(DEMOS).map((name) => (
            <button key={name} type="button" onClick={() => void runDemo(name)}>
              {name}
            </button>
          ))}
          <button type="button" onClick={control.stop} disabled={!control.selected.length}>
            Stop
          </button>
          <button type="button" onClick={sim.reset}>
            Reset
          </button>
        </div>
        <div className="lab-hint">
          Route colours: <span className="lab-shortest">shortest</span> ·{" "}
          <span className="lab-fastest">fastest</span> ·{" "}
          <span className="lab-rejected">blocked</span> ·{" "}
          <span className="lab-waiting">waiting</span>
        </div>
        <ul className="lab-log" data-testid="selection">
          {control.selectedUnits.length === 0 && <li>No unit selected</li>}
          {control.selectedUnits.map((u) => (
            <li key={u.id}>{describeUnit(u, control.unitName)}</li>
          ))}
        </ul>
        <div className="lab-hint">Commands, newest first</div>
        <ul className="lab-log" data-testid="ack-log">
          {control.acks.map((a) => (
            <AckLine key={a.seq} entry={a} />
          ))}
        </ul>
      </aside>
    </>
  );
}

function describeUnit(u: OwnUnitView, name: (id: number) => string): string {
  const who = `${u.kind} #${u.id}`;
  const queued = u.queue.length ? `, then ${u.queue.length} more` : "";
  switch (u.state) {
    case "route_blocked":
      return `${who}: route blocked — no known way to (${u.goal!.map((v) => v.toFixed(0)).join(", ")}); keeps the order`;
    case "waiting":
      return `${who}: waiting for ${u.blocker === null ? "traffic" : name(u.blocker)}${queued}`;
    case "moving":
      return `${who}: ${u.policy} route to (${u.goal!.map((v) => v.toFixed(0)).join(", ")})${queued}`;
    default:
      return `${who}: holding`;
  }
}
