import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import type { SideName } from "@web/battle/sim/protocol";
import sensorsMap from "@fixtures/sensors-lab.json";
import village from "@fixtures/village.json";
import { LabViewport } from "../LabViewport";
import { labScenario, type LabScript } from "../scenarios";
import { sideInstances } from "../sideInstances";
import { useSimSession } from "../useSimSession";
import { useStaticWorld } from "../useStaticWorld";

// Blue watches from open ground west of the thin forest. Red's scripted tank
// tours thin forest, deep forest, the ridge's far side and the building's
// shadow; a red squad walks into the thin forest's edge.
const route = (
  side: SideName,
  unit: number,
  points: [number, number][],
  gesture: number,
): LabScript[] =>
  points.map((goal, k) => ({
    tick: 30,
    side,
    queued: k > 0,
    order: { kind: "move", units: [unit], gesture, goal, route: "shortest" },
  }));
const SCENARIO = labScenario(
  sensorsMap,
  [
    { side: "blue", kind: "recon", position: [300, 120] },
    { side: "blue", kind: "rifle", position: [300, 170] },
    { side: "blue", kind: "tank", position: [300, 70] },
    { side: "blue", kind: "rifle", position: [420, 470] },
    { side: "red", kind: "tank", position: [480, 110] },
    { side: "red", kind: "rifle", position: [470, 60] },
  ],
  [],
  [
    ...route(
      "red",
      4,
      [
        [480, 300],
        [620, 300],
        [820, 440],
        [1100, 110],
        [480, 110],
      ],
      1,
    ),
    ...route(
      "red",
      5,
      [
        [420, 60],
        [470, 60],
      ],
      2,
    ),
  ],
);
const SEED = 5;

export const SENSORS_CAMERA: Camera3DParams = {
  target: [640, 290, 0],
  distance: 640,
  pitch: 1.0,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

export default function Sensors() {
  const world = useStaticWorld(sensorsMap);
  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface"),
    [world],
  );
  const [side, setSide] = useState<SideName>("blue");
  const [fogOn, setFogOn] = useState(true);
  const sim = useSimSession(SCENARIO, SEED);
  const { observation } = sim;
  const sideRef = useRef(side);
  sideRef.current = side;

  useEffect(() => sim.client?.observeAs(side), [sim.client, side]);

  const frameInstances = useCallback(
    (now: number): SceneInstance[] | null => {
      const poses = sim.interpolator.current?.sample(now);
      if (!poses || !observation) return null;
      return sideInstances(sideRef.current, poses, observation, []).instances;
    },
    [observation, sim.interpolator],
  );

  const fog = useMemo(() => (fogOn && observation ? observation.fog : null), [fogOn, observation]);

  const ranges: Record<string, number> = {
    recon: village.sensors.recon_ground_m,
    rifle: village.sensors.infantry_ground_m,
    at: village.sensors.infantry_ground_m,
    tank: village.sensors.tank_ground_m,
    supply: village.sensors.supply_ground_m,
  };

  const diagnostics = useMemo(
    () => ({
      tick: () => observation?.tick ?? 0,
      observation: () => observation,
      // Sent now, so a following advance already publishes the new side.
      setSide: (next: SideName) => {
        sim.client?.observeAs(next);
        setSide(next);
      },
      setFog: setFogOn,
      pause: () => sim.client?.pause(),
      resume: () => sim.client?.resume(),
      advance: (n: number) => sim.client!.advance(n),
    }),
    [observation, sim.client],
  );

  if (!meshes) return null;
  const own = observation?.own ?? [];
  const identified = observation?.identified ?? [];
  return (
    <>
      <LabViewport
        fixture="sensors"
        world={meshes}
        fog={fog}
        instances={[]}
        frameInstances={frameInstances}
        initialCamera={SENSORS_CAMERA}
        onReady={sim.onViewportReady}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="sensors-panel">
        <strong>Sensors</strong>
        <div className="lab-warning" data-testid="viewing-as">
          Viewing as {side.toUpperCase()} — diagnostic side switch
        </div>
        <div className="lab-row">
          {(["blue", "red"] as const).map((s) => (
            <button key={s} type="button" aria-pressed={side === s} onClick={() => setSide(s)}>
              {s === "blue" ? "Blue view" : "Red view"}
            </button>
          ))}
          <label>
            <input type="checkbox" checked={fogOn} onChange={(e) => setFogOn(e.target.checked)} />
            Ground fog
          </label>
        </div>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status}
        </div>
        <div className="lab-hint">Own units: what each unit's own sensors identify</div>
        <ul className="lab-log" data-testid="own-sensors">
          {own.map((u) => (
            <li key={u.id}>
              {u.kind} #{u.id} ({ranges[u.kind]} m):{" "}
              {u.sees.length ? u.sees.map((id) => `contact ${id}`).join(", ") : "nothing"}
            </li>
          ))}
        </ul>
        <div className="lab-hint">Team identification, shared with every unit</div>
        <ul className="lab-log" data-testid="identified">
          {identified.length === 0 && <li>No enemy identified</li>}
          {identified.map((e) => {
            const nearest = Math.min(
              ...own.map((u) =>
                Math.hypot(u.position[0] - e.position[0], u.position[1] - e.position[1]),
              ),
            );
            const seers = own.filter((u) => u.sees.includes(e.id)).map((u) => `${u.kind} #${u.id}`);
            return (
              <li key={e.id}>
                contact {e.id}: {e.kind}
                {e.members.length ? ` (${e.members.length} soldiers seen)` : ""} ·{" "}
                {nearest.toFixed(0)} m · seen by {seers.join(", ")}
              </li>
            );
          })}
        </ul>
      </aside>
    </>
  );
}
