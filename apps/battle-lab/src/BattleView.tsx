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
import type { BattleConnectionSession } from "@web/battle/prepare/client";
import type { ObservationView, OwnUnitView } from "@web/battle/sim/observationView";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useNavigate } from "react-router";
import { contactFocusTargets } from "@web/battle/input/contactFocus";
import { RejectedOrder } from "@web/battle/present/rejectedOrder";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { CameraPresentation } from "@packages/renderer-core/src/cameraController";
import { ReadoutLayer } from "@web/battle/present/readouts";
import { battleReadouts } from "./BattleReadouts";
import { SkirmishStatus } from "./battleStatus";
import { MatchResultOverlay } from "./MatchResultOverlay";
import { PurchasePicker, type PurchaseLookup } from "@web/battle/present/purchasePicker";
import { ArmyDeck } from "@web/battle/present/armyDeck";
import { ContactLines, contactLines } from "@web/battle/present/contactLines";
import { useBattleOverlay } from "./useBattleOverlay";
import { LabViewport } from "./LabViewport";
import { useUnitFocus } from "./useUnitFocus";
import { FireBlinks } from "@web/battle/present/fireBlink";
import { MenuButton, PauseMenu, usePauseMenu } from "./PauseMenu";
import { useBattleSession, type BattleSession } from "./useBattleSession";
import { commandedEconomy } from "./commandingPlayer";
import { useReplayCamera } from "./useReplayCamera";
import type { ReplayCameraTrack } from "./replayCamera";
import { ReplayCameraButton } from "./replayFile";
import type { ScriptedSim } from "./useSimSession";
import type { ViewportPilot } from "./viewportRender";
import { useFeed } from "@web/feed";
import { useSyncExternalStore } from "react";
import {
  graphicsProfiles,
  graphicsSettings,
  resolveEffectiveTier,
  testQualityProfile,
} from "./graphicsSettings";
import {
  RangeRulerLabels,
  type RangeRulerLabelsHandle,
} from "@web/battle/present/rangeRulerLabels";

/** What has finished loading: the static world's meshes are built
 *  (`world`), the viewport has drawn its first frame (`renderer`), and the
 *  battle's first observation has arrived (`playable`). */
