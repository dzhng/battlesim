import { useEffect, useMemo, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { SideName } from "@web/battle/sim/protocol";
import { buildSightOverlay } from "@packages/battle-renderer/src/sightOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// The sensors map's saved encounter
// (`fixtures/maps/sensors/encounters/sensors.json`). Blue watches from open
// ground west of the thin forest. Red's scripted tank
// tours thin forest, deep forest, the ridge's far side and the building's
// shadow; a red squad walks into the thin forest's edge. Nobody opens fire:
// this lab is about sight, not combat. The sight-lobe overlay outlines how
// far each own unit's eyes reach in every direction, with its forward line.
const SEED = 5;
const NO_LOBES = { opaque: new Float32Array(0), translucent: new Float32Array(0) };

const SENSORS_CAMERA: Camera3DParams = {
  target: [640, 290, 0],
  distance: 640,
  pitch: 1.0,
  yaw: -1.57,
  ...gameCamera.lens,
};

export default function Sensors() {
  return (
    <SavedEncounter fixture="sensors" encounter="sensors">
      {(battle) => <SensorsLab battle={battle} />}
    </SavedEncounter>
  );
}

function SensorsLab({ battle }: { battle: SavedBattle }) {
  const [side, setSide] = useState<SideName>("blue");
  const [fogOn, setFogOn] = useState(true);
  const [lobesOn, setLobesOn] = useState(true);
  const session = useBattleSession({ ...battle, seed: SEED, side });
  const { meshes, sim, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;

  useEffect(() => sim.client?.observeAs(side), [sim.client, side]);

  const fog = fogOn ? session.fog : null;
  const fogFeed = useFeed(fog);

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
  const overlayFeed = useFeed(overlay);

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
        world={worldFeed}
        buildings={session.buildingsFeed}
        overlay={overlayFeed}
        fog={fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={SENSORS_CAMERA}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-testid="sensors-panel">
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
        <TickStatus tick={observation?.tick} status={sim.status.status} />
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
