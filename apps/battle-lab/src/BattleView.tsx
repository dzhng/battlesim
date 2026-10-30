import { SessionCatalogProvider } from "@web/battle/catalog/context";
import {
  ObjectiveMarkers,
  type ObjectiveMarkersHandle,
} from "@web/battle/present/objectiveMarkers";
// A played (or replayed) battle for the selected side: the world, that side's units and
// every overlay, and the HUD: the top bar's readout, the army deck and its
// command bar, subtitles, and the pause menu. Routes compose it with their
// own readout (a battle's clock, a lab's telemetry) and
// pause menu items (the scenario, replays, a lab's switches).
import type { PreparedConnectionSession } from "@web/battle/prepare/client";
import type { ObservationView } from "@web/battle/sim/observationView";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { contactFocusTargets } from "@web/battle/input/contactFocus";
import { RejectedOrder } from "@web/battle/present/rejectedOrder";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { CameraPresentation } from "@packages/renderer-core/src/cameraController";
import { ReadoutLayer } from "@web/battle/present/readouts";
import { SkirmishStatus } from "./battleStatus";
import { MatchResultOverlay } from "./MatchResultOverlay";
import { PurchasePicker } from "@web/battle/present/purchasePicker";
import { ArmyDeck } from "@web/battle/present/armyDeck";
import { CaptionList, useCaptions } from "@web/battle/present/captions";
import { battleOverlayScenario, buildBattleOverlay } from "./battleOverlay";
import { useContactFacing } from "./contactFacing";
import { borderWidthM, buildMapBorder } from "@packages/battle-renderer/src/playAreaOverlay";
import { metresPerPxAt } from "@packages/renderer-core/src/camera3d";
import { gameMapBorder } from "./gameFog";
import { gameStroke } from "./gameOverlay";
import { LabViewport } from "./LabViewport";
import { useUnitFocus } from "./useUnitFocus";
import { MenuButton, PauseMenu, usePauseMenu } from "./PauseMenu";
import { useBattleSession, type BattleSession } from "./useBattleSession";
import { commandedEconomy } from "./commandingPlayer";
import type { ScriptedSim } from "./useSimSession";
import type { ViewportPilot } from "./LabViewport";
import { useFeed } from "./feed";
import { useSyncExternalStore } from "react";
import {
  graphicsProfiles,
  graphicsSettings,
  resolveEffectiveTier,
  sceneMinimumProfile,
} from "./graphicsSettings";
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