export type BattleLoadStage = "world" | "renderer" | "playable";

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
  replayCamera,
  onSaveReplay,
  scripted,
  camera,
  cameraConfig,
  status,
  menu,
  coach,
  diagnostics,
  cover,
  onLoadStage,
  onFailure,
  onLeave,
  onDecoded,
  callsigns,
}: {
  fixture: string;
  /** The scenario JSON the authority runs; the view draws its map. */
  scenario: string;
  seed: number;
  prepared?: BattleConnectionSession;
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
  replayCamera?: ReplayCameraTrack;
  onSaveReplay?: (
    session: BattleSession,
    camera: ReplayCameraTrack | undefined,
  ) => Promise<unknown>;
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
  /** An authored scenario's coach, over the battle while it is played. */
  coach?: (session: BattleSession) => ReactNode;
  /** Route-specific lab probes, merged into the shared ones. */
  diagnostics?: (session: BattleSession) => Record<string, unknown>;
  /** A loading screen shown over the view until the battle is playable. */
  cover?: ReactNode;
  /** Each stage of loading, once, as it completes. */
  onLoadStage?: (stage: BattleLoadStage) => void;
  /** The battle's authority refused or failed it: the route's cover shows the
   *  refusal, and nothing is drawn under it. Without one, the HUD says why. */
  onFailure?: (error: string) => void;
  /** Route-owned departure, from the pause menu (through its confirmation)
   *  and from the finished match's report: back to Multiplayer. Without one
   *  both leave for the main menu. */
  onLeave?: () => void;
  /** An online battle's public player names, for its report. */
  callsigns?: Record<string, string>;
  /** Scenario/runtime audio may react to the first authoritative publication. */
  onDecoded?: (observation: ObservationView, digest: string) => void;
}) {
  const [menuOpen, setMenuOpen] = useState(false);
  const navigate = useNavigate();
  const [viewportReady, setViewportReady] = useState(false);
  const [purchaseLookup, setPurchaseLookup] = useState<PurchaseLookup | null>(null);
  const purchaseLookupSerial = useRef(0);
  const { cameraFocus, focusUnit } = useUnitFocus();
  // Every publication, not every render, so a pass published for one tick
  // still blinks its card.
  const blinksRef = useRef<FireBlinks | null>(null);
  const noteDecoded = useCallback(
    (observation: ObservationView, digest: string) => {
      blinksRef.current?.observe(observation);
      onDecoded?.(observation, digest);
    },
    [onDecoded],
  );
  const session = useBattleSession({
    scenario,
    seed,
    prepared,
    scenarioArt,
    player,
    side,
    onDecoded: noteDecoded,
    replay,
    scripted,
    sound: true,
    inputEnabled: !menuOpen && viewportReady && inCommand,
    onCenter: focusUnit,
  });
  const input = !replay && !scripted;
  const { meshes, sim, control, surfaceZ } = session;
  const cameraReplay = useReplayCamera(
    session,
    camera,
    replayCamera,
    input && !!onSaveReplay,
    inCommand,
  );
  const exportReplay =
    sim.client && onSaveReplay ? () => onSaveReplay(session, cameraReplay.track) : undefined;
  const blinks = useMemo(() => new FireBlinks(session.rules.tick_hz), [session.rules.tick_hz]);
  blinksRef.current = blinks;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  const matchTarget = useMemo(() => {
    const setup = JSON.parse(scenario).skirmish as { rules: { victory_score: number } } | undefined;
    return setup?.rules.victory_score;
  }, [scenario]);
  const matchFinished = observation?.skirmish?.phase === "finished";
  const viewingPlayer = commandedEconomy(observation?.skirmish, session.viewer?.id);
  const { contacts } = session;
  // Contacts N steps through, newest first.
  const focusTargets = useMemo(() => contactFocusTargets(contacts), [contacts]);
  const lines = useMemo(
    () => contactLines(session.units, contacts, observation?.tick ?? 0, session.rules.tick_hz),
    [session.units, contacts, observation?.tick, session.rules.tick_hz],
  );
  const { pointerPaint } = session;
  const rulerLabels = useRef<RangeRulerLabelsHandle>(null);
  const objectiveMarkers = useRef<ObjectiveMarkersHandle>(null);
  const pause = usePauseMenu(sim.local, menuOpen, setMenuOpen);
  const graphics = useSyncExternalStore(graphicsSettings.subscribe, graphicsSettings.get);
  const quality =
    testQualityProfile(window.location.search) ??
    graphicsProfiles[resolveEffectiveTier(graphics.mode, graphics.detectedMode)];
  const { audio } = session;
  useEffect(() => {
    audio?.reset();
  }, [sim.client, audio]);

  const overlay = useBattleOverlay(session, { scenario, camera });

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
  // A card's right-click asks the store for that exact card. A serial lets
  // the same unit be asked for again after the store closed.
  const lookupPurchase = useCallback((unit: OwnUnitView) => {
    purchaseLookupSerial.current += 1;
    setPurchaseLookup({ card: unit.kind, serial: purchaseLookupSerial.current });
  }, []);
  const clearPurchaseLookup = useCallback(() => setPurchaseLookup(null), []);

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
          overlay={overlay.overlayFeed}
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
          pilot={scripted?.pilot ?? cameraReplay.pilot}
          onFrame={(project, view, pointer, scale, time) => {
            cameraReplay.record(time, view);
            session.hear();
            overlay.follow(view, scale);
            session.placePanels(project, view, pointer);
            objectiveMarkers.current?.place(project, surfaceZ);
          }}
          diagnostics={{ ...session.probes, ...diagnostics?.(session), exportReplay }}
          quality={quality}
        />
      )}
      <ObjectiveMarkers
        objectives={observation?.skirmish?.objectives ?? []}
        viewer={session.viewer?.side ?? null}
        handle={objectiveMarkers}
      />
      <ReadoutLayer {...battleReadouts(session)} />
      <RangeRulerLabels handle={rulerLabels} />
      {/* The army roster and command row keep their place as selection changes. */}
      <div className="hud" data-testid="battle-panel" inert={pause.open || matchFinished}>
        <header className="hud-panel hud-top" data-occludes-readouts>
          {!sim.error &&
            (observation?.skirmish ? (
              <SkirmishStatus
                target={matchTarget!}
                match={observation.skirmish}
                viewer={session.viewer?.side ?? null}
              />
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
        {input && coach?.(session)}
        {cameraReplay.pilot && (
          <ReplayCameraButton
            following={cameraReplay.following}
            onToggle={() => cameraReplay.setFollowing(!cameraReplay.following)}
          />
        )}
        <ArmyDeck
          state={armyState}
          selected={control.selected}
          onSelect={control.setSelected}
          onCenter={input ? focusUnit : undefined}
          onLookup={battleInput && session.purchase ? lookupPurchase : undefined}
          groups={input ? control.groups : undefined}
          blinks={blinks.feed}
          onDeleteGroup={control.deleteGroup}
          onHover={session.hoverFromDeck}
          rules={session.rules}
          pending={viewingPlayer?.pending}
          tick={observation?.tick}
          contactLines={<ContactLines lines={lines} />}
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
                  lookup={purchaseLookup}
                  onLookupHandled={clearPurchaseLookup}
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
          onLeave={onLeave}
          online={!!prepared && "transport" in prepared}
          onClose={() => pause.show(false)}
          onRestart={
            !sim.local || scripted?.pilot
              ? undefined
              : () => {
                  pause.show(false);
                  sim.restart();
                }
          }
          onSettingsApply={() => window.__lab?.rebuild?.()}
          onSaveReplay={exportReplay}
        >
          {menu?.(session)}
        </PauseMenu>
      )}
      {observation?.skirmish && (
        <MatchResultOverlay
          match={observation.skirmish}
          seats={session.seats}
          viewer={session.viewer?.id ?? null}
          callsigns={callsigns}
          leave={
            onLeave
              ? { label: "Return to Multiplayer", onLeave }
              : { label: "Main menu", onLeave: () => navigate("/") }
          }
        />
      )}
    </SessionCatalogProvider>
  );
}
