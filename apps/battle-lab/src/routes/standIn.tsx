import { useEffect } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { LabViewport } from "../LabViewport";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useBattleSession } from "../useBattleSession";
import { useFeed } from "@web/feed";
import { gameCamera } from "../gameCamera";
import { withholdRuntimeAppearances } from "../gameAppearances";

const SEED = 10;
const CAMERA: Camera3DParams = {
  target: [350, 180, 0],
  distance: 330,
  pitch: 0.9,
  yaw: -1.57,
  ...gameCamera.lens,
};

/** Slice 10: the sensors encounter with the vehicle and squad art omitted
 * from the runtime catalog. Reloading with `release=1` admits the same battle
 * art and lets the canonical blocks disappear on the next catalog commit. */
export default function StandIn() {
  const released = new URLSearchParams(window.location.search).has("release");
  withholdRuntimeAppearances(released ? [] : ["test_tank", "test_rifle"]);
  useEffect(() => () => withholdRuntimeAppearances([]), []);
  return (
    <SavedEncounter fixture="stand-in" encounter="sensors">
      {(battle) => <StandInLab battle={battle} released={released} />}
    </SavedEncounter>
  );
}

function StandInLab({ battle, released }: { battle: SavedBattle; released: boolean }) {
  const session = useBattleSession({ ...battle, seed: SEED });
  const worldFeed = useFeed(session.meshes);
  if (!session.meshes) return null;
  return (
    <>
      <LabViewport
        fixture="stand-in"
        world={worldFeed}
        {...session.known}
        {...session.viewport}
        {...session.input}
        initialCamera={CAMERA}
        diagnostics={session.probes}
      />
      <aside className="hud-panel lab-panel" data-testid="stand-in-panel">
        <strong>Stand-in mechanics</strong>
        <div data-testid="stand-in-state">
          {released ? "Model art released" : "Vehicle and squad art withheld"}
        </div>
        <div className="lab-hint">
          Click a block to select it. Right-click an enemy block to order an attack.
        </div>
        {!released && (
          <button type="button" onClick={() => window.location.assign("/labs/stand-in?release=1")}>
            Release art
          </button>
        )}
      </aside>
    </>
  );
}