// eslint-disable-next-line complexity -- debt: most branches are `observation?.` and `input &&` guards on HUD props; moving the `.hud` block into a component that takes a non-null observation would split it, but proving nothing on screen moved needs the HUD screenshot gates.
export function BattleView({
  fixture,
  scenario,
  seed,
  prepared,
  scenarioArt,
  player,
  inCommand = true,
  side,
  replay,
  scripted,
  camera,
  cameraConfig,
  status,
  menu,
  diagnostics,
  cover,
  onLoadStage,
  onFailure,
  onDecoded,
}: {
  fixture: string;
  /** The scenario JSON the authority runs; the view draws its map. */
  scenario: string;
  seed: number;
  prepared?: PreparedConnectionSession;
  /** Uploaded resolved sandbox bundles can narrow art to their fielded map. */
  scenarioArt?: boolean;
  /** Player issuing commands in a player-owned scenario; null when the
   *  session seats no human, and input is off. */
  player?: string | null;
  /** False once AI takes the player over: input stops while the same
   *  battle goes on (changing `player` would start a new one). */
  inCommand?: boolean;
  /** Side whose observation and command stream this player controls. */
  side?: import("@web/battle/sim/protocol").SideName;
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
  /** The battle's authority refused or failed it: the route's cover shows the
   *  refusal, and nothing is drawn under it. Without one, the HUD says why. */
  onFailure?: (error: string) => void;
  /** Scenario/runtime audio may react to the first authoritative publication. */
  onDecoded?: (observation: ObservationView, digest: string) => void;
}) {
  const parsed = useMemo(
    () => ({
      size: (JSON.parse(scenario) as { map: { size: [number, number] } }).map.size,
      drawn: battleOverlayScenario(scenario, side ?? "blue"),
      tickHz: (JSON.parse(scenario) as { rules: { tick_hz: number } }).rules.tick_hz,
    }),
    [scenario, side],
  );
  // The border and the orders are rebuilt only when the zoom crosses a step
  // (×1.25), so their widths hold near their `_px` widths on screen.
  const [zoom, setZoom] = useState(() => zoomStep(camera.distance));
  const zoomRef = useRef(zoom);
  const metresPerPx = metresPerPxAt(ZOOM_BASE ** zoom, camera.fovY, window.innerHeight);
  const [renderScale, setRenderScale] = useState(1);
  const contactFacing = useContactFacing(camera, metresPerPx, renderScale);
  const [menuOpen, setMenuOpen] = useState(false);
  const [viewportReady, setViewportReady] = useState(false);
  const session = useBattleSession({
    scenario,
    seed,
    prepared,
    scenarioArt,
    player,
    side,
    onDecoded: (observation, digest) => {
      noteCues(observation);
      onDecoded?.(observation, digest);
    },
    replay,
    scripted,
    sound: true,
    inputEnabled: !menuOpen && viewportReady && inCommand,
  });
  const cues = useCaptions(session.units, session.rules.tick_hz);
  const { note: noteCues } = cues;
  const input = !replay && !scripted;
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  const matchFinished = observation?.skirmish?.phase === "finished";
  const viewingPlayer = commandedEconomy(observation?.skirmish, session.viewer?.id);
  const { contacts } = session;
  // Contacts N steps through, newest first.
  const focusTargets = useMemo(() => contactFocusTargets(contacts), [contacts]);
  const { cameraFocus, focusUnit } = useUnitFocus();
  const { pointerPaint } = session;
  const rulerLabels = useRef<RangeRulerLabelsHandle>(null);
  const objectiveMarkers = useRef<ObjectiveMarkersHandle>(null);
  const { clear: clearCues } = cues;
  const pause = usePauseMenu(sim.client, menuOpen, setMenuOpen);
  const graphics = useSyncExternalStore(graphicsSettings.subscribe, graphicsSettings.get);
  const requestedQuality = import.meta.env.DEV
    ? new URLSearchParams(window.location.search).get("test-quality")
    : null;
  const quality =
    requestedQuality === "minimum"
      ? sceneMinimumProfile
      : requestedQuality && requestedQuality in graphicsProfiles
        ? graphicsProfiles[requestedQuality as keyof typeof graphicsProfiles]
        : graphicsProfiles[resolveEffectiveTier(graphics.mode, graphics.detectedMode)];
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
            borderWidthM(gameMapBorder, gameStroke(metresPerPx, renderScale)),
            surfaceZ,
          )
        : null,
    [world, parsed.size, metresPerPx, renderScale, surfaceZ],
  );
  const overlay = useMemo(
    () =>
      world && observation
        ? buildBattleOverlay(
            session.units,
            session.rules,
            observation,
            control.selected,
            surfaceZ,
            parsed.drawn,
            {
              showOrders: control.showOrders,
              reveal: session.revealed,
              contacts,
              facing: contactFacing.facing,
              allies: session.allies,
            },
            border,
            gameStroke(metresPerPx, renderScale),
          )
        : undefined,
    [
      session.allies,
      contactFacing.facing,
      session.units,
      session.rules,
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
      renderScale,
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
  const onFailureRef = useRef(onFailure);
  onFailureRef.current = onFailure;
  useEffect(() => {
    if (sim.error) onFailureRef.current?.(sim.error);
  }, [sim.error]);
  // The battle accepts keys only while it is the thing on screen.
  const battleInput = !pause.open && !matchFinished && loadStage === "playable";

  const armyState = useFeed({ own: control.controlled, control: input ? control : undefined });

  if (sim.error && onFailure) return cover ?? null;
  if (!meshes && !sim.error) return cover ?? null;
  return (
    <SessionCatalogProvider catalog={session.catalog}>
      {/* A battle that failed to start draws nothing under its refusal. */}
      {meshes && !sim.error && (
        <LabViewport
          fixture={fixture}
          inputEnabled={battleInput}
          world={worldFeed}
          {...session.known}
          overlay={overlayFeed}
          pointerMarks={pointerPaint.feed}
          fog={session.fogFeed}
          {...session.viewport}
          initialCamera={camera}
          focusTargets={focusTargets}
          cameraFocus={cameraFocus}
          cameraConfig={cameraConfig}
          groundAt={surfaceZ}
          onPick={input ? session.input.onPick : undefined}
          leftDragAction={input && !!session.purchase?.placing}
          onRightPress={session.input.onRightPress}
          onCursor={(pointer, view, project, scale) => {
            const action = session.input.onCursor(pointer, view, project, scale);
            rulerLabels.current?.place(project, pointerPaint.shown);
            return action;
          }}
          onBox={scripted ? undefined : session.input.onBox}
          onReady={(gpu) => {
            session.viewport.onReady(gpu);
            setViewportReady(true);
          }}
          pilot={scripted?.pilot}
          onFrame={(project, view, pointer, scale) => {
            setRenderScale(scale);
            session.hear();
            contactFacing.follow(view);
            const step = zoomStep(view.distance);
            if (step !== zoomRef.current) {
              zoomRef.current = step;
              setZoom(step);
            }
            session.placePanels(project, view, pointer);
            objectiveMarkers.current?.place(project, surfaceZ);
          }}
          diagnostics={{ ...session.probes, ...diagnostics?.(session) }}
          quality={quality}
        />
      )}
      <ObjectiveMarkers
        objectives={observation?.skirmish?.objectives ?? []}
        handle={objectiveMarkers}
      />
      <ReadoutLayer
        own={observation?.own ?? []}
        identified={observation?.identified}
        contacts={contacts}
        tick={observation?.tick}
        rules={session.rules}
        selected={control.selected}
        handle={session.readouts}
        ghosts={session.ghostReadouts}
        allies={session.allies}
      />
      <RangeRulerLabels handle={rulerLabels} />
      {/* The army roster and command row keep their place as selection changes. */}
      <div className="hud" data-testid="battle-panel" inert={pause.open || matchFinished}>
        <header className="hud-panel hud-top" data-occludes-readouts>
          {!sim.error &&
            (observation?.skirmish ? (
              <SkirmishStatus match={observation.skirmish} viewer={session.viewer?.side ?? null} />
            ) : (
              status(session)
            ))}
          {sim.error && (
            <div className="hud-error" data-testid="error">
              {sim.error}
            </div>
          )}
        </header>
        {input && observation?.skirmish?.phase === "preparation" && session.purchase && (
          <div className="hud-ready-layer" data-occludes-readouts>
            <button
              type="button"
              className="hud-menu-choice hud-purchase-ready"
              disabled={viewingPlayer?.ready ?? false}
              onClick={session.purchase.ready}
            >
              {viewingPlayer?.ready ? "READY" : "START BATTLE"}
            </button>
          </div>
        )}
        <MenuButton onOpen={() => pause.show(true)} />
        <ArmyDeck
          state={armyState}
          selected={control.selected}
          onSelect={control.setSelected}
          onCenter={input ? focusUnit : undefined}
          rules={session.rules}
          pending={viewingPlayer?.pending}
          tick={observation?.tick}
          captions={<CaptionList captions={cues} />}
          reinforcements={
            input && session.purchase && observation?.skirmish ? (
              <>
                <PurchasePicker
                  cards={session.purchase.cards}
                  faction={session.purchase.faction}
                  match={observation.skirmish}
                  player={viewingPlayer}
                  onChoose={session.purchase.choose}
                  rules={session.rules}
                  enabled={battleInput}
                />
              </>
            ) : undefined
          }
        />
      </div>
      {input && <RejectedOrder acks={control.acks} {...session.refusals} />}
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
          onSettingsApply={() => window.__lab?.rebuild?.()}
        >
          {menu?.(session)}
        </PauseMenu>
      )}
      {observation?.skirmish && (
        <MatchResultOverlay
          match={observation.skirmish}
          seats={session.seats}
          viewer={session.viewer?.id ?? null}
        />
      )}
    </SessionCatalogProvider>
  );
}
