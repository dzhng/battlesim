import { gameStroke, OPENING_METRES_PER_PX } from "../gameOverlay";
import { useMemo, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import { ArmyDeck } from "@web/battle/present/armyDeck";
import { ReadoutLayer } from "@web/battle/present/readouts";
import { AckLog } from "../AckLog";
import { orderLayer, tracerLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { useUnitFocus } from "../useUnitFocus";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "@web/feed";
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
  const { cameraFocus, focusUnit } = useUnitFocus();
  const worldFeed = useFeed(meshes);
  const { observation } = sim;

  const [renderScale, setRenderScale] = useState(1);
  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const orders = orderLayer(
      session.units,
      session.rules.air,
      observation,
      control.selected,
      session.revealed,
      surfaceZ,
      gameStroke(OPENING_METRES_PER_PX, renderScale),
    );
    const tracers = tracerLayer(observation);
    return combineWorldMeshes([orders, tracers]);
  }, [
    renderScale,
    world,
    observation,
    surfaceZ,
    control.selected,
    session.revealed,
    session.units,
    session.rules.air,
  ]);
  const overlayFeed = useFeed(overlay);

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = { ...session.probes, mode: () => control.mode };

  const armyState = useFeed({ own: observation?.own ?? [], control });

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="readouts"
        world={worldFeed}
        buildings={session.known.buildings}
        overlay={overlayFeed}
        fog={session.fogFeed}
        {...session.viewport}
        initialCamera={READOUTS_CAMERA}
        {...session.input}
        cameraFocus={cameraFocus}
        onFrame={(project, camera, pointer, scale) => {
          setRenderScale(scale);
          session.placePanels(project, camera, pointer);
        }}
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
          state={armyState}
          selected={control.selected}
          onSelect={control.setSelected}
          onCenter={focusUnit}
          rules={session.rules}
          captions={null}
        />
      </div>
    </>
  );
}
