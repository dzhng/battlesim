import { useEffect, useMemo, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { SideName } from "@web/battle/sim/protocol";
import { buildSightOverlay } from "@packages/battle-renderer/src/sightOverlay";
import sensorsMap from "@fixtures/sensors-lab.json";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario, type LabScript } from "../scenarios";

// Blue watches from open ground west of the thin forest. Red's scripted tank
// tours thin forest, deep forest, the ridge's far side and the building's
// shadow; a red squad walks into the thin forest's edge. Nobody opens fire:
// this lab is about sight, not combat. The sight-lobe overlay outlines how
// far each own unit's eyes reach in every direction, with its forward line.
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
    { side: "blue", kind: "recon", position: [300, 120], engagement: "return_fire_only" },
    { side: "blue", kind: "rifle", position: [300, 170], engagement: "return_fire_only" },
    { side: "blue", kind: "tank", position: [300, 70], engagement: "return_fire_only" },
    { side: "blue", kind: "rifle", position: [420, 470], engagement: "return_fire_only" },
    {
      side: "red",
      kind: "tank",
      position: [480, 110],
      yaw: Math.PI,
      engagement: "return_fire_only",
    },
    { side: "red", kind: "rifle", position: [470, 60], engagement: "return_fire_only" },
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
const NO_LOBES = { opaque: new Float32Array(0), translucent: new Float32Array(0) };

const SENSORS_CAMERA: Camera3DParams = {
  target: [640, 290, 0],
  distance: 640,
  pitch: 1.0,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

export default function Sensors() {
  const [side, setSide] = useState<SideName>("blue");
  const [fogOn, setFogOn] = useState(true);
  const [lobesOn, setLobesOn] = useState(true);
  const session = useBattleSession({ map: sensorsMap, scenario: SCENARIO, seed: SEED, side });
  const { meshes, sim, surfaceZ } = session;
  const { observation } = sim;

  useEffect(() => sim.client?.observeAs(side), [sim.client, side]);

  const fog = fogOn ? session.fog : null;

  const overlay = useMemo(
    () =>
      lobesOn && observation
        ? buildSightOverlay(
            observation.own.map((u) => u.sight),
            surfaceZ,
          )
        : NO_LOBES,
    [lobesOn, observation, surfaceZ],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    ...session.probes,
    // Sent now, so a following advance already publishes the new side.
    setSide: (next: SideName) => {
      sim.client?.observeAs(next);
      setSide(next);
    },
    setFog: setFogOn,
    setLobes: setLobesOn,
  };

  if (!meshes) return null;
  const own = observation?.own ?? [];
  const identified = observation?.identified ?? [];
  return (
    <>
      <LabViewport
        fixture="sensors"
        world={meshes}
        overlay={overlay}
        fog={fog}
        instances={[]}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={SENSORS_CAMERA}
        onReady={session.onReady}
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
          <label>
            <input
              type="checkbox"
              checked={lobesOn}
              onChange={(e) => setLobesOn(e.target.checked)}
            />
            Sight lobes
          </label>
        </div>
        {lobesOn && (
          <div className="lab-hint" data-testid="lobe-key">
            Cyan lobe: vehicle sight, farthest along the arrow. White ring: even 360° infantry
            sight.
          </div>
        )}
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
        <div className="lab-hint">Own units: what each unit's own sensors identify</div>
        <ul className="lab-log" data-testid="own-sensors">
          {own.map((u) => (
            <li key={u.id}>
              {u.kind} #{u.id} ({Math.round(u.sight.range * u.sight.shape.front)} m ahead):{" "}
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
