// A played (or replayed) battle for blue: the world, the side's units and
// every overlay, and the HUD: the top bar's readout, the unit card and
// command bar, subtitles, and the pause menu. Routes compose it with their
// own readout (the village's objective and clock, a lab's telemetry) and
// pause menu items (the scenario, replays, a lab's switches).
import { useEffect, useMemo, useRef, useState, useSyncExternalStore, type ReactNode } from "react";
import { soundSettings } from "@packages/battle-audio/src/settings";
import { RejectedOrder } from "@web/battle/present/rejectedOrder";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { CommandBar, ReadoutLayer, SelectionCard } from "@web/battle/present/readouts";
import { CaptionList, useCaptions } from "@web/battle/present/captions";
import { buildBattleOverlay, type BattleOverlayScenario } from "./battleOverlay";
import { borderWidthM, buildMapBorder } from "@packages/battle-renderer/src/playAreaOverlay";
import { metresPerPxAt } from "@packages/renderer-core/src/camera3d";
import { villageMapBorder } from "./villageFog";
import { villageStroke } from "./villageOverlay";
import { LabViewport } from "./LabViewport";
import { MenuButton, PauseMenu, usePauseMenu } from "./PauseMenu";
import { useBattleSession, type BattleSession } from "./useBattleSession";
import type { ScriptedSim } from "./useSimSession";
import type { ViewportPilot } from "./LabViewport";
import { useFeed } from "./feed";
import { RulerPaint, rulerAt } from "./rulerFeed";
import {
  RangeRulerLabels,
  type RangeRulerLabelsHandle,
} from "@web/battle/present/rangeRulerLabels";

/** Zoom steps for the marks whose strokes are sized on screen (the border,
 *  the orders, the ruler): distance = ZOOM_BASE ** step. */
const ZOOM_BASE = 1.25;
const zoomStep = (distance: number) => Math.round(Math.log(distance) / Math.log(ZOOM_BASE));

export function BattleView({
  fixture,
  scenario,
  seed,
  replay,
  scripted,
  camera,
  status,
  menu,
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
  /** The top bar's readout: what the player tracks while playing. */
  status: (session: BattleSession) => ReactNode;
  /** The route's own pause menu items. */
  menu?: (session: BattleSession) => ReactNode;
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
  const cues = useCaptions();
  const { note: noteCues } = cues;
  const session = useBattleSession({
    map: parsed.map,
    scenario,
    seed,
    onDecoded: noteCues,
    replay,
    scripted,
    destroyable: "apart",
    sound: true,
  });
  const input = !replay && !scripted;
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  // The range ruler (Space held with a selection): its paint and its text,
  // both following the pointer every frame.
  const [rulerPaint] = useState(() => new RulerPaint());
  const rulerLabels = useRef<RangeRulerLabelsHandle>(null);
  const { clear: clearCues } = cues;
  const { subtitles } = useSyncExternalStore(soundSettings.subscribe, soundSettings.get);
  const pause = usePauseMenu(sim.client, control.mode !== "move");
  const { audio } = session;
  useEffect(() => {
    clearCues();
    audio?.reset();
  }, [sim.client, clearCues, audio]);

  const border = useMemo(
    () =>
      world
        ? buildMapBorder(
            parsed.size,
            villageMapBorder,
            borderWidthM(villageMapBorder, villageStroke(metresPerPx)),
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
            control.selected,
            surfaceZ,
            parsed.drawn,
            { showOrders: control.showOrders, reveal: session.revealed },
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
      session.revealed,
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
        pointerMarks={rulerPaint.feed}
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
        onFrame={(project, view, pointerRay) => {
          session.hear(view);
          const ruler =
            input && control.showOrders && world
              ? rulerAt(
                  pointerRay(),
                  world,
                  control.selectedUnits,
                  session.drawnAt.current,
                  session.rules,
                  surfaceZ,
                )
              : null;
          rulerPaint.update(ruler, surfaceZ, metresPerPx);
          rulerLabels.current?.place(project, ruler?.ruler ?? null);
          const step = zoomStep(view.distance);
          if (step !== zoomRef.current) {
            zoomRef.current = step;
            setZoom(step);
          }
          session.placePanels(project, view.distance);
        }}
        diagnostics={{
          ...session.probes,
          transcript: () => cues.transcript.current,
          audio: () => session.audio?.stats() ?? null,
          /** The range ruler shown last frame (Space held with a selection). */
          ruler: () => rulerPaint.shown,
          ...diagnostics?.(session),
        }}
      />
      <ReadoutLayer
        own={observation?.own ?? []}
        identified={observation?.identified}
        contacts={observation?.contacts}
        tick={observation?.tick}
        rules={session.rules}
        selected={control.selected}
        handle={session.readouts}
      />
      <RangeRulerLabels handle={rulerLabels} />
      {/* The HUD: the route's readout at the top, the menu button, and,
          while something is selected, a strategy game's command bar along
          the bottom: the selection's unit card and its commands. */}
      <div className="hud" data-testid="battle-panel">
        <header className="hud-panel hud-top" data-occludes-readouts>
          {status(session)}
          {sim.error && (
            <div className="hud-error" data-testid="error">
              {sim.error}
            </div>
          )}
        </header>
        <MenuButton onOpen={() => pause.show(true)} />
        {control.selectedUnits.length > 0 && (
          <footer className="hud-panel hud-bar hud-bottom" data-occludes-readouts>
            <SelectionCard
              units={control.selectedUnits}
              own={observation?.own ?? []}
              rules={session.rules}
            />
            {input && (
              <CommandBar
                mode={control.mode}
                setMode={control.setMode}
                selected={control.selectedUnits}
                onStop={control.stop}
                onTogglePolicy={control.togglePolicy}
                onToggleDeployment={control.toggleDeployment}
                onExit={control.exitBuilding}
              />
            )}
          </footer>
        )}
        {subtitles && <CaptionList captions={cues} />}
      </div>
      {input && <RejectedOrder acks={control.acks} />}
      {pause.open && (
        <PauseMenu
          onClose={() => pause.show(false)}
          onRestart={
            scripted?.pilot
              ? undefined
              : () => {
                  pause.show(false);
                  sim.reset();
                }
          }
        >
          {menu?.(session)}
        </PauseMenu>
      )}
    </>
  );
}
