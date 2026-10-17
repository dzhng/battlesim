import { useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import { LabViewport } from "../LabViewport";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useBattleSession } from "../useBattleSession";
import { useFeed } from "../feed";
import { orderLayer, tracerLayer } from "../battleOverlay";
import { TickStatus } from "../TickStatus";
import { ReadoutLayer } from "@web/battle/present/readouts";
import { gameCamera } from "../gameCamera";

const CAMERA: Camera3DParams = {
  target: [320, 250, 0],
  distance: 380,
  pitch: 0.95,
  yaw: -1.57,
  ...gameCamera.lens,
};

export default function Transport() {
  return (
    <SavedEncounter fixture="transport" encounter="board">
      {(battle) => <TransportLab battle={battle} />}
    </SavedEncounter>
  );
}

function TransportLab({ battle }: { battle: SavedBattle }) {
  const session = useBattleSession({ ...battle, seed: 21 });
  const { meshes, surfaceZ, sim } = session;
  const { observation } = sim;
  const worldFeed = useFeed(meshes);
  const overlay = useMemo(() => {
    if (!observation) return undefined;
    return combineWorldMeshes([
      tracerLayer(observation),
      orderLayer(session.units, observation, session.control.selected, session.revealed, surfaceZ),
    ]);
  }, [observation, session.units, session.control.selected, session.revealed, surfaceZ]);
  const overlayFeed = useFeed(overlay);
  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="transport"
        world={worldFeed}
        {...session.known}
        overlay={overlayFeed}
        fog={session.fogFeed}
        {...session.viewport}
        initialCamera={CAMERA}
        {...session.input}
        onFrame={session.placePanels}
        diagnostics={session.probes}
      />
      <ReadoutLayer
        own={observation?.own ?? []}
        rules={session.rules}
        selected={session.control.selected}
        handle={session.readouts}
      />
      <aside className="hud-panel lab-panel" data-occludes-readouts data-testid="transport-panel">
        <strong>Passenger transport</strong>
        <div className="lab-hint">
          Board and Collect are scripted from both directions. Passengers leave the field while
          aboard.
        </div>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
      </aside>
    </>
  );
}
