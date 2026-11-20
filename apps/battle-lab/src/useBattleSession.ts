import { knownCraters } from "./groundCells";
import { useViewportGpu } from "./useViewportGpu";
import * as gameWasm from "@wasm/game_wasm";
import type { ViewportFrame } from "./presentedFrame";
// One side's session owns the authority, commands and canonical presentation.
// Picking and readouts observe its drawn state; routes add their own probes.
import { admitScenario, composeScenarioCatalog } from "@web/battle/catalog/sets";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { ContactPresentation } from "@web/battle/present/contactPresentation";
import { gameContactStyle } from "./gameFog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReadoutLayerHandle } from "@web/battle/present/readouts";
import { metresPerPxAt } from "@packages/renderer-core/src/camera3d";
import { pickBox } from "@packages/battle-renderer/src/picking";
import { useWorldLayers } from "./useWorldLayers";
import { useBattleArt } from "./useBattleArt";
import { useBattleWarmup } from "./useBattleWarmup";
import { gameOrderFlash } from "./gameOverlay";
import {
  NOTHING_REVEALED,
  OrderReveal,
  sameReveal,
  type RevealedOrders,
} from "@web/battle/present/orderReveal";
import { highAir } from "@packages/scene-assets/src/units";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import { placementPose, usePurchasePlacement } from "./usePurchasePlacement";
import {
  isArmed,
  facedIntent,
  reconcilePointerIntent,
  type PointerIntent,
  type PointerPick,
} from "@web/battle/input/pointerIntent";
import {
  PointerPaint,
  attackGroundMarks,
  rulerAt,
  previewForIntent,
  pointerCursor,
  garrisonTarget,
  intentForOrder,
} from "./pointerPaint";
import { OrderPreview } from "./orderPreview";
import { usePreviewRevision } from "./previewRevision";
import { useKnownBuildings } from "./knownBuildings";
import { ReleasedAction } from "./releasedAction";
import type { ObservationView } from "@web/battle/sim/observationView";
import type { Order, SideName } from "@web/battle/sim/protocol";
import type { Player } from "@web/battle/prepare/protocol";
import type { LabBox, LabPick, ViewportCursor } from "./LabViewport";
import { drawnUnits, pickToPointer, sideInstances, type DrawnInstances } from "./sideInstances";
import { surfaceHeights } from "./surfaceHeights";
import { usePanels } from "./panelPlacement";
import { commandedEconomy, commandingPlayer, takesOrders } from "./commandingPlayer";
import { allyOwners, controlled, playerIndex } from "@web/battle/input/controlled";
import { useSimSession } from "./useSimSession";
import type { BattleSessionOptions, ScenarioRules } from "./battleSessionOptions";
import { groundUnderRay, useMapBuildings, useStaticWorld } from "./useStaticWorld";
import { createBattleAudio } from "./soundFeed";
import type { UnitResponses } from "./unitResponses";
import { useUnitVoices } from "./useUnitVoices";
import { frameCounts, presentedCamera } from "./presentedFrame";
import type { FogRequest, ScenePresentation } from "@web/battle/sim/presentation";
import type { ViewportCamera } from "@packages/renderer-core/src/viewportCamera";
import { useAppAudio } from "./AppAudio";
import { useFeed } from "@web/feed";
import type { Vec3 } from "math";
import {
  GHOST_OWN,
  GHOST_SELECTED,
  ghostBodies,
  ghostAloft,
  aircraftOrderReadouts,
  orderedGhost,
  ownSoldiers,
  placementUnder,
  type UnitGhost,
} from "./unitGhosts";
import { boughtGhosts, byReference, heldGhost, pendingGhosts } from "./shopGhosts";
import { presentationProbes } from "./presentationProbes";

