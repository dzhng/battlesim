// One side's live battle session: the static world and its meshes, the worker
// authority, the player command path, the drawn units (interpolated; soldiers
// and vehicles posed by the pose driver as models, each picked by the
// simulation's box for its body), the buildings it knows stand or fell, the
// pick and box-select adapters over what is drawn, and the
// base lab probes. The battle view and every lab that plays a battle
// share it; routes add only what they show.
import { admitScenario, presentationRules } from "@web/battle/catalog/sets";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { ContactPresentation } from "@web/battle/present/contactPresentation";
import { gameContactStyle } from "./gameFog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Project, ReadoutLayerHandle } from "@web/battle/present/readouts";
import { metresPerPxAt, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { GpuAllocationCounts } from "@packages/renderer-core/src/gpuAllocations";
import { knownStanding, type PropBox } from "@packages/battle-renderer/src/models/propAppearance";
import { pickBox } from "@packages/battle-renderer/src/picking";
import {
  FRAME_FLOATS,
  type SceneBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import { buildingRemains } from "@packages/scene-assets/src/authority";
import {
  buildingObstacles,
  buildingPartProps,
  knownOf,
} from "@packages/battle-renderer/src/buildingObstacles";
import { useWorldLayers } from "./useWorldLayers";
import { gameRuntime, mapAppearances, useMapAppearances } from "./gameAppearances";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { gameHud } from "@web/battle/present/hudTheme";
import { gameOrderFlash, gameXray } from "./gameOverlay";
import {
  NOTHING_REVEALED,
  OrderReveal,
  sameReveal,
  type RevealedOrders,
} from "@web/battle/present/orderReveal";
import { highAir } from "@packages/scene-assets/src/units";
import { PurchasePlacementControl, type PurchaseGhost } from "@web/battle/input/purchasePlacement";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import {
  dragFacing,
  reconcilePointerIntent,
  type PointerIntent,
  type PointerPick,
} from "@web/battle/input/pointerIntent";
import {
  PointerPaint,
  attackGroundSpread,
  rulerAt,
  previewForIntent,
  cursorForIntent,
  cursorForRelease,
  intentForOrder,
  samePointerIntent,
  previewContextIdentity,
} from "./pointerPaint";
import type { KnownPropView, ObservationView } from "@web/battle/sim/observationView";
import type { Order, SideName } from "@web/battle/sim/protocol";
import type { Player } from "@web/battle/prepare/protocol";
import type {
  LabBox,
  LabPick,
  ViewportFrame,
  ViewportGpu,
  ViewportPointer,
  ViewportCursor,
} from "./LabViewport";
import {
  drawnUnits,
  pickedUnit,
  pickToPointer,
  sideInstances,
  type DrawnInstances,
} from "./sideInstances";
import { commandedEconomy, commandingPlayer, takesOrders } from "./commandingPlayer";
import { allyOwners, controlled, playerIndex } from "@web/battle/input/controlled";
import { useSimSession } from "./useSimSession";
import type { BattleSessionOptions, ScenarioRules } from "./battleSessionOptions";
import { groundUnderRay, useMapBuildings, useStaticWorld } from "./useStaticWorld";
import { createBattleAudio } from "./soundFeed";
import { UnitResponses } from "./unitResponses";
import { assignments as voiceAssignments } from "@fixtures/voices.json";
import { factionVoices } from "@packages/battle-audio/src/voices/voiceCatalog";
import { fieldedBattle, frameCounts, presentedCamera } from "./presentedFrame";
import { gameLight } from "./gameLight";
import type { FogRequest, PresentedBattle, ScenePresentation } from "@web/battle/sim/presentation";
import type { ViewportCamera } from "@packages/renderer-core/src/viewportCamera";
import { useAppAudio } from "./AppAudio";
import { useFeed } from "./feed";
import type { Vec3 } from "math";
import {
  GHOST_BLOCKED,
  GHOST_OWN,
  GHOST_SELECTED,
  fieldedSoldiers,
  ghostBodies,
  ghostAloft,
  aircraftOrderReadouts,
  orderedGhost,
  ownSoldiers,
  type UnitGhost,
} from "./unitGhosts";
import { byReference, pendingGhosts } from "./pendingGhosts";

const PURCHASE_BLOCKED = [...gameHud.bad, gameXray.selected[3]] as const;
/** Ground heights the page remembers before starting afresh. */
const SURFACE_HEIGHTS_MAX = 1 << 20;

// eslint-disable-next-line complexity -- debt: lifecycle, observation, input and presentation share 19 refs; split needs those refs grouped by concern first, so each concern can become its own hook.
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
  xray = true,
  prepared,
  scenarioArt = false,
}: BattleSessionOptions) {
  const appAudio = useAppAudio();
  const catalog = useSessionCatalog();
  // The session draws, reads and loads its own catalog's units; a scenario
  // fielding a unit the catalog lacks is refused by name, not left undrawn.
  const { map, rules, rulesText, skirmish, units, fielded, players } = useMemo(() => {
    const parsed = JSON.parse(scenario) as {
      map: unknown;
      rules: ScenarioRules;
      players?: Player[];
      skirmish?: object;
      units?: { kind: string }[];
    };
    return {
      ...parsed,
      rules: presentationRules(parsed.rules, catalog),
      rulesText: JSON.stringify(parsed.rules),
      units: admitScenario(parsed, catalog),
      fielded: (parsed.units ?? []).map((u) => u.kind),
      players: parsed.players ?? [],
    };
  }, [scenario, catalog]);
  const world = useStaticWorld(map, rules);
  // Sound reads the same publications as the effects, plus the side's hearing cues.
  const audio = useMemo(
    () => (sound ? createBattleAudio(rules.tick_hz, appAudio, seed) : null),
    [sound, rules.tick_hz, appAudio, seed],
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
  const noteDecoded = useCallback(
    (o: ObservationView, digest: string) => {
      const presentation = presentationRef.current;
      if (presentation) audio?.note(presentation.wasm);
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
  const viewer = players.find((p) => p.id === commanding && p.controller === "human") ?? null;
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
  /** The chosen reinforcement must be requested before the cursor asks the
   *  presentation to draw its ghost.  Keep this in React state so the art
   *  request advances on the same render as the picker choice. */
  const [purchasing, setPurchasing] = useState<string | null>(null);
  useEffect(() => {
    responses.current =
      audio && sim.client ? new UnitResponses(audio, units, faction, rules.tick_hz) : null;
  }, [audio, sim.client, units, faction, rules.tick_hz]);
  // A battle may purchase or reveal any unit in the catalog after its opening
  // scenario. Keep every unit appearance resident from map startup so the
  // cursor preview never installs a new GPU generation during interaction.
  const { observation } = sim;
  const requestedKindKey = useMemo(() => {
    const kinds = new Set(fielded);
    if (purchasing) kinds.add(purchasing);
    for (const unit of observation?.own ?? []) kinds.add(unit.kind);
    for (const unit of observation?.identified ?? []) kinds.add(unit.kind);
    for (const pending of observation?.skirmish?.players.flatMap((player) => player.pending) ?? [])
      kinds.add(pending.kind);
    return [...kinds].sort().join("\0");
  }, [fielded, observation, purchasing]);
  const requestedKinds = useMemo(
    () => (requestedKindKey ? requestedKindKey.split("\0") : []),
    [requestedKindKey],
  );
  const factionsForArt = useMemo(
    () =>
      skirmish
        ? new Set(players.flatMap((player) => (player.faction ? [player.faction] : [])))
        : null,
    [skirmish, players],
  );
  const sceneryForArt = useMemo(() => {
    if (!scenarioArt) return undefined;
    const candidate = map as { props?: unknown[]; forests?: unknown[] } | null;
    return (candidate?.props?.length ?? 0) === 0 && (candidate?.forests?.length ?? 0) === 0
      ? null
      : undefined;
  }, [map, scenarioArt]);
  // A shout heard in the open is a bark in its kind's set for its side's
  // faction; an unseen enemy's is drawn from the enemy faction's sets. The
  // requested kinds still scope the simulation's descriptions and voices;
  // the renderer's unit art is resident for the complete catalog.
  // Outside a skirmish the factions are unknown, so shouts are silent.
  const enemy = skirmish ? sideFaction(side === "blue" ? "red" : "blue") : null;
  useEffect(() => {
    if (!audio || !faction || !enemy) return;
    const rows = units.ids.map((id) => ({ id, factions: units.type(id).roster?.factions ?? [] }));
    audio.speakers({
      own: factionVoices(voiceAssignments, rows, faction),
      enemy: factionVoices(voiceAssignments, rows, enemy),
    });
  }, [audio, units, faction, enemy]);
  const contactPresentation = useMemo(
    () => (sim.client ? new ContactPresentation(rules.tick_hz, gameContactStyle.fade_s) : null),
    [sim.client, rules.tick_hz],
  );
  const contacts = useMemo(
    () => contactPresentation?.update(observation?.contacts ?? [], observation?.tick ?? 0) ?? [],
    [contactPresentation, observation],
  );
  // The last drawn frame's presentation clock: the callouts' nudges ease on
  // it, and an order's flash starts at it.
  const drawnClock = useRef<number | null>(null);
  // The publication the last drawn frame drew (`FrameSample.observation`).
  // Whatever reads the drawn frame (its picks, its drawn positions) reads the
  // battle from this, not from React's `observation`, which lags it by a render.
  const drawnObservation = useRef<ObservationView | null>(null);
  // Which units' order marks show (Space, or an order's flash), refreshed
  // each frame and kept as state only when it changes.
  const orderReveal = useMemo(() => new OrderReveal(gameOrderFlash), []);
  /** Bridge the released preview until the publication contains its order. */
  const [pointerPaint] = useState(() => new PointerPaint(units));
  const pendingAction = useRef<{ order: Order; queued: boolean; generation: number } | null>(null);
  const pressGeneration = useRef(0);
  const captured = useRef<{
    pick: PointerPick;
    intent: PointerIntent;
    generation: number;
    client: typeof sim.client;
  } | null>(null);
  const shownIntent = useRef<PointerIntent>({ kind: "none" });
  /** Where a held right-drag's units will stand. */
  const dragGhosts = useRef<UnitGhost[]>([]);
  const [revealed, setRevealed] = useState<RevealedOrders>(NOTHING_REVEALED);
  const revealedRef = useRef(revealed);
  const noteOrder = useCallback(
    (order: Order, queued: boolean) => {
      pendingAction.current = { order, queued, generation: captured.current?.generation ?? 0 };
      if (drawnClock.current !== null) orderReveal.noteOrder(order, drawnClock.current);
    },
    [orderReveal],
  );
  const control = useUnitControl(
    replay || scripted ? null : sim.client,
    observation,
    {
      onIssue: noteOrder,
      onAck: (order, ack, queued) => responses.current?.acknowledged(order, ack, queued),
      onSelect: (added, clicked) => responses.current?.selected(added, clicked),
    },
    inputEnabled && takesOrders(players, commanding),
    commanding,
  );
  /** The side's units an ally owns, with their panels' owner lines. */
  const allies = useMemo(() => allyOwners(observation, commanding), [observation, commanding]);
  const [purchasePlacement] = useState(() => new PurchasePlacementControl());
  const purchaseGhost = useRef<PurchaseGhost | null>(null);
  /** The ghosts the authority accepted, for their facing: a pending purchase
   *  carries only its kind and destination. */
  const placedGhosts = useRef<PurchaseGhost[]>([]);
  const cancelPurchase = useCallback(() => {
    purchasePlacement.cancel();
    purchaseGhost.current = null;
    setPurchasing(null);
  }, [purchasePlacement]);
  const choosePurchase = useCallback(
    (variant: string) => {
      control.setSelected([]);
      control.setMode("move");
      purchasePlacement.choose(variant);
      setPurchasing(variant);
    },
    [control, purchasePlacement],
  );
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || !purchasePlacement.variant) return;
      event.preventDefault();
      cancelPurchase();
    };
    window.addEventListener("keydown", onKey, { capture: true });
    return () => window.removeEventListener("keydown", onKey, { capture: true });
  }, [purchasePlacement, cancelPurchase]);
  // A new battle carries no flash over.
  useEffect(() => {
    cancelPurchase();
    placedGhosts.current = [];
    orderReveal.clear();
    pendingAction.current = null;
    captured.current = null;
    pointerPaint.resolvePreview(null, [], null, 0);
  }, [sim.client, orderReveal, pointerPaint, cancelPurchase]);

  // Every building is drawn from its template's rows, as instances of kit
  // modules, standing or fallen: no fitted model stands for one or for its
  // remains.
  const mapBuildings = useMapBuildings(world);
  const placedProps = mapBuildings?.props ?? null;
  const drawnBuildings = mapBuildings?.index ?? null;
  // The catalog, and the kits this map's buildings and stand-in boxes draw
  // from and its region's looks, fetched once the map is known: the loading
  // cover stays up for them.
  const appearances = useMapAppearances(
    drawnBuildings?.placed ?? null,
    world?.exports.buildings.regionalFamily ?? null,
    (world?.exports.props.length ?? 0) > 0 || (drawnBuildings?.placed.template.length ?? 0) > 0,
    null,
    factionsForArt,
    sceneryForArt,
  );
  // The ground, and the art its scenery and grass draw from; its props and
  // what the side knows of them (rubble, wrecks, shoved bodies where it last
  // saw them) are the battle's presentation's to draw.
  const meshes = useWorldLayers(world, appearances, sceneryForArt);
  const knownProps = observation?.knownProps;
  const knownKey = useMemo(() => JSON.stringify(knownProps ?? []), [knownProps]);
  // What the side knows of the map's buildings, apart from everything else
  // it knows: what follows changes only when that does.
  const buildingParts = useMemo(() => world && buildingPartProps(world.exports.buildings), [world]);
  const knownBuildingsKey = useMemo(
    () =>
      buildingParts &&
      JSON.stringify(knownOf(JSON.parse(knownKey) as KnownPropView[], buildingParts)),
    [knownKey, buildingParts],
  );
  // The map's buildings, which the battle's presentation draws in the state
  // the side's publication knows them in.
  const buildings = useMemo<SceneBuildings | null>(
    () =>
      drawnBuildings && {
        placed: drawnBuildings.placed,
        partBuilding: drawnBuildings.partBuilding,
        shells: buildingRemains(units).shells,
      },
    [drawnBuildings, units],
  );
  const buildingsFeed = useFeed(buildings);
  // What the camera keeps clear of: the ground, and every building part the
  // side knows stands (a fallen one's remains once it has seen the fall).
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

  // The page's world is the map as loaded: a point's height never changes,
  // and the overlays drape their marks at the same points every publication.
  // Each height is asked of the module once, until the memory is full.
  const surfaceZ = useMemo(() => {
    let heights = new Map<number, Map<number, number>>();
    let size = 0;
    return (x: number, y: number) => {
      let row = heights.get(x);
      let z = row?.get(y);
      if (z === undefined) {
        z = world?.view.surface_at(x, y)[0] ?? 0;
        if (size >= SURFACE_HEIGHTS_MAX) {
          heights = new Map();
          size = 0;
          row = undefined;
        }
        if (!row) heights.set(x, (row = new Map()));
        row.set(y, z);
        size++;
      }
      return z;
    };
  }, [world]);
  const cameraObstacles = useMemo(() => {
    if (!placedProps || !buildingParts || !knownBuildingsKey) return null;
    const started = performance.now();
    const view = buildingObstacles(
      placedProps,
      JSON.parse(knownBuildingsKey) as KnownPropView[],
      buildingParts,
      surfaceZ,
    );
    return { view, buildMs: performance.now() - started };
  }, [placedProps, buildingParts, knownBuildingsKey, surfaceZ]);
  const cameraObstaclesFeed = useFeed(cameraObstacles?.view ?? null);

  // What the last frame drew: which unit or enemy each pick box is, and each
  // own unit's drawn (interpolated) position.
  const drawn = useRef<DrawnInstances>({ picks: [], owners: [], enemies: [], rings: [] });
  const drawnAt = useRef(new Map<number, Readonly<Vec3>>());
  const drawnEnemyAt = useRef(new Map<number, Readonly<Vec3>>());
  const readouts = useRef<ReadoutLayerHandle>(null);
  // The models layer installs only what the battle draws as models.
  const modelAppearances = useMemo<InstalledAppearances | null>(
    () =>
      appearances && world && drawnBuildings
        ? mapAppearances(
            appearances,
            world,
            drawnBuildings.placed,
            true,
            skirmish !== undefined || world.exports.forests.length > 0,
          )
        : null,
    [appearances, world, drawnBuildings, skirmish],
  );
  const modelAppearancesFeed = useFeed(modelAppearances);

  // What the battle draws, handed to the presentation once the runtime
  // catalog is loaded: its units and effects, which Rust resolves from the
  // files the page reads. Another fielded kind loads again, the last battle
  // drawn meanwhile; another scenario or side draws none until its own.
  const factions = skirmish
    ? ([sideFaction("blue")!, sideFaction("red")!] as [string, string])
    : undefined;
  const presentation = sim.client?.presentation ?? null;
  const artLoaded = appearances !== null;
  const [fieldedFor, setFielded] = useState<{
    scenario: string;
    side: SideName;
    battle: PresentedBattle;
  } | null>(null);
  const worldInput =
    fieldedFor?.scenario === scenario && fieldedFor.side === side ? fieldedFor.battle : null;
  const [fieldError, setFieldError] = useState<unknown>(null);
  if (fieldError) throw fieldError;
  useEffect(() => {
    if (!artLoaded) return;
    let live = true;
    const facts = {
      side,
      tickHz: rules.tick_hz,
      sun: { azimuth: gameLight.sun_azimuth, elevation: gameLight.sun_elevation },
      soldier: rules.physics,
      xray: gameXray,
      blocked: PURCHASE_BLOCKED,
    };
    fieldedBattle({
      rules: rulesText,
      catalog: units.view,
      kinds: requestedKinds,
      factions,
      runtime: gameRuntime,
      ...facts,
    }).then(
      (battle) => {
        if (!live) return;
        setFielded({ scenario, side, battle });
      },
      (error: unknown) => live && setFieldError(error),
    );
    return () => {
      live = false;
    };
  }, [artLoaded, scenario, rulesText, units, requestedKinds, factions, rules, side]);
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
    [commanding],
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
      if (time == null || !published || !presentation) {
        drawnObservation.current = null;
        return null;
      }
      drawnClock.current = time;
      drawnObservation.current = published;
      // The order just issued has not landed: its units still publish the old one.
      const pending = pendingAction.current;
      const ack = pending && control.acks.find(({ order }) => order === pending.order)?.ack;
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
      // Ghosts: purchases placed, every ordered destination with Space held,
      // a held right-drag's destinations, and the purchase being placed.
      const placement = purchaseGhost.current;
      const ghosts = [...placedGhostList.of(published.skirmish), ...dragGhosts.current];
      if (control.showOrders)
        for (const unit of published.own) {
          if (!reveal.has(unit.id)) continue;
          const ghost = orderedGhost(unit, GHOST_OWN);
          if (ghost) ghosts.push(ghost);
        }
      if (placement && inputEnabled)
        ghosts.push({
          kind: placement.variant,
          at: placement.destination,
          soldiers: fieldedSoldiers(placement.spots),
          yaw: placement.facing,
          look: placement.valid === false ? GHOST_BLOCKED : GHOST_SELECTED,
        });
      const presented = presentation.frame(
        time,
        presentedCamera(viewport),
        {
          selected: control.selected,
          xray,
          player: playerIndex(published, commanding),
          ghosts: ghostBodies(
            ghosts,
            (kind) => Boolean(units.hull(kind)),
            surfaceZ,
            ghostAloft(units, rules.air),
          ),
        },
        generation,
      );
      const { own, enemies } = drawnUnits(presented);
      drawn.current = sideInstances(presented, own, enemies, published, units);
      drawnAt.current = new Map([...own].map(([id, u]) => [id, u.position]));
      drawnEnemyAt.current = new Map([...enemies].map(([id, u]) => [id, u.position]));
      const { models: counts, effects: effectCounts } = frameCounts(presented);
      return {
        picks: drawn.current.picks,
        presented,
        counts,
        clock: time,
        effectCounts,
        ground: sim.ground.current,
      };
    },
    [
      observation,
      sim.latest,
      sim.ground,
      presentation,
      worldInput,
      units,
      orderReveal,
      control.showOrders,
      control.acks,
      control.selected,
      xray,
      placedGhostList,
      inputEnabled,
      surfaceZ,
      commanding,
    ],
  );
  /** Sound for the last frame, heard from its camera: call once a frame. */
  const hear = useCallback(() => {
    const presentation = sim.client?.presentation;
    if (audio && presentation) audio.update(presentation.wasm);
  }, [audio, sim.client]);

  const semanticPick = useCallback(
    (pick: LabPick): PointerPick | null => {
      if (!world) return null;
      const pointer = pickToPointer(world, drawn.current, pick, observation?.contacts);
      const panel = readouts.current?.pick(pick.x, pick.y);
      return panel ? { ...pointer, ...panel } : pointer;
    },
    [world, observation],
  );
  const onRightPress = useCallback(
    (pick: LabPick | null) => {
      const pointer = pick && semanticPick(pick);
      captured.current = pointer
        ? {
            pick: pointer,
            intent: control.intentAt(pointer),
            generation: ++pressGeneration.current,
            client: sim.client,
          }
        : null;
    },
    [semanticPick, control, sim.client],
  );
  const facedIntent = (
    intent: PointerIntent,
    pick: PointerPick,
    release?: [number, number] | null,
  ): PointerIntent => {
    if (intent.kind !== "move" && intent.kind !== "occupy_building") return intent;
    const facing = dragFacing({ ground: pick.ground, facingTo: release });
    return { ...intent, facing };
  };
  const onPick = useCallback(
    (pick: LabPick) => {
      if (purchasePlacement.variant) {
        if (pick.button === "right") cancelPurchase();
        else
          void purchasePlacement.confirm(control.issue).then((accepted) => {
            if (accepted) placedGhosts.current.push(accepted);
            if (accepted && !purchasePlacement.variant) {
              purchaseGhost.current = null;
              setPurchasing(null);
            }
          });
        return;
      }
      const held = captured.current;
      if (pick.button === "right") {
        // A reset cancels the old press; its release cannot command the new battle.
        if (!held || held.client !== sim.client) return;
        const release = pick.release && world && groundUnderRay(world.view, pick.release);
        control.onPointer(
          held.pick,
          facedIntent(held.intent, held.pick, release && [release[0], release[1]]),
        );
        captured.current = null;
      } else {
        const pointer = semanticPick(pick);
        if (pointer) control.onPointer(pointer);
      }
    },
    [semanticPick, control, world, sim.client, purchasePlacement, cancelPurchase],
  );
  const eligibilityIdentity = JSON.stringify(
    (observation?.own ?? []).map((u) => [
      u.id,
      u.kind,
      u.members.length,
      u.hold?.host.kind,
      u.hold?.host.id,
      u.hold?.phase,
      u.queue,
    ]),
  );
  const pendingClaims = JSON.stringify(
    control.acks
      .filter(({ ack }) => !ack.error && ack.applied_tick > (observation?.tick ?? 0))
      .map(({ seq, order }) => [seq, order]),
  );
  const semanticIdentity = useMemo(
    () =>
      previewContextIdentity(
        side,
        knownKey,
        eligibilityIdentity,
        pendingClaims,
        clearedCount,
        clearingEpoch,
      ),
    [side, knownKey, eligibilityIdentity, pendingClaims, clearedCount, clearingEpoch],
  );
  const semanticRevision = useRef({ identity: "", version: 0 });
  if (semanticRevision.current.identity !== semanticIdentity) {
    semanticRevision.current = {
      identity: semanticIdentity,
      version: semanticRevision.current.version + 1,
    };
  }
  // eslint-disable-next-line complexity -- debt: purchase placement, held-drag intent, pending-order marks and ruler all read the same closure refs; the purchase-ghost branch could leave first, but the rest needs a pure intent resolver that returns marks instead of writing refs.
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
      (!pointer.rightPress || captured.current?.client === sim.client) &&
      sim.client &&
      world;
    if (purchasePlacement.variant) {
      // A held left drag pins the ghost where it was pressed and turns it
      // toward the pointer.
      const at =
        active && inputEnabled && groundUnderRay(world.view, pointer.leftPress ?? pointer.ray!);
      const facingTo =
        active && inputEnabled && pointer.leftPress
          ? groundUnderRay(world.view, pointer.ray!)
          : null;
      const facing =
        at && facingTo ? Math.atan2(facingTo[1] - at[1], facingTo[0] - at[0]) : undefined;
      purchaseGhost.current = purchasePlacement.at(
        at ? [at[0], at[1]] : null,
        active ? sim.client : null,
        `${semanticRevision.current.version}`,
        facing,
      );
      pointerPaint.update(
        null,
        [],
        surfaceZ,
        metresPerPxAt(camera.distance, camera.fovY, window.innerHeight),
        null,
        renderScale,
      );
      return active ? (purchaseGhost.current?.valid === false ? "blocked" : "default") : null;
    }
    const pick = active
      ? semanticPick({
          ...pointer.position!,
          ray: pointer.ray!,
          instance: pickBox(pointer.ray!, drawn.current.picks),
          button: "right",
          ctrl: pointer.ctrl,
          shift: pointer.shift,
          time: 0,
        })
      : null;
    const held =
      pointer.rightPress && captured.current?.client === sim.client ? captured.current : null;
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
    const pending = pendingAction.current;
    const accepted = pending && control.acks.find(({ order }) => order === pending.order)?.ack;
    const released = pending && intentForOrder(pending.order, pending.queued);
    // Until admission answers, the released request retains its resolved marks,
    // including facing. It uses the same coalesced resolver as held/hover intent.
    const waiting = !held && pending && !accepted && released;
    const queryIntent = waiting ? reconcilePointerIntent(released, shown, commanding) : intent;
    const marks = pointerPaint.resolvePreview(
      active ? previewForIntent(queryIntent) : null,
      shown?.own ?? [],
      active ? sim.client : null,
      `${shown?.tick ?? 0}:${control.acks[0]?.seq ?? 0}`,
      `${semanticRevision.current.version}:${held?.generation ?? (waiting ? pending.generation : 0)}`,
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
        ? pointerPaint.destinations.flatMap((d) => {
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
      if ((shown?.tick ?? 0) >= accepted.applied_tick) pendingAction.current = null;
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
          accepted.placement?.destinations ?? accepted.building?.destinations ?? [],
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
        ? attackGroundSpread(
            [intent.target.point[0], intent.target.point[1]],
            control.selectedUnits,
            drawnAt.current,
            units,
            catalog.weapons,
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
    );
    if (!active) return null;
    // Over an own unit the arrow is plain, unless the click would board it.
    if (!held && pick?.unit != null && intent.kind !== "board" && intent.kind !== "collect")
      return "default";
    if (!held && accepted && released && (shown?.tick ?? 0) < accepted.applied_tick) {
      const action = cursorForRelease(intent, released, accepted);
      if (action !== null) return action;
    }
    return cursorForIntent(
      intent,
      samePointerIntent(intent, queryIntent) ? pointerPaint : { state: "pending", building: null },
    );
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

  const gpu = useRef<ViewportGpu | null>(null);
  const { onViewportReady } = sim;
  const onReady = useCallback(
    (viewport: ViewportGpu) => {
      gpu.current = viewport;
      onViewportReady();
    },
    [onViewportReady],
  );
  const gpuAllocations = useCallback(
    (): GpuAllocationCounts | null => gpu.current?.allocations() ?? null,
    [],
  );
  const gpuFrame = useCallback(() => gpu.current?.frame().stats().gpu ?? null, []);

  const probes = {
    tick: () => sim.latest.current?.tick ?? 0,
    observation: () => sim.latest.current,
    /** The last drawn frame: the tick of the observation it presented, and
     *  its presentation clock in ticks. Null before the first frame. */
    presented: () =>
      drawnClock.current === null || drawnObservation.current === null
        ? null
        : { tick: drawnObservation.current.tick, clock: drawnClock.current * rules.tick_hz },
    /** The static pieces the last drawn frame drew beside the posed units (a
     *  cook-off's moving wreck and its debris): each one's appearance, state
     *  and how far its motion lowers it, metres. */
    cookOffPieces: () => presentationRef.current?.probe().cookOffPieces ?? [],
    /** The soldiers the last drawn frame lays as static corpses. */
    lying: () => presentationRef.current?.probe().lying ?? [],
    /** Where the last drawn frame draws the muzzle of the shown side's
     *  vehicle `id`'s mount `mount`, where its flash goes; null when no rig
     *  draws it. */
    drawnMuzzle: (id: number, mount: number): Vec3 | null =>
      presentationRef.current?.drawnMuzzle(id, mount) ?? null,
    digest: () => sim.digest.current,
    error: () => sim.error,
    status: () => sim.status,
    selected: () => control.selected,
    select: (ids: number[]) => control.setSelected(ids),
    pointerIntent: () => shownIntent.current,
    pointerResolution: () => ({ state: pointerPaint.state, building: pointerPaint.building }),
    movePreview: () => pointerPaint.preview,
    /** The currently selected reinforcement's live cursor ghost. */
    purchaseGhost: () => purchaseGhost.current,
    ruler: () => pointerPaint.shown,
    /** Space held: the order overlay shows every own unit (D2+). */
    showOrders: () => control.showOrders,
    acks: () => control.acks,
    command: (order: Order, queued = false) => control.issue(order, queued),
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
    reset: () => sim.restart(),
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
    /** The camera's obstacles: how many boxes, and what indexing them took. */
    cameraObstacles: () =>
      cameraObstacles && { boxes: cameraObstacles.view.count, buildMs: cameraObstacles.buildMs },
    /** The map's buildings: each one's template, owner, frame, the state the
     *  side draws it in, its parts as the map has them (`authored`) and as the
     *  side knows them (`parts`: the remains or the shell of one it has seen
     *  destroyed), which are the boxes its camera keeps clear of. */
    buildings: () => {
      if (!drawnBuildings || !knownBuildingsKey) return [];
      const fallen = presentation?.fallenBuildings() ?? [];
      const state = new Map(fallen.map((f) => [f.building, f.state]));
      const known = JSON.parse(knownBuildingsKey) as KnownPropView[];
      const { placed } = drawnBuildings;
      const box = (p: PropBox) => ({
        kind: p.kind,
        center: p.center,
        baseZ: p.baseZ,
        yaw: p.yaw,
        half: p.half,
      });
      return drawnBuildings.parts.map((parts, i) => ({
        template: placed.templates[placed.template[i]],
        owner: placed.owners[i],
        frame: [...placed.frames.subarray(i * FRAME_FLOATS, (i + 1) * FRAME_FLOATS)],
        fallen: state.has(i),
        state: state.get(i) ?? "intact",
        authored: parts.map(box),
        parts: knownStanding(parts, known, new Set(parts.map((p) => p.id))).map(box),
      }));
    },
    /** The boxes the fog is cut by: the occluders the side knows stand. */
    fogOccluders: () => sim.client?.presentation?.fogOccluders() ?? [],
    /** The side's known craters: marked cells, and the centre of the
     *  `binM`-square block holding the most (framing a shelled field). */
    craters: (binM = 16) => {
      const g = sim.ground.current;
      if (!g) return null;
      const per = Math.max(1, Math.round(binM / g.cellM));
      const bins = new Map<number, number>();
      let cells = 0;
      const marked = g.markedCells();
      for (let k = 0; k < marked.length; k += 2) {
        if ((marked[k + 1] & 255) < 64) continue;
        const [i, j] = [marked[k] % g.cols, Math.floor(marked[k] / g.cols)];
        cells++;
        const key = Math.floor(j / per) * g.cols + Math.floor(i / per);
        bins.set(key, (bins.get(key) ?? 0) + 1);
      }
      let densest: [number, number] | null = null;
      let most = 0;
      for (const [key, n] of bins)
        if (n > most) {
          most = n;
          const [bi, bj] = [key % g.cols, Math.floor(key / g.cols)];
          densest = [(bi + 0.5) * per * g.cellM, (bj + 0.5) * per * g.cellM];
        }
      return { cells, densest, most };
    },
    /** Sound: voices, budget, holds and counts; null before audio starts. */
    sound: () => audio?.stats() ?? null,
    /** Combat effects: running, drawn last frame, dropped, and the last tick noted. */
    effects: () => presentationRef.current?.probe().effects ?? null,
    /** The instances the effect pass drew last frame: shape (`SHAPE`), the
     *  point it sits on (a streak's tail), its size, and a glow's rays. */
    effectInstances: () => presentationRef.current?.probe().effectInstances ?? [],
    /** The lights the effects offered last frame: where, how far, their
     *  colour × intensity, and what cast each. */
    castLights: () => presentationRef.current?.probe().castLights ?? [],
    /** Every drawn model's muzzles in the world, where the presentation
     *  draws them (a soldier's crossfaded and kicked, a hull's articulated
     *  and rocked), with each model's origin to select the firing body before
     *  its attachment is measured. */
    muzzleSockets: () => {
      const probe = presentationRef.current?.probe();
      if (!probe) return [];
      const soldiers = probe.soldiers.flatMap((m) =>
        m.muzzle
          ? [{ appearance: m.appearance, origin: m.position, name: "muzzle", at: m.muzzle }]
          : [],
      );
      const vehicles = probe.vehicles.flatMap((m) =>
        m.muzzles.map(({ name, at }) => ({
          appearance: m.appearance,
          origin: m.position,
          name,
          at,
        })),
      );
      return [...soldiers, ...vehicles];
    },
    /** The vehicles as last posed: appearance, placement and articulation. */
    vehicles: () => presentationRef.current?.probe().vehicles ?? [],
    /** What the props draw (`ScenePresentation.propStats`): the bodies drawn,
     *  those a building draws, those no model draws, the models placed and
     *  those drawing what the side knows. */
    structureBodies: () => presentationRef.current?.propStats() ?? null,
    /** Every prop model the side draws: appearance, state and placement. */
    structures: () => presentationRef.current?.propModels() ?? [],
  };

  const visibleGhostReadouts = useMemo(
    () =>
      aircraftOrderReadouts(
        observation?.own ?? [],
        control.showOrders,
        revealed,
        (kind) => highAir(units.type(kind)),
        surfaceZ,
        ghostAloft(units, rules.air),
      ),
    [control.showOrders, observation, revealed, units, surfaceZ, rules.air],
  );

  return {
    world,
    meshes,
    /** What every battle view hands its viewport (`LabViewport`): each
     *  animation frame's drawn units and presentation clock, the battle's
     *  presentation (which draws the buildings too), the appearances its
     *  models layer installs, and the GPU's arrival. */
    viewport: {
      frame,
      presentation: presentationRef,
      appearances: modelAppearancesFeed,
      onReady,
    },
    /** What the side knows stands, for a viewport that draws it: the map's
     *  buildings, and what the camera keeps clear of. */
    known: { buildings: buildingsFeed, obstacles: cameraObstaclesFeed },
    /** The player's pointer on a viewport that takes orders. */
    input: { onPick, onRightPress, onCursor, onBox, pointerMarks: pointerPaint.feed },
    /** What renderer fog is drawn from; `fogFeed` carries it to the viewport. */
    fog,
    fogFeed,
    rules,
    /** The session's unit catalog, for views drawing its units. */
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
    /** The info panels' layer (`ReadoutLayer`'s handle), and its placing
     *  off what the last frame drew, at its presentation clock; call once
     *  per animation frame. */
    readouts,
    placePanels: (project: Project, camera: Camera3DParams, pointer: ViewportPointer) =>
      readouts.current?.place(
        project,
        camera,
        { own: drawnAt.current, enemies: drawnEnemyAt.current, ground: surfaceZ },
        drawnClock.current,
        control.showOrders,
        pointer.position
          ? {
              ...pointer.position,
              ...pickedUnit(
                drawn.current,
                pointer.ray ? pickBox(pointer.ray, drawn.current.picks) : -1,
              ),
            }
          : null,
      ),
    /** The refusal callout's inputs: the side's per-unit refusals, and
     *  where a unit's card stands. */
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
