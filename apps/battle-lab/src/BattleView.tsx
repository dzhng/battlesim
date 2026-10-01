// A played (or replayed) battle for blue: the world, the side's units and
// every overlay, and the HUD: the top bar's readout, the unit card and
// command bar, subtitles, and the pause menu. Routes compose it with their
// own readout (the village's objective and clock, a lab's telemetry) and
// pause menu items (the scenario, replays, a lab's switches).
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { RejectedOrder } from "@web/battle/present/rejectedOrder";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { CameraPresentation } from "@packages/renderer-core/src/cameraController";
import { CommandBar, ReadoutLayer, SelectionCard } from "@web/battle/present/readouts";
import { CaptionList, useCaptions } from "@web/battle/present/captions";
import { buildBattleOverlay, type BattleOverlayScenario } from "./battleOverlay";
import { borderWidthM, buildMapBorder } from "@packages/battle-renderer/src/playAreaOverlay";
import { metresPerPxAt } from "@packages/renderer-core/src/camera3d";
import { gameMapBorder } from "./gameFog";
import { gameStroke } from "./gameOverlay";
import { LabViewport } from "./LabViewport";
import { MenuButton, PauseMenu, usePauseMenu } from "./PauseMenu";
import { useBattleSession, type BattleSession } from "./useBattleSession";
import type { ScriptedSim } from "./useSimSession";
import type { ViewportPilot } from "./LabViewport";
import { useFeed } from "./feed";
import { PointerPaint, rulerAt, movePreviewAt } from "./pointerPaint";
import {
  RangeRulerLabels,
  type RangeRulerLabelsHandle,
} from "@web/battle/present/rangeRulerLabels";

/** What has finished loading: the static world's meshes are built
 *  (`world`), the viewport has drawn its first frame (`renderer`), and the
 *  battle's first observation has arrived (`playable`). */
export type BattleLoadStage = "world" | "renderer" | "playable";

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
  cameraConfig,
  status,
  menu,
  diagnostics,
  cover,
  onLoadStage,
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
  /** The camera rig's numbers; the game's when omitted. */
  cameraConfig?: CameraPresentation;
  /** The top bar's readout: what the player tracks while playing. */
  status: (session: BattleSession) => ReactNode;
  /** The route's own pause menu items. */
  menu?: (session: BattleSession) => ReactNode;
  /** Route-specific lab probes, merged into the shared ones. */
  diagnostics?: (session: BattleSession) => Record<string, unknown>;
  /** A loading screen shown over the view until the battle is playable. */
  cover?: ReactNode;
  /** Each stage of loading, once, as it completes. */
  onLoadStage?: (stage: BattleLoadStage) => void;
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
  // Cursor paint updates independently of observations, including while paused.
  const [pointerPaint] = useState(() => new PointerPaint());
  const rulerLabels = useRef<RangeRulerLabelsHandle>(null);
  const { clear: clearCues } = cues;
  const pause = usePauseMenu(sim.client);
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
            gameMapBorder,
            borderWidthM(gameMapBorder, gameStroke(metresPerPx)),
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

  // Loading: the static world's meshes, then the viewport's first frame, then
  // the battle's first observation, which is when the player can act.
  const [viewportReady, setViewportReady] = useState(false);
  const loadStage: BattleLoadStage | null = !meshes
    ? null
    : !viewportReady
      ? "world"
      : !observation
        ? "renderer"
        : "playable";
  const onLoadStageRef = useRef(onLoadStage);
  onLoadStageRef.current = onLoadStage;
  useEffect(() => {
    if (loadStage) onLoadStageRef.current?.(loadStage);
  }, [loadStage]);

  if (!meshes) return cover ?? null;
  return (
    <>
      <LabViewport
        fixture={fixture}
        world={worldFeed}
        structures={session.structures}
        massing={session.massingFeed}
        obstacles={session.cameraObstaclesFeed}
        overlay={overlayFeed}
        pointerMarks={pointerPaint.feed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={camera}
        cameraConfig={cameraConfig}
        groundAt={surfaceZ}
        onPick={scripted ? undefined : session.onPick}
        onBox={scripted ? undefined : session.onBox}
        onReady={(gpu) => {
          session.onReady(gpu);
          setViewportReady(true);
        }}
        pilot={scripted?.pilot}
        onFrame={(project, view, pointer) => {
          session.hear(view);
          const ruler =
            input && control.showOrders && world
              ? rulerAt(
                  pointer.ray,
                  world,
                  control.selectedUnits,
                  session.drawnAt.current,
                  session.rules,
                  surfaceZ,
                )
              : null;
          const heldMove =
            input && control.mode === "move" && world
              ? movePreviewAt(
                  pointer.rightPress,
                  pointer.rightDragging ? pointer.ray : null,
                  world,
                  control.selectedUnits,
                )
              : null;
          const pending = session.pendingMove.current;
          const accepted =
            pending &&
            control.acks.find(
              ({ order }) => order.kind === "move" && order.gesture === pending.gesture,
            )?.ack;
          const awaiting = pending && !accepted;
          const move = heldMove ?? (awaiting ? pending : null);
          let preview = pointerPaint.resolveMove(
            move,
            control.selectedUnits,
            session.sim.client,
            observation?.tick ?? 0,
          );
          if (!heldMove && accepted?.placement) {
            const applied = (observation?.tick ?? 0) >= accepted.applied_tick;
            preview = pointerPaint.markers(
              accepted.placement.destinations.filter((mark) => !applied || !mark.placed),
              observation?.own ?? [],
              session.revealed,
            );
          }
          pointerPaint.update(ruler, preview, surfaceZ, metresPerPx);
          rulerLabels.current?.place(project, ruler?.ruler ?? null);
          const step = zoomStep(view.distance);
          if (step !== zoomRef.current) {
            zoomRef.current = step;
            setZoom(step);
          }
          session.placePanels(project, view, pointer);
        }}
        diagnostics={{
          ...session.probes,
          audio: () => session.audio?.stats() ?? null,
          /** The range ruler shown last frame (Space held with a selection). */
          ruler: () => pointerPaint.shown,
          movePreview: () => pointerPaint.preview,
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
            {input && <CommandBar control={control} />}
          </footer>
        )}
        <CaptionList captions={cues} />
      </div>
      {input && <RejectedOrder acks={control.acks} />}
      {loadStage !== "playable" && cover}
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
