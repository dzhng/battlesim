// /lab/city-ruins: buildings destroyed in a battle, drawn as each side knows
// them. The camera lab's map with its saved shelling encounter
// (`fixtures/maps/camera-lab/encounters/shelling.json`): HE bursts scattered
// over the five-floor U block's yard bring it down to remains, and more in
// front of the twenty-floor tower gut it, which stands. Blue's squad watches both from between them. Red's stands behind
// the two slabs to the east and sees neither, so for red both stand intact
// until its scripted walk takes it round the slabs.
//
// A building is drawn in the state the observed side was published of it
// (`fallenBuildings`): its template's ruin, its gutted shell, or intact. The
// camera's obstacles, the fog's occluders and the smoke over what is
// destroyed follow the same knowledge.
import { useEffect, useMemo, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { CameraController } from "@packages/renderer-core/src/cameraController";
import type { SideName } from "@web/battle/sim/protocol";
import { tierBoundaries } from "../buildingTier";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { LabViewport } from "../LabViewport";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { TickStatus } from "../TickStatus";
import { useBattleSession } from "../useBattleSession";

const SEED = 7;
/** Both shelled buildings and blue's squad between them, from the south,
 *  with room over the tower for its smoke. */
const OPENING: Camera3DParams = {
  target: [330, 270, 0],
  distance: 430,
  pitch: 0.6,
  yaw: -1.57,
  ...gameCamera.lens,
};

export default function CityRuins() {
  return (
    <SavedEncounter map="camera-lab" encounter="shelling">
      {(battle) => <Shelling battle={battle} />}
    </SavedEncounter>
  );
}

function Shelling({ battle }: { battle: SavedBattle }) {
  const [side, setSide] = useState<SideName>("blue");
  const session = useBattleSession({ ...battle, seed: SEED, side });
  const { meshes, sim } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  useEffect(() => sim.client?.observeAs(side), [sim.client, side]);

  // The rig for this map: its wheel reaches out to the whole map.
  const cameraConfig = useMemo(() => gameCamera.forMap(battle.map.size), [battle]);
  const overview = useMemo(
    () => ({
      target: [battle.map.size[0] / 2, battle.map.size[1] / 2],
      distance: cameraConfig.zoom_max,
      pitch: new CameraController(cameraConfig).pitchAt(cameraConfig.zoom_max),
      yaw: gameCamera.opening().yaw,
    }),
    [battle, cameraConfig],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    ...session.probes,
    side: () => side,
    // Sent now, so a following advance already publishes the new side.
    setSide: (next: SideName) => {
      sim.client?.observeAs(next);
      setSide(next);
    },
    /** How far off a building changes to tier 1, 2 and 3 in this window. */
    boundaries: tierBoundaries,
    /** The whole-map view, as the rig frames it. */
    overview: () => overview,
  };

  if (!meshes) return null;
  const destroyed = session.probes.buildings().filter((b) => b.fallen);
  return (
    <>
      <LabViewport
        fixture="city-ruins"
        world={worldFeed}
        structures={session.structures}
        buildings={session.buildingsFeed}
        obstacles={session.cameraObstaclesFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={OPENING}
        cameraConfig={cameraConfig}
        groundAt={session.surfaceZ}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-testid="city-ruins-panel">
        <strong>Destroyed buildings, as a side knows them</strong>
        <div className="lab-warning" data-testid="viewing-as">
          Viewing as {side.toUpperCase()} — diagnostic side switch
        </div>
        <div className="lab-row">
          {(["blue", "red"] as const).map((s) => (
            <button
              key={s}
              type="button"
              aria-pressed={side === s}
              onClick={() => diagnostics.setSide(s)}
            >
              {s === "blue" ? "Blue view" : "Red view"}
            </button>
          ))}
          <button type="button" onClick={sim.restart}>
            Reset
          </button>
        </div>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
        <div className="lab-hint">
          Blue watches the shelling; red is behind the slabs to the east until it walks out.
        </div>
        <ul className="lab-log" data-testid="known-destroyed">
          {destroyed.length === 0 && <li>{side} knows every building intact</li>}
          {destroyed.map((b) => (
            <li key={b.owner}>
              {b.template}: {b.state}
            </li>
          ))}
        </ul>
      </aside>
    </>
  );
}
