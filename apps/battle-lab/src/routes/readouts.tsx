import { useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import { ArmyDeck } from "@web/battle/present/armyDeck";
import { ReadoutLayer } from "@web/battle/present/readouts";
import { AckLog } from "../AckLog";
import { orderLayer, tracerLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// A tank (cannon with AP/HE, and an HMG), an AT team, a rifle squad and a
// supply truck setting up, facing a red tank: every kind of timer runs at once.
const SEED = 14;

const READOUTS_CAMERA: Camera3DParams = {
  target: [215, 230, 0],
  distance: 400,
  pitch: 0.95,
  yaw: -1.57,
  ...gameCamera.lens,
};

export default function Readouts() {
  return (
    <SavedEncounter fixture="readouts" encounter="readouts">
      {(battle) => <ReadoutsLab battle={battle} />}
    </SavedEncounter>
  );
}

function ReadoutsLab({ battle }: { battle: SavedBattle }) {
  const session = useBattleSession({ ...battle, seed: SEED });
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const orders = orderLayer(
      session.units,
      observation,
      control.selected,
      session.revealed,
      surfaceZ,
    );
    const tracers = tracerLayer(observation);
    return combineWorldMeshes([orders, tracers]);
  }, [world, observation, surfaceZ, control.selected, session.revealed, session.units]);
  const overlayFeed = useFeed(overlay);

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = { ...session.probes, mode: () => control.mode };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="readouts"
        world={worldFeed}
        buildings={session.buildingsFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={READOUTS_CAMERA}
        onPick={session.onPick}
        onRightPress={session.onRightPress}
        onCursor={session.onCursor}
        pointerMarks={session.pointerPaint.feed}
        onBox={session.onBox}
        onReady={session.onReady}
        onFrame={session.placePanels}
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
        <TickStatus tick={observation?.tick} status={sim.status.status} />
        <div className="lab-legend">
          Rings: <span className="lab-swatch lab-swatch-aim" /> aim ·{" "}
          <span className="lab-swatch lab-swatch-reload" /> reload (dashed) · number: rounds left ·
          the mark after a weapon: why it cannot fire
        </div>
        <AckLog acks={control.acks} />
      </aside>
      <div className="hud">
        <ArmyDeck
          own={observation?.own ?? []}
          selected={control.selected}
          onSelect={control.setSelected}
          rules={session.rules}
          control={control}
          captions={null}
        />
      </div>
    </>
  );
}
