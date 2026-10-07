import {
  ObjectiveMarkers,
  type ObjectiveMarkersHandle,
} from "@web/battle/present/objectiveMarkers";
// A played (or replayed) battle for blue: the world, the side's units and
// every overlay, and the HUD: the top bar's readout, the unit card and
// command bar, subtitles, and the pause menu. Routes compose it with their
// own readout (the village's objective and clock, a lab's telemetry) and
// pause menu items (the scenario, replays, a lab's switches).
import type { PreparedSession } from "@web/battle/prepare/client";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { RejectedOrder } from "@web/battle/present/rejectedOrder";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { CameraPresentation } from "@packages/renderer-core/src/cameraController";
import { ReadoutLayer } from "@web/battle/present/readouts";
import { SkirmishStatus } from "./battleStatus";
import { PurchasePicker } from "@web/battle/present/purchasePicker";
import { ArmyDeck } from "@web/battle/present/armyDeck";
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
import {
  RangeRulerLabels,
  type RangeRulerLabelsHandle,
} from "@web/battle/present/rangeRulerLabels";
import { SpawnMarker, type SpawnMarkerHandle } from "@web/battle/present/spawnMarker";

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
  prepared,
  replay,
  scripted,
  camera,
  cameraConfig,
  status,
  menu,
  diagnostics,
  cover,
  onLoadStage,
  spawn,
}: {
  fixture: string;
  /** The scenario JSON the authority runs; the view draws its map. */
  scenario: string;
  seed: number;
  prepared?: PreparedSession;
  /** A recorded battle to replay: input is off. */
  replay?: string;
  /** A scripted run: a comparison commander or scenario orders play it, with input off. The benchmark's
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
  /** The authoritative blue entry, used for the deployment cue. */
  spawn?: [number, number] | null;
}) {
  const parsed = useMemo(() => {
    const s = JSON.parse(scenario) as {
      map: { size: [number, number] };
      rules: { service: { radius_m: number } };
      /** Absent or null when the scenario has no completion rule. */
      encounter?: { success_zone_center: [number, number]; success_zone_radius_m: number } | null;
    };
    const drawn: BattleOverlayScenario = {
      supplyRadius: s.rules.service.radius_m,
      zone: s.encounter
        ? { center: s.encounter.success_zone_center, radius: s.encounter.success_zone_radius_m }
        : null,
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
  const [menuOpen, setMenuOpen] = useState(false);
  const [viewportReady, setViewportReady] = useState(false);
  const session = useBattleSession({
    scenario,
    seed,
    prepared,
    onDecoded: noteCues,
    replay,
    scripted,
    destroyable: "apart",
    sound: true,
    inputEnabled: !menuOpen && viewportReady,
  });
  const input = !replay && !scripted;
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  const { contacts } = session;
  const { pointerPaint } = session;
  const rulerLabels = useRef<RangeRulerLabelsHandle>(null);
  const objectiveMarkers = useRef<ObjectiveMarkersHandle>(null);
  const spawnMarker = useRef<SpawnMarkerHandle>(null);
  const { clear: clearCues } = cues;
  const pause = usePauseMenu(sim.client, menuOpen, setMenuOpen);
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
            session.units,
            observation,
            control.selected,
            surfaceZ,
            parsed.drawn,
            { showOrders: control.showOrders, reveal: session.revealed, contacts },
            border,
            metresPerPx,
          )
        : undefined,
    [
      session.units,
      world,
      observation,
      contacts,
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

  if (!meshes && !sim.error) return cover ?? null;
  return (
    <>
      {/* A battle that failed to start draws nothing under its refusal. */}
      {meshes && !sim.error && (
        <LabViewport
          fixture={fixture}
          inputEnabled={!pause.open && loadStage === "playable"}
          world={worldFeed}
          structures={session.structures}
          buildings={session.buildingsFeed}
          obstacles={session.cameraObstaclesFeed}
          overlay={overlayFeed}
          pointerMarks={pointerPaint.feed}
          fog={session.fogFeed}
          frame={session.frame}
          appearances={session.appearances}
          initialCamera={camera}
          cameraConfig={cameraConfig}
          groundAt={surfaceZ}
          onPick={input ? session.onPick : undefined}
          leftDragAction={input && !!session.purchase?.placing}
          onRightPress={session.onRightPress}
          onCursor={(pointer, view, project) => {
            const action = session.onCursor(pointer, view);
            rulerLabels.current?.place(project, pointerPaint.shown);
            return action;
          }}
          onBox={scripted ? undefined : session.onBox}
          onReady={(gpu) => {
            session.onReady(gpu);
            setViewportReady(true);
          }}
          pilot={scripted?.pilot}
          onFrame={(project, view, pointer) => {
            session.hear(view);
            const step = zoomStep(view.distance);
            if (step !== zoomRef.current) {
              zoomRef.current = step;
              setZoom(step);
            }
            session.placePanels(project, view, pointer);
            objectiveMarkers.current?.place(project, surfaceZ);
            spawnMarker.current?.place(project, surfaceZ);
          }}
          diagnostics={{
            ...session.probes,
            audio: () => session.audio?.stats() ?? null,
            ...diagnostics?.(session),
          }}
        />
      )}
      <ObjectiveMarkers
        objectives={observation?.skirmish?.objectives ?? []}
        handle={objectiveMarkers}
      />
      <SpawnMarker at={spawn ?? null} handle={spawnMarker} />
      <ReadoutLayer
        own={observation?.own ?? []}
        identified={observation?.identified}
        contacts={contacts}
        tick={observation?.tick}
        rules={session.rules}
        selected={control.selected}
        handle={session.readouts}
      />
      <RangeRulerLabels handle={rulerLabels} />
      {/* The army roster and command row keep their place as selection changes. */}
      <div className="hud" data-testid="battle-panel" inert={pause.open}>
        <header className="hud-panel hud-top" data-occludes-readouts>
          {!sim.error &&
            (observation?.skirmish ? (
              <SkirmishStatus match={observation.skirmish} />
            ) : (
              status(session)
            ))}
          {sim.error && (
            <div className="hud-error" data-testid="error">
              {sim.error}
            </div>
          )}
        </header>
        <MenuButton onOpen={() => pause.show(true)} />
        <ArmyDeck
          own={observation?.own ?? []}
          selected={control.selected}
          onSelect={control.setSelected}
          rules={session.rules}
          control={input ? control : undefined}
          captions={<CaptionList captions={cues} />}
          reinforcements={
            input && session.purchase && observation?.skirmish ? (
              <>
                <PurchasePicker
                  cards={session.purchase.cards}
                  faction={session.purchase.faction}
                  match={observation.skirmish}
                  onChoose={session.purchase.choose}
                  onReady={session.purchase.ready}
                  onCancelPending={session.purchase.cancelPending}
                />
                {session.purchase.placing && (
                  <button
                    type="button"
                    className="hud-menu-choice hud-placement-cancel"
                    onClick={session.purchase.cancel}
                  >
                    Cancel deployment · Esc
                  </button>
                )}
              </>
            ) : undefined
          }
        />
      </div>
      {input && <RejectedOrder acks={control.acks} />}
      {!sim.error && loadStage !== "playable" && cover}
      {pause.open && (
        <PauseMenu
          onClose={() => pause.show(false)}
          onRestart={
            scripted?.pilot
              ? undefined
              : () => {
                  pause.show(false);
                  sim.restart();
                }
          }
        >
          {menu?.(session)}
        </PauseMenu>
      )}
    </>
  );
}
