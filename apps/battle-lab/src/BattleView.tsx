// A played (or replayed) battle for blue: the world, the side's units and
// every overlay, the production readouts, selection panel and command bar.
// Routes compose it with their own panel content (the village's hold status
// and replay controls, the endurance lab's telemetry).
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import {
  CommandBar,
  ReadoutLayer,
  SelectionPanel,
  type ReadoutLayerHandle,
} from "@web/battle/present/readouts";
import { CaptionList, useCaptions } from "@web/battle/present/captions";
import type { ObservationView } from "@web/battle/sim/observation";
import { AckLog } from "./AckLog";
import { BattleMemory, buildBattleOverlay, type BattleOverlayScenario } from "./battleOverlay";
import { borderWidthM, buildMapBorder } from "@packages/battle-renderer/src/playAreaOverlay";
import { metresPerPxAt } from "@packages/renderer-core/src/camera3d";
import { villageMapBorder } from "./villageFog";
import { LabViewport } from "./LabViewport";
import { SoundControls } from "./SoundControls";
import { useBattleSession, type BattleSession } from "./useBattleSession";
import type { ScriptedSim } from "./useSimSession";
import type { ViewportPilot } from "./LabViewport";
import { useFeed } from "./feed";

/** Zoom steps for the lines drawn a fixed width on screen (the border, the
 *  orders): distance = ZOOM_BASE ** step. */
const ZOOM_BASE = 1.25;
const zoomStep = (distance: number) => Math.round(Math.log(distance) / Math.log(ZOOM_BASE));

export function BattleView({
  fixture,
  scenario,
  seed,
  replay,
  scripted,
  camera,
  title,
  panel,
  diagnostics,
}: {
  fixture: string;
  /** The scenario JSON the authority runs; the view draws its map. */
  scenario: string;
  seed: number;
  /** A recorded battle to replay: input is off. */
  replay?: string;
  /** A scripted run: a script plays blue and input is off. The benchmark's
   *  pilot also flies the camera and measures every frame; without one the
   *  camera is the player's (watching the script play). */
  scripted?: ScriptedSim & { pilot?: ViewportPilot };
  camera: Camera3DParams;
  title: string;
  /** Route panel content under the title. */
  panel: (session: BattleSession) => ReactNode;
  /** Route-specific lab probes, merged into the shared ones. */
  diagnostics?: (session: BattleSession) => Record<string, unknown>;
}) {
  const parsed = useMemo(() => {
    const s = JSON.parse(scenario) as {
      map: { size: [number, number] };
      rules: { service: { radius_m: number } };
      encounter: { success_zone_center: [number, number]; success_zone_radius_m: number } | null;
    };
    const drawn: BattleOverlayScenario = {
      supplyRadius: s.rules.service.radius_m,
      zone: s.encounter && {
        center: s.encounter.success_zone_center,
        radius: s.encounter.success_zone_radius_m,
      },
    };
    return { map: s.map, size: s.map.size, drawn };
  }, [scenario]);
  // The border and the orders are rebuilt only when the zoom crosses a step
  // (×1.25), so their widths hold near their `_px` widths on screen.
  const [zoom, setZoom] = useState(() => zoomStep(camera.distance));
  const zoomRef = useRef(zoom);
  const metresPerPx = metresPerPxAt(ZOOM_BASE ** zoom, camera.fovY, window.innerHeight);
  const memory = useRef(new BattleMemory());
  const cues = useCaptions();
  const { note: noteCues } = cues;
  const onDecoded = useCallback(
    (o: ObservationView) => {
      memory.current.note(o);
      noteCues(o);
    },
    [noteCues],
  );
  const session = useBattleSession({
    map: parsed.map,
    scenario,
    seed,
    onDecoded,
    replay,
    scripted,
    destroyable: "apart",
    sound: true,
  });
  const input = !replay && !scripted;
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  const readouts = useRef<ReadoutLayerHandle>(null);
  const { clear: clearCues } = cues;
  const { audio } = session;
  useEffect(() => {
    memory.current.clear();
    clearCues();
    audio?.reset();
  }, [sim.client, clearCues, audio]);

  const border = useMemo(
    () =>
      world
        ? buildMapBorder(
            parsed.size,
            villageMapBorder,
            borderWidthM(villageMapBorder, metresPerPx),
            surfaceZ,
          )
        : null,
    [world, parsed.size, metresPerPx, surfaceZ],
  );
  const overlay = useMemo(
    () =>
      world && observation
        ? buildBattleOverlay(
            observation,
            memory.current,
            control.selected,
            surfaceZ,
            parsed.drawn,
            control.showOrders,
            border,
            metresPerPx,
          )
        : undefined,
    [
      world,
      observation,
      surfaceZ,
      control.selected,
      parsed.drawn,
      control.showOrders,
      border,
      metresPerPx,
    ],
  );
  const overlayFeed = useFeed(overlay);

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture={fixture}
        world={worldFeed}
        structures={session.structures}
        overlay={overlayFeed}
        fog={session.fogFeed}
        instances={[]}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={camera}
        groundAt={surfaceZ}
        onPick={scripted ? undefined : session.onPick}
        onBox={scripted ? undefined : session.onBox}
        onReady={session.onReady}
        pilot={scripted?.pilot}
        onFrame={(project, view) => {
          session.hear(view);
          const step = zoomStep(view.distance);
          if (step !== zoomRef.current) {
            zoomRef.current = step;
            setZoom(step);
          }
          readouts.current?.place(
            project,
            view.distance,
            session.drawnAt.current,
            session.drawnClock.current,
          );
        }}
        diagnostics={{
          ...session.probes,
          transcript: () => cues.transcript.current,
          ...diagnostics?.(session),
        }}
      />
      <ReadoutLayer
        observation={observation}
        rules={session.rules}
        selected={control.selected}
        handle={readouts}
      />
      {/* The HUD: a slim top bar for the battle's status and
          controls, and a strategy game's command bar along the bottom: the
          selection's unit card, the command grid, and what was heard and
          ordered. */}
      <div className="hud" data-testid="battle-panel">
        <header className="lab-panel hud-bar hud-top" data-occludes-readouts>
          <strong>{title}</strong>
          {sim.error && (
            <div className="lab-rejected" data-testid="error">
              {sim.error}
            </div>
          )}
          {panel(session)}
          <SoundControls />
        </header>
        <footer className="lab-panel hud-bar hud-bottom" data-occludes-readouts>
          <section className="hud-card" aria-label="Selection">
            <SelectionPanel units={control.selectedUnits} rules={session.rules} />
          </section>
          {input && (
            <CommandBar
              mode={control.mode}
              setMode={control.setMode}
              selected={control.selectedUnits}
              onStop={control.stop}
              onTogglePolicy={control.togglePolicy}
              onDeploy={control.setDeployment}
              onExit={control.exitBuilding}
            />
          )}
          <section className="hud-feed" aria-label="Radio and command log">
            <CaptionList captions={cues} />
            {input && <AckLog acks={control.acks} />}
          </section>
        </footer>
      </div>
    </>
  );
}