export function useBattleSession({
  scenario,
  seed,
  onDecoded,
  replay,
  scripted,
  side = "blue",
  player,
  sound = false,
  inputEnabled = true,
  onCenter,
  xray = true,
  prepared,
  scenarioArt = false,
}: BattleSessionOptions) {
  const appAudio = useAppAudio();
  const sourceCatalog = useSessionCatalog();
  const { map, rules, rulesText, skirmish, units, fielded, players, catalog } = useMemo(() => {
    const parsed = JSON.parse(scenario) as {
      map: unknown;
      rules: ScenarioRules;
      players?: Player[];
      skirmish?: object;
      units?: { kind: string }[];
    };
    const rules = JSON.parse(gameWasm.resolve_rules(JSON.stringify(parsed.rules)));
    const catalog = { ...composeScenarioCatalog(rules, gameWasm), set: sourceCatalog.set };
    return {
      ...parsed,
      catalog,
      rules: rules as ScenarioRules,
      rulesText: JSON.stringify(rules),
      units: admitScenario(parsed, catalog),
      fielded: (parsed.units ?? []).map((u) => u.kind),
      players: parsed.players ?? [],
    };
  }, [scenario, sourceCatalog.set]);
  const world = useStaticWorld(map, rules);
  const audio = useMemo(
    () => (sound ? createBattleAudio(appAudio, seed, catalog) : null),
    [sound, catalog, appAudio, seed],
  );
  useEffect(() => () => audio?.dispose(), [audio]);
  // The units' answers to their owner (R1–R12), one battle's at a time.
  const responses = useRef<UnitResponses | null>(null);
  // The presentation notes every publication for the effects as it decodes
  // it (`presentation.ts`); sound hears the same. The viewport reads it each
  // frame through this one holder for the session's life: a holder that
  // changed with its client (a Restart) would have React's development build
  // log the old battle's tables into a measure detail (`feed.ts`).
  const presentationRef = useRef<ScenePresentation | null>(null);
  const admittedPresentationRef = useRef<ScenePresentation | null>(null);
  const noteDecoded = useCallback(
    (o: ObservationView, digest: string) => {
      const presentation = presentationRef.current;
      presentation?.noteAudio(audio);
      // An enemy kind's voice set loads once the side first identifies one.
      audio?.identified(o.identified.map((unit) => unit.kind));
      responses.current?.observed(o);
      onDecoded?.(o, digest);
    },
    [audio, onDecoded],
  );
  // The player this client's orders go out as, and whose units it orders.
  const commanding = commandingPlayer(players, side, player);
  /** The human the client plays as; null when it watches a battle it has no
   *  seat in (AI against AI), and the result reads neutral. */
  const viewer = players.find((p) => p.id === commanding) ?? null;
  const sim = useSimSession({
    scenario,
    seed,
    onDecoded: noteDecoded,
    replay,
    scripted,
    prepared,
    player: commanding,
    side,
  });
  presentationRef.current = sim.client?.presentation ?? null;
  const sideFaction = (which: SideName) =>
    players.find((player) => player.side === which)?.faction ?? null;
  const faction = sideFaction(side);
  const {
    placement: purchasePlacement,
    placing: purchasing,
    ghost: purchaseGhost,
    placed: placedGhosts,
    bought: boughtPurchases,
    choose: chooseVariant,
    click: clickPurchase,
  } = usePurchasePlacement(sim.client);
  const { observation } = sim;
  const sceneryForArt = useMemo(() => {
    if (!scenarioArt) return undefined;
    const candidate = map as { props?: unknown[]; forests?: unknown[] } | null;
    return (candidate?.props?.length ?? 0) === 0 && (candidate?.forests?.length ?? 0) === 0
      ? null
      : undefined;
  }, [map, scenarioArt]);
  // The requested kinds still scope the simulation's descriptions and voices;
  // map art is scoped to this session, independently of the page's asset cache.
  const enemy = skirmish ? sideFaction(side === "blue" ? "red" : "blue") : null;
  useUnitVoices(responses, audio, sim.client, units, faction, enemy, rules.tick_hz);
  const contactPresentation = useMemo(
    () => (sim.client ? new ContactPresentation(rules.tick_hz, gameContactStyle.fade_s) : null),
    [sim.client, rules.tick_hz],
  );
  const contacts = useMemo(
    () => (observation && contactPresentation?.update(observation)) || [],
    [contactPresentation, observation],
  );
  // The last drawn frame's presentation clock: the callouts' nudges ease on
  // it, and an order's flash starts at it.
  const drawnClock = useRef<number | null>(null);
  // The publication the last drawn frame drew (`FrameSample.observation`).
  // Whatever reads the drawn frame (its picks, its drawn positions) reads the
  // battle from this, not from React's `observation`, which lags it by a render.
  const drawnObservation = useRef<ObservationView | null>(null);
  const orderReveal = useMemo(() => new OrderReveal(gameOrderFlash), []);
  const [pointerPaint] = useState(() => new PointerPaint(units));
  const [orderPreview] = useState(() => new OrderPreview());
  const [releasedAction] = useState(() => new ReleasedAction());
  const shownIntent = useRef<PointerIntent>({ kind: "none" });
  const dragGhosts = useRef<UnitGhost[]>([]);
  const [revealed, setRevealed] = useState<RevealedOrders>(NOTHING_REVEALED);
  const revealedRef = useRef(revealed);
  const noteOrder = useCallback(
    (order: Order, queued: boolean) => {
      releasedAction.issue(order, queued, sim.client, intentForOrder(order, queued));
      if (drawnClock.current !== null) orderReveal.noteOrder(order, drawnClock.current);
    },
    [orderReveal, releasedAction, sim.client],
  );
  const control = useUnitControl(
    replay || scripted ? null : sim.client,
    observation,
    {
      onIssue: noteOrder,
      onAck: (order, ack, queued) => {
        releasedAction.acknowledge(order, ack, queued, sim.client);
        responses.current?.acknowledged(order, ack, queued);
      },
      onSelect: (added, clicked) => responses.current?.selected(added, clicked),
      onCenter,
    },
    inputEnabled && takesOrders(players, commanding),
    commanding,
  );
  /** The side's units an ally owns, with their panels' owner lines. */
  const allies = useMemo(() => allyOwners(observation, commanding), [observation, commanding]);
  const choosePurchase = useCallback(
    (variant: string, carrier?: string) => {
      control.setSelected([]);
      control.setMode("move");
      chooseVariant(variant, carrier);
    },
    [control, chooseVariant],
  );
  // A new battle carries no flash over.
  useEffect(() => {
    orderReveal.clear();
    releasedAction.reset();
    orderPreview.update(null, null, 0);
  }, [sim.client, orderReveal, orderPreview, releasedAction]);

  // Every building is drawn from its template's rows, as instances of kit
  // modules, standing or fallen: no fitted model stands for one or for its
  // remains.
  const mapBuildings = useMapBuildings(world);
  const placedProps = mapBuildings?.props ?? null;
  const drawnBuildings = mapBuildings?.index ?? null;
  const presentation = sim.client?.presentation ?? null;
  const factionsForArt = useMemo(
    () =>
      skirmish
        ? new Set(players.flatMap((player) => (player.faction ? [player.faction] : [])))
        : null,
    [skirmish, players],
  );
  const admitted = useBattleArt({
    scenario,
    seed,
    side,
    presentation,
    world,
    buildings: mapBuildings,
    units,
    fielded,
    rules,
    rulesText,
    skirmish: skirmish !== undefined,
    players,
    factions: factionsForArt,
    viewer: viewer !== null,
    observation,
    purchasing,
    scenery: sceneryForArt,
  });
  const {
    battle: worldInput,
    appearances,
    models: modelAppearances,
  } = admitted ?? {
    battle: null,
    appearances: null,
    models: null,
  };
  admittedPresentationRef.current = admitted ? presentation : null;
  // The ground, and the art its scenery and grass draw from; its props and
  // what the side knows of them (rubble, wrecks, shoved bodies where it last
  // saw them) are the battle's presentation's to draw.
  const meshes = useWorldLayers(world, appearances, sceneryForArt);
  const knownProps = observation?.knownProps;
  const knownKey = useMemo(() => JSON.stringify(knownProps ?? []), [knownProps]);
  // Fog of war: the presentation sees from the side's eyes at each
  // publication over the ground, cut by what the side knows stands. Its
  // foliage is the side's: less the trees on ground it has seen cleared (a
  // lane knocked, a patch shelled), re-exported when that ground grows.
  const clearedCount = observation ? (sim.ground.current?.clearedCount ?? 0) : 0;
  const clearingEpoch = sim.ground.current?.epoch ?? 0;
  const foliage = useMemo(() => {
    const g = sim.ground.current;
    if (!world || !(clearedCount > 0 && g)) return undefined;
    return world.view.foliage_cleared(g.clearedRuns(), g.cols, g.cellM);
  }, [world, clearedCount, clearingEpoch, sim.ground]);
  const fog = useMemo<FogRequest | null>(
    () => (world ? { sensors: rules.sensors, foliage } : null),
    [world, rules, foliage],
  );

  const fogFeed = useFeed(fog);

  const surfaceZ = useMemo(() => surfaceHeights(world), [world]);
  const aloft = useMemo(() => ghostAloft(units, rules.air), [units, rules.air]);
  const knownBuildings = useKnownBuildings(
    world,
    mapBuildings,
    knownKey,
    units,
    surfaceZ,
    presentation,
  );

  // What the last frame drew: which unit or enemy each pick box is, and each
  // own unit's drawn (interpolated) position.
  const drawn = useRef<DrawnInstances>({ picks: [], owners: [], enemies: [], rings: [] });
  const drawnAt = useRef(new Map<number, Readonly<Vec3>>());
  const drawnEnemyAt = useRef(new Map<number, Readonly<Vec3>>());
  const readouts = useRef<ReadoutLayerHandle>(null);
  // The models layer installs only what the battle draws as models.
  const modelAppearancesFeed = useFeed(modelAppearances);

  // Each unit placed during preparation stands as a ghost where it will go,
  // turned as it was placed, until the battle starts.
  const placedGhostList = useMemo(
    () =>
      byReference(
        (match: ObservationView["skirmish"]): UnitGhost[] =>
          pendingGhosts(
            match?.phase === "preparation"
              ? (commandedEconomy(match, commanding)?.pending ?? [])
              : [],
            placedGhosts.current,
          ),
        [],
      ),
    [commanding, placedGhosts],
  );
  const frame = useCallback(
    (now: number, viewport: ViewportCamera, generation: number): ViewportFrame | null => {
      // React's `observation` only says the session has a battle (it is reset
      // with the battle). Everything this frame draws or decides reads the
      // latest publication, the tick the presentation blends toward.
      const published = sim.latest.current;
      // Staged as soon as it is known: the presentation loads it with the
      // buildings by its next frame, the viewport's until the first publication.
      presentation?.setBattle(worldInput);
      const time = observation && published ? presentation?.clock(now) : null;
      if (time == null || !published || !presentation || !worldInput) {
        return null;
      }
      // The order just issued has not landed: its units still publish the old one.
      const pending = releasedAction.released;
      const ack = pending?.ack;
      const landed = ack && (ack.error || published.tick >= ack.applied_tick);
      const awaiting = new Set(
        pending && !landed && "units" in pending.order ? pending.order.units : [],
      );
      const reveal = orderReveal.at(
        time,
        control.showOrders,
        controlled(published, commanding),
        awaiting,
      );
      if (!sameReveal(reveal, revealedRef.current)) {
        revealedRef.current = reveal;
        setRevealed(reveal);
      }
      // Ghosts: purchases placed (in battle, those just bought, fading),
      // every ordered destination with Space held, a held right-drag's
      // destinations, and the purchase being placed.
      const placement = purchaseGhost.current;
      const ghosts = [...placedGhostList.of(published.skirmish), ...dragGhosts.current];
      ghosts.push(...boughtGhosts(boughtPurchases.current, now, published.own));
      if (control.showOrders)
        for (const unit of published.own) {
          if (!reveal.has(unit.id)) continue;
          const ghost = orderedGhost(unit, GHOST_OWN);
          if (ghost) ghosts.push(ghost);
        }
      if (placement && inputEnabled) ghosts.push(heldGhost(placement));
      const presented = presentation.frame(
        time,
        presentedCamera(viewport),
        {
          selected: control.selected,
          xray,
          player: playerIndex(published, commanding),
          ghosts: ghostBodies(ghosts, (kind) => Boolean(units.hull(kind)), surfaceZ, aloft),
        },
        generation,
      );
      const { own, enemies } = drawnUnits(presented);
      const candidate = sideInstances(presented, own, enemies, published, units);
      const ownAt = new Map([...own].map(([id, u]) => [id, u.position]));
      const enemyAt = new Map([...enemies].map(([id, u]) => [id, u.position]));
      const { models: counts, effects: effectCounts } = frameCounts(presented);
      return {
        submitted: () => {
          drawn.current = candidate;
          drawnAt.current = ownAt;
          drawnEnemyAt.current = enemyAt;
          drawnClock.current = time;
          drawnObservation.current = published;
        },
        picks: candidate.picks,
        presented,
        counts,
        clock: time,
        effectCounts,
      };
    },
    [
      observation,
      sim.latest,
      presentation,
      worldInput,
      units,
      orderReveal,
      control.showOrders,
      releasedAction,
      control.selected,
      xray,
      placedGhostList,
      boughtPurchases,
      purchaseGhost,
      inputEnabled,
      aloft,
      surfaceZ,
      commanding,
    ],
  );
  /** Sound for the last frame, heard from its camera: call once a frame. */
  const hear = useCallback(() => {
    const presentation = sim.client?.presentation;
    presentation?.updateAudio(audio);
  }, [audio, sim.client]);

  const semanticPick = useCallback(
    (pick: LabPick): PointerPick | null => {
      if (!world) return null;
      const pointer = pickToPointer(world, drawn.current, pick, drawnObservation.current?.contacts);
      const panel = readouts.current?.pick(pick.x, pick.y);
      return panel ? { ...pointer, ...panel } : pointer;
    },
    [world, observation],
  );
  const onRightPress = useCallback(
    (pick: LabPick | null) => {
      const pointer = pick && semanticPick(pick);
      if (pointer) releasedAction.capture(pointer, control.intentAt(pointer), sim.client);
      else releasedAction.cancelCapture();
    },
    [semanticPick, control, sim.client, releasedAction],
  );
  const onPick = useCallback(
    (pick: LabPick) => {
      if (purchasePlacement.variant) return clickPurchase(pick, control.issue, sim.latest.current);
      const held = releasedAction.capturedFor(sim.client);
      if (pick.button === "right") {
        // A reset cancels the old press; its release cannot command the new battle.
        if (!held) return;
        const release = pick.release && world && groundUnderRay(world.view, pick.release);
        const intent = facedIntent(held.intent, held.pick, release && [release[0], release[1]]);
        releasedAction.beginRelease();
        control.onPointer(held.pick, intent);
        releasedAction.cancelCapture();
      } else {
        const pointer = semanticPick(pick);
        if (pointer) control.onPointer(pointer);
      }
    },
    [
      semanticPick,
      control,
      world,
      sim.client,
      sim.latest,
      purchasePlacement,
      clickPurchase,
      releasedAction,
    ],
  );
  const semanticRevision = usePreviewRevision(
    side,
    observation,
    control.acks,
    knownKey,
    clearedCount,
    clearingEpoch,
  );
  // eslint-disable-next-line complexity -- debt: held-drag intent, pending-order marks and ruler read the same closure refs; splitting needs a pure intent resolver that returns marks instead of writing refs.
  const onCursor: ViewportCursor = (pointer, camera, _project, renderScale) => {
    // The battle as the last frame drew it: the picks and drawn positions
    // below are that frame's, so its units and tick are too. Nothing drawn
    // yet, nothing to point at.
    const shown = drawnObservation.current;
    const active =
      pointer.position &&
      pointer.ray &&
      !pointer.cameraDragging &&
      !replay &&
      !scripted &&
      (!pointer.rightPress || releasedAction.capturedFor(sim.client) !== null) &&
      sim.client &&
      world;
    if (purchasePlacement.arriving) {
      const pose =
        active && inputEnabled
          ? placementPose(
              pointer,
              placementUnder(units, purchasePlacement.arriving, aloft, surfaceZ, world.view),
            )
          : null;
      purchaseGhost.current = purchasePlacement.at(
        pose?.at ?? null,
        active ? sim.client : null,
        `${semanticRevision.current.version}`,
        pose?.facing,
      );
      pointerPaint.update(
        null,
        [],
        surfaceZ,
        metresPerPxAt(camera.distance, camera.fovY, window.innerHeight),
        null,
        renderScale,
        purchaseGhost.current,
      );
      return active ? (purchaseGhost.current?.valid === false ? "blocked" : "default") : null;
    }
    const pick = active
      ? semanticPick({
          ...pointer.position!,
          ray: pointer.ray!,
          instance: pickBox(pointer.ray!, drawn.current.picks),
          button: isArmed(control.mode) ? "left" : "right",
          ctrl: pointer.ctrl,
          shift: pointer.shift,
          time: 0,
        })
      : null;
    const held = pointer.rightPress ? releasedAction.capturedFor(sim.client) : null;
    const release =
      held &&
      pointer.rightDragging &&
      pointer.ray &&
      world &&
      groundUnderRay(world.view, pointer.ray);
    const intent =
      active && held
        ? facedIntent(
            reconcilePointerIntent(held.intent, shown, commanding),
            held.pick,
            release ? [release[0], release[1]] : null,
          )
        : pick
          ? control.intentAt(pick)
          : ({ kind: "none" } as PointerIntent);
    shownIntent.current = intent;
    const pending = releasedAction.released;
    const accepted = pending?.ack;
    const released = pending?.intent;
    // Until admission answers, the released request keeps its marks and facing.
    const waiting = !held && pending && !accepted && released;
    const queryIntent = waiting ? reconcilePointerIntent(released, shown, commanding) : intent;
    const previewSnapshot = orderPreview.update(
      active ? previewForIntent(queryIntent) : null,
      active ? sim.client : null,
      `${shown?.tick ?? 0}:${control.acks[0]?.seq ?? 0}:${semanticRevision.current.version}`,
      `${held?.generation ?? (waiting ? pending.generation : 0)}`,
      shown?.own,
    );
    // Unanswered, the point ordered carries the page's own mark.
    const marks = pointerPaint.markers(
      previewSnapshot.answered ? previewSnapshot : { destinations: previewSnapshot.provisional },
      shown?.own ?? [],
    );
    const showDestinations =
      queryIntent.kind === "move" ||
      queryIntent.kind === "occupy_building" ||
      queryIntent.kind === "unload";
    let preview = (held || waiting) && showDestinations ? marks : [];
    // A right-drag that sets facing shows each unit as it will stand; a plain
    // right-click does not.
    dragGhosts.current =
      active && held && pointer.rightDragging && showDestinations
        ? previewSnapshot.destinations.flatMap((d) => {
            const unit = shown?.own.find((u) => u.id === d.unit);
            if (!unit || !d.placed) return [];
            return [
              {
                kind: unit.kind,
                at: d.goal,
                soldiers: ownSoldiers(unit, d.spots),
                yaw: d.facing,
                look: GHOST_SELECTED,
              },
            ];
          })
        : [];
    if (!held && accepted) {
      if ((shown?.tick ?? 0) >= accepted.applied_tick) releasedAction.retire(pending);
      else if (
        !accepted.error &&
        released &&
        (released.kind === "move" ||
          released.kind === "occupy_building" ||
          released.kind === "unload")
      )
        // The accepted marks stand in for an order its units do not publish
        // yet, so the order overlay leaves those units out (`orderReveal`);
        // drawn at its opacity they would vanish. They show in full until
        // the order lands and the overlay's own flash takes over.
        preview = pointerPaint.markers(
          { destinations: accepted.placement?.destinations ?? [], building: accepted.building },
          shown?.own ?? [],
        );
    }
    const ruler =
      active && control.showOrders
        ? rulerAt(
            pointer.ray,
            world!,
            control.selectedUnits,
            drawnAt.current,
            rules,
            units,
            surfaceZ,
          )
        : null;
    const attackGround =
      active && intent.kind === "attack" && intent.target.kind === "ground" && shown
        ? attackGroundMarks(
            [intent.target.point[0], intent.target.point[1]],
            control.selectedUnits,
            drawnAt.current,
            rulesText,
          )
        : null;
    const action = active
      ? pointerCursor(
          intent,
          !held && pick?.unit != null,
          !held && accepted && released && (shown?.tick ?? 0) < accepted.applied_tick
            ? { released, ack: accepted }
            : null,
          { intent: queryIntent, preview: previewSnapshot },
        )
      : null;
    // An unload's destination carries the unload mark, held or released.
    const unloading = (!held && pending ? released?.kind : queryIntent.kind) === "unload";
    pointerPaint.update(
      ruler,
      active ? (unloading ? preview.map((mark) => ({ ...mark, unload: true })) : preview) : [],
      surfaceZ,
      metresPerPxAt(camera.distance, camera.fovY, window.innerHeight),
      attackGround,
      renderScale,
      null,
      garrisonTarget(action, intent, shown, drawnBuildings, surfaceZ),
    );
    return action;
  };
  /** Drag-select own units whose drawn position falls in the rectangle. */
  const onBox = useCallback(
    (box: LabBox) =>
      control.selectInRect((u) => {
        const at = drawnAt.current.get(u.id) ?? u.position;
        const p = box.project(at[0], at[1], at[2] + 1);
        return (
          !!readouts.current?.inRect(u.id, box) ||
          (!!p && p[0] >= box.x0 && p[0] <= box.x1 && p[1] >= box.y0 && p[1] <= box.y1)
        );
      }, box.shift),
    [control],
  );

  const { onReady, onDispose, gpuAllocations, gpuFrame } = useViewportGpu(
    useBattleWarmup({
      scope: presentation,
      units,
      enemy,
      factions: factionsForArt,
      observation,
      onReady: sim.onViewportReady,
    }),
  );

  const probes = {
    tick: () => sim.latest.current?.tick ?? 0,
    observation: () => sim.latest.current,
    /** The last drawn frame: the tick of the observation it presented, and
     *  its presentation clock in ticks. Null before the first frame. */
    presented: () =>
      drawnClock.current === null || drawnObservation.current === null
        ? null
        : {
            tick: drawnObservation.current.tick,
            clock: drawnClock.current * rules.tick_hz,
            side: drawnObservation.current.groundPatch.side,
            epoch: drawnObservation.current.groundPatch.epoch,
          },
    digest: () => sim.digest.current,
    error: () => sim.error,
    status: () => sim.status,
    selected: () => control.selected,
    select: (ids: number[]) => control.setSelected(ids),
    pointerIntent: () => shownIntent.current,
    pointerResolution: () => {
      const { state, building } = orderPreview.snapshot;
      return { state, building };
    },
    movePreview: () => pointerPaint.preview,
    /** The currently selected reinforcement's live cursor ghost. */
    purchaseGhost: () => purchaseGhost.current,
    ruler: () => pointerPaint.shown,
    garrisonTarget: () => pointerPaint.garrison,
    /** Space held: the order overlay shows every own unit (D2+). */
    showOrders: () => control.showOrders,
    acks: () => control.acks,
    command: (order: Order, queued = false) => control.issue(order, queued),
    ...sim.authorityProbes,
    surfaceZ,
    /** The static ground under a point: its kind and whether units cross it;
     *  null off the map. */
    surfaceAt: (x: number, y: number) => {
      const s = world?.view.surface_at(x, y);
      return s?.length
        ? { kind: world!.layout.surfaceKinds[s[5]], forest: s[6] === 1, traversable: s[7] === 1 }
        : null;
    },
    /** The static map's props of `kind` nearest (x, y), nearest first. */
    propsNear: (kind: string, x: number, y: number, count = 1) =>
      (placedProps ?? [])
        .filter((p) => p.kind === kind)
        .map((p) => ({ ...p, distance: Math.hypot(p.center[0] - x, p.center[1] - y) }))
        .sort((a, b) => a.distance - b.distance)
        .slice(0, count),
    ...knownBuildings.probes,
    /** The boxes the fog is cut by: the occluders the side knows stand. */
    fogOccluders: () => sim.client?.presentation?.fogOccluders() ?? [],
    /** The side's known craters (`knownCraters`); null before any ground. */
    craters: (binM = 16) => sim.ground.current && knownCraters(sim.ground.current, binM),
    /** Sound: voices, budget, holds and counts; null before audio starts. */
    sound: () => audio?.stats() ?? null,
    ...presentationProbes(() => presentationRef.current),
  };

  const visibleGhostReadouts = useMemo(
    () =>
      aircraftOrderReadouts(
        observation?.own ?? [],
        control.showOrders,
        revealed,
        (kind) => highAir(units.type(kind)),
        surfaceZ,
        aloft,
      ),
    [control.showOrders, observation, revealed, units, surfaceZ, aloft],
  );

  return {
    world,
    meshes,
    /** The admitted frame inputs and viewport GPU lifetime. */
    viewport: {
      frame,
      presentation: admittedPresentationRef,
      appearances: modelAppearancesFeed,
      onReady,
      onDispose,
    },
    /** What the side knows stands, for a viewport that draws it: the map's
     *  buildings, and what the camera keeps clear of. */
    known: knownBuildings.known,
    /** The map's buildings and their parts (null before the map loads). */
    buildingIndex: drawnBuildings,
    /** The player's pointer on a viewport that takes orders. */
    input: { onPick, onRightPress, onCursor, onBox, pointerMarks: pointerPaint.feed },
    /** What renderer fog is drawn from; `fogFeed` carries it to the viewport. */
    fog,
    fogFeed,
    rules,
    catalog,
    units,
    contacts,
    sim,
    control,
    allies,
    /** The battle's players in scenario order, and the one the client plays. */
    seats: players,
    viewer,
    purchase: skirmish
      ? {
          faction: faction!,
          cards: units.cards,
          placing: purchasing,
          choose: choosePurchase,
          ready: () => void control.issue({ kind: "ready" }),
        }
      : null,
    /** Which own units' order marks show, at what opacity (`OrderReveal`):
     *  every unit's with Space held, an order's units' as it flashes. */
    revealed,
    pointerPaint,
    surfaceZ,
    /** The battle's sound (null unless `sound`), and its per-frame listener. */
    audio,
    hear,
    /** The info panels' layer, and its placing and hover (`usePanels`). */
    readouts,
    ...usePanels(readouts, { drawn, drawnAt, drawnEnemyAt, drawnClock }, surfaceZ, control),
    refusals: {
      refusals: sim.refusals,
      cardBox: (unit: number) => readouts.current?.cardBox(unit) ?? null,
    },
    ghostReadouts: visibleGhostReadouts,
    gpuAllocations,
    gpuFrame,
    probes,
  };
}

export type BattleSession = ReturnType<typeof useBattleSession>;
