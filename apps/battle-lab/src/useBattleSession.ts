// One side's live battle session: the static world and its meshes, the worker
// authority, the player command path, the drawn units (interpolated; soldiers
// and vehicles posed by the pose driver as models, each picked by the
// simulation's box for its body), the props the side knows stand (fitted
// appearances), the buildings it knows stand or fell (template art), the
// pick and box-select adapters over what is drawn, and the
// base lab probes. The battle view and every lab that plays a battle
// share it; routes add only what they show.
import { GAME_RULES } from "./scenarios";
import { ContactPresentation } from "@web/battle/present/contactPresentation";
import { gameContactStyle } from "./gameFog";
import type { PreparedSession } from "@web/battle/prepare/client";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Project, ReadoutLayerHandle } from "@web/battle/present/readouts";
import { metresPerPxAt, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { SoundMotion } from "@packages/battle-audio/src/soundFrame";
import type { GpuAllocationCounts } from "@packages/renderer-core/src/gpuAllocations";
import {
  apartKinds,
  buildWorldLayers,
  worldStructureBodies,
} from "@packages/battle-renderer/src/worldMesh";
import {
  knownStanding,
  drawnBy,
  PropAppearances,
  drawnStructures,
  fitMapProps,
  NO_STRUCTURES,
  sideStructures,
  type DrawnBody,
  type PropBox,
  type SideStructures,
} from "@packages/battle-renderer/src/models/propAppearance";
import { pickBox, type SoldierBody } from "@packages/battle-renderer/src/picking";
import {
  fogEyes,
  fogWorld,
  knownOccluders,
  mapOccluders,
  type FogInput,
  type FogSensorRules,
} from "@packages/battle-renderer/src/frame/fogInputs";
import {
  FRAME_FLOATS,
  type SideBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import {
  buildingObstacles,
  buildingPartProps,
  knownOf,
} from "@packages/battle-renderer/src/buildingObstacles";
import { gameBiome } from "./gameBiome";
import { knownFallen } from "./destroyedBuildings";
import {
  CookOffWatch,
  LastSeenHulls,
  cookOffModels,
  type LastHull,
  effectCookOff,
  transitionOf,
  type CookOffTransition,
} from "./cookOffs";
import { landedAfter } from "@packages/battle-renderer/src/effects/cookOff";
import { mapAppearances, useMapAppearances } from "./gameAppearances";
import { gameStandIns } from "./gameModels";
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import {
  corpseInstances,
  poseFrameInstances,
  restingModelPose,
  type CorpseInstance,
  type ModelInstance,
  type ResolveAppearance,
  type XrayOf,
} from "@packages/battle-renderer/src/models/modelInstances";
import { gameHud } from "@web/battle/present/hudTheme";
import { gameOrderFlash, gameXray, gameOrderStyle } from "./gameOverlay";
import {
  NOTHING_REVEALED,
  OrderReveal,
  sameReveal,
  type RevealedOrders,
} from "@web/battle/present/orderReveal";
import type { Faction } from "@packages/scene-assets/src/units";
import { PurchasePlacementControl, type PurchaseGhost } from "@web/battle/input/purchasePlacement";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import {
  dragFacing,
  reconcilePointerIntent,
  type PointerIntent,
  type PointerPick,
} from "@web/battle/input/pointerIntent";
import type { CursorAction } from "@web/battle/present/gameCursor";
import {
  PointerPaint,
  rulerAt,
  previewForIntent,
  cursorForIntent,
  cursorForRelease,
  intentForOrder,
  samePointerIntent,
  previewContextIdentity,
} from "./pointerPaint";
import type { PanelRules } from "@web/battle/present/panelRows";
import type { RulerRules } from "@web/battle/present/rangeRuler";
import type { KnownPropView, ObservationView } from "@web/battle/sim/observation";
import type { Order, SideName } from "@web/battle/sim/protocol";
import type { LabBox, LabPick, ViewportFrame, ViewportGpu, ViewportPointer } from "./LabViewport";
import { pickedUnit, pickToPointer, sideInstances, type DrawnInstances } from "./sideInstances";
import { createPoseDriver, ObservationFeed, type PoseRules } from "./poseFeed";
import { DrawnMuzzles } from "@packages/battle-renderer/src/models/drawnMuzzles";
import { useSimSession, type ScriptedSim } from "./useSimSession";
import { createEffectFrame, drawnMuzzleSource, effectPublication, gameEffects } from "./effectFeed";
import {
  createEffectBatch,
  EFFECT_FLOATS,
} from "@packages/battle-renderer/src/effects/effectFrame";
import { offeredCastLights } from "@packages/battle-renderer/src/light/castLights";
import { groundUnderRay, useMapBuildings, useStaticWorld } from "./useStaticWorld";
import { createBattleAudio, soundMotion } from "./soundFeed";
import { useAppAudio } from "./AppAudio";
import { useFeed } from "./feed";
import { posedSockets } from "./workbench/benchWorld";
import type { Vec3 } from "math";
import type { Pose } from "@web/battle/present/interpolate";
import { circleContains, unitCircle } from "@packages/battle-renderer/src/orderOverlay";
import type { FelledTree } from "@packages/battle-renderer/src/scenery/felled";
import { orderView } from "./battleOverlay";

const PURCHASE_BLOCKED = [...gameHud.bad, gameXray.selected[3]] as const;

/** Ground heights the page remembers before starting afresh. */
const SURFACE_HEIGHTS_MAX = 1 << 20;

/** Seconds a cook-off's pieces stay drawn, still, before the whole wreck
 *  takes over: the swap rebuilds the side's structures, and this keeps the
 *  pieces drawn until it has. */
const LANDED_HOLD_S = 0.5;

export interface BattleSessionOptions {
  /** The scenario JSON the authority runs. */
  scenario: string;
  seed: number;
  prepared?: PreparedSession;
  /** Called for every decoded frame, before its credit returns. */
  onDecoded?: (o: ObservationView, digest: string) => void;
  /** A recorded battle to replay: input is off. */
  replay?: string;
  /** Blue is played by a script (the benchmark): input is off. */
  scripted?: ScriptedSim;
  /** Whose units and identified enemies are drawn (a lab's diagnostic side switch). */
  side?: SideName;
  /** "apart": every prop kind fire can destroy is drawn apart from the
   *  world, so one seen destroyed leaves the map and its known remains (a
   *  ruin, rubble, a lighter wreck) stand in its place. */
  destroyable?: "apart";
  /** Play the battle's sound (heard from the camera `hear` is given). */
  sound?: boolean;
  /** Menus and loading covers suspend command input without losing selection. */
  inputEnabled?: boolean;
  /** Own units hidden by the world are drawn through it (the player's x-ray);
   *  off, the picture shows only what the camera sees. */
  xray?: boolean;
}

/** The rule values the scenario runs under (only what views read). Its
 *  units are the shipped catalog's (`UNITS`), as every lab scenario's are. */
export interface ScenarioRules extends PoseRules, PanelRules, RulerRules {
  tick_hz: number;
  weapons: PanelRules["weapons"] & RulerRules["weapons"];
  physics: SoldierBody & RulerRules["physics"];
  service: { radius_m: number };
  sensors: FogSensorRules;
}

export function useBattleSession({
  scenario,
  seed,
  onDecoded,
  replay,
  scripted,
  side = "blue",
  destroyable,
  sound = false,
  inputEnabled = true,
  xray = true,
  prepared,
}: BattleSessionOptions) {
  const appAudio = useAppAudio();
  const { map, rules, skirmish } = useMemo(
    () =>
      JSON.parse(scenario) as {
        map: unknown;
        rules: ScenarioRules;
        skirmish?: { factions: [Faction, Faction] };
      },
    [scenario],
  );
  const world = useStaticWorld(map, rules);
  // Combat effects: every decoded publication noted (the frame dedupes),
  // drawn at each animation frame's presentation clock.
  const effects = useMemo(() => createEffectFrame(rules.tick_hz), [rules.tick_hz]);
  const effectBatch = useMemo(() => createEffectBatch(gameEffects.capacity), []);
  // Sound reads the same publication, plus the side's hearing cues.
  const audio = useMemo(
    () => (sound ? createBattleAudio(rules.tick_hz, appAudio) : null),
    [sound, rules.tick_hz, appAudio],
  );
  useEffect(() => () => audio?.dispose(), [audio]);
  // Hulls the side watched brew up (`cookOffs.ts`): the effects blow each
  // up, and while its wreck moves it draws each frame, not
  // among the side's structures. The wreck is fitted as the side's props
  // are, once they can be. A side learns a wreck within a second of losing
  // its hull.
  const cookOffWatch = useMemo(() => new CookOffWatch(UNITS, rules.tick_hz), [rules.tick_hz]);
  const transitionFitting = useRef<{
    fit: PropAppearances;
    installed: InstalledAppearances;
  } | null>(null);
  const [transitions, setTransitions] = useState<readonly CookOffTransition[]>([]);
  // Each vehicle as last drawn: a hull that brews up is drawn whole until
  // its ammunition goes, and only then as its moving wreck.
  const lastHulls = useMemo(
    () =>
      new LastSeenHulls(UNITS, (kind) => {
        const m = UNITS.type(kind).mobility;
        const road = "tracked" in m ? m.tracked : "wheeled" in m ? m.wheeled : m.foot;
        // As the simulation stops a dead hull: full road speed lost in `wreck_stop_s`.
        return road.road_kmh / 3.6 / GAME_RULES.movement.drive.wreck_stop_s;
      }),
    [],
  );
  // Each cook-off's hull, as last seen, found once as it starts.
  const transitionHulls = useRef(new WeakMap<CookOffTransition, LastHull | null>());
  const lastDecoded = useRef(-1);
  const noteDecoded = useCallback(
    (o: ObservationView, digest: string) => {
      if (o.tick < lastDecoded.current) setTransitions([]);
      lastDecoded.current = o.tick;
      const fitting = transitionFitting.current;
      const brewed = cookOffWatch.note(o).map((c) => ({
        c,
        transition: fitting && transitionOf(c, fitting.fit, fitting.installed, rules.tick_hz),
      }));
      const moving = brewed.flatMap(({ transition }) => (transition ? [transition] : []));
      if (moving.length) setTransitions((now) => [...now, ...moving]);
      const pub = {
        ...effectPublication(o, side, UNITS),
        cookOffs: brewed.map(({ c, transition }) => effectCookOff(c, transition)),
      };
      effects.note(pub);
      audio?.note({ effects: pub, audible: o.audible });
      onDecoded?.(o, digest);
    },
    [effects, audio, side, onDecoded, cookOffWatch, rules.tick_hz],
  );
  const sim = useSimSession({ scenario, seed, onDecoded: noteDecoded, replay, scripted, prepared });
  const { observation } = sim;
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
  // The tick of the observation the last drawn frame presented: React state,
  // so it can lag the clock, which follows each publication as it arrives.
  const drawnTick = useRef<number | null>(null);
  // Which units' order marks show (Space, or an order's flash), refreshed
  // each frame and kept as state only when it changes.
  const orderReveal = useMemo(() => new OrderReveal(gameOrderFlash), []);
  /** Bridge the released preview until the publication contains its order. */
  const [pointerPaint] = useState(() => new PointerPaint());
  const pendingAction = useRef<{ order: Order; queued: boolean; generation: number } | null>(null);
  const pressGeneration = useRef(0);
  const captured = useRef<{
    pick: PointerPick;
    intent: PointerIntent;
    generation: number;
    client: typeof sim.client;
  } | null>(null);
  const shownIntent = useRef<PointerIntent>({ kind: "none" });
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
    noteOrder,
    inputEnabled,
  );
  const [purchasePlacement] = useState(() => new PurchasePlacementControl());
  const [purchasing, setPurchasing] = useState<string | null>(null);
  const purchaseGhost = useRef<PurchaseGhost | null>(null);
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
    true,
  );
  // Props that can move (shoved) or be destroyed ("apart") are drawn from
  // what the side knows, apart from the world.
  const apart = useMemo(
    () => (world ? apartKinds(world.layout, destroyable === "apart") : []),
    [world, destroyable],
  );
  const meshes = useMemo(
    () =>
      world &&
      appearances &&
      buildWorldLayers(
        world.exports,
        world.layout,
        gameBiome,
        "surface",
        apart,
        appearances,
        gameStandIns,
      ),
    [world, apart, appearances],
  );
  // What the side knows stands, rebuilt only when knowledge changes: the
  // props it has learned (rubble, wrecks, shoved bodies where it last saw
  // them), and the map's props drawn apart that it has not seen fall or move,
  // each a fitted appearance. Known rubble replaces its wall in the same
  // list, a known shoved body its own map pose; an unseen fall or shove
  // leaves the map's prop standing.
  const knownProps = observation?.knownProps;
  const knownKey = useMemo(() => JSON.stringify(knownProps ?? []), [knownProps]);
  const props = useMemo(
    () =>
      world && placedProps && appearances
        ? {
            map: placedProps,
            fit: new PropAppearances(
              appearances,
              world.layout,
              world.exports.buildings.regionalFamily,
              gameStandIns,
            ),
          }
        : null,
    [world, placedProps, appearances],
  );
  // What the side knows of the map's buildings, apart from everything else
  // it knows: what follows changes only when that does.
  const buildingParts = useMemo(() => world && buildingPartProps(world.exports.buildings), [world]);
  const knownBuildingsKey = useMemo(
    () =>
      buildingParts &&
      JSON.stringify(knownOf(JSON.parse(knownKey) as KnownPropView[], buildingParts)),
    [knownKey, buildingParts],
  );
  const buildings = useMemo<SideBuildings | null>(
    () =>
      drawnBuildings && knownBuildingsKey
        ? {
            placed: drawnBuildings.placed,
            fallen: knownFallen(drawnBuildings, JSON.parse(knownBuildingsKey) as KnownPropView[]),
          }
        : null,
    [drawnBuildings, knownBuildingsKey],
  );
  const buildingsFeed = useFeed(buildings);
  // The map's props drawn apart, fitted once for the battle; what the side
  // knows then hides those it replaced and fits only its own (`sideStructures`).
  const fittedMap = useMemo(
    () => props && fitMapProps(props.map, props.fit, (prop) => apart.includes(prop.kind)),
    [props, apart],
  );
  const sideProps = useMemo(() => {
    if (!props || !fittedMap || !drawnBuildings) return null;
    const part = drawnBuildings.partBuilding;
    // A moving wreck is drawn each frame rather than among static props.
    const moving = new Set(transitions.map((f) => f.cookOff.prop));
    const known = (JSON.parse(knownKey) as KnownPropView[]).filter(
      (k) => (k.authoredProp === null || !part.has(k.authoredProp)) && !moving.has(k.id),
    );
    return sideStructures(fittedMap, known, props.fit);
  }, [props, fittedMap, drawnBuildings, knownKey, transitions]);
  useEffect(() => {
    transitionFitting.current =
      props && appearances ? { fit: props.fit, installed: appearances } : null;
  }, [props, appearances]);
  const structures: SideStructures = sideProps ?? NO_STRUCTURES;
  const structuresFeed = useFeed<SideStructures>(structures);
  // What the camera keeps clear of: the ground, and every building part the
  // side knows stands (a fallen one's remains once it has seen the fall).
  // Renderer fog: the side's eyes at the published tick over the static
  // world, cut by the occluders it knows (rebuilt only when knowledge changes).
  // Its foliage is the side's: less the trees on ground it has seen cleared
  // (a lane knocked, a patch shelled), re-exported when that ground grows.
  const clearedCount = observation ? (sim.ground.current?.clearedCount ?? 0) : 0;
  const clearingEpoch = sim.ground.current?.epoch ?? 0;
  const foliage = useMemo(() => {
    const g = sim.ground.current;
    if (!world) return null;
    return clearedCount > 0 && g
      ? world.view.foliage_cleared(g.clearedRuns(), g.cols, g.cellM)
      : world.exports.foliage;
  }, [world, clearedCount, clearingEpoch, sim.ground]);
  const fogMap = useMemo(() => world && fogWorld(world.exports, rules.sensors), [world, rules]);
  const fogStatic = useMemo(
    () => fogMap && foliage && (foliage === fogMap.foliage ? fogMap : { ...fogMap, foliage }),
    [fogMap, foliage],
  );
  const staticOccluders = useMemo(
    () => world && mapOccluders(world.exports, world.layout),
    [world],
  );
  const occluders = useMemo(
    () =>
      staticOccluders
        ? knownOccluders(staticOccluders, JSON.parse(knownKey) as KnownPropView[])
        : [],
    [staticOccluders, knownKey],
  );
  const fog = useMemo<FogInput | null>(
    () =>
      fogStatic && observation
        ? { world: fogStatic, sight: { eyes: fogEyes(observation.own), occluders } }
        : null,
    [fogStatic, observation, occluders],
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
    if (!props || !buildingParts || !knownBuildingsKey) return null;
    const started = performance.now();
    const view = buildingObstacles(
      props.map,
      JSON.parse(knownBuildingsKey) as KnownPropView[],
      buildingParts,
      surfaceZ,
    );
    return { view, buildMs: performance.now() - started };
  }, [props, buildingParts, knownBuildingsKey, surfaceZ]);
  const cameraObstaclesFeed = useFeed(cameraObstacles?.view ?? null);

  // What the last frame drew: which unit or enemy each pick box is, and each
  // own unit's drawn (interpolated) position.
  const drawn = useRef<DrawnInstances>({ picks: [], owners: [], enemies: [] });
  const drawnAt = useRef(new Map<number, Readonly<Vec3>>());
  const drawnPoses = useRef<readonly Pose[]>([]);
  const drawnEnemyAt = useRef(new Map<number, Readonly<Vec3>>());
  const readouts = useRef<ReadoutLayerHandle>(null);
  // The last frame's clock and drawn motion, which sound hears at the camera.
  const heard = useRef<{ clock: number; motion: SoundMotion } | null>(null);
  const indoorUnits = useMemo(
    () =>
      new Set(
        (observation?.own ?? [])
          .filter((u) => u.garrison && u.garrison.phase !== "entering")
          .map((u) => u.id),
      ),
    [observation?.own],
  );
  // Occupants stay occluded indoors; their panel identifies the building.
  // The observing side's outdoor units are x-rayed where the world hides them: the
  // selection in its colour, so a selected unit behind a house still reads
  // as selected, the rest in the side's. Their visible parts are never tinted:
  // the selection's marker is on the ground.
  const xrayOf = useRef<XrayOf>(() => null);
  xrayOf.current = (unitSide, unit) =>
    !xray || unitSide !== side || indoorUnits.has(unit)
      ? null
      : control.selected.includes(unit)
        ? gameXray.selected
        : gameXray.own;

  // The models layer installs only what the battle draws as models.
  const modelAppearances = useMemo<InstalledAppearances | null>(
    () =>
      appearances && props && drawnBuildings
        ? mapAppearances(appearances, props.map, props.fit, drawnBuildings.placed, true)
        : null,
    [appearances, props, drawnBuildings],
  );

  // Soldiers: the observation, fed per soldier to the pose driver, drawn as
  // the appearance for their kind and side. A new side or catalog starts over.
  const posing = useMemo(() => {
    if (!appearances) return null;
    const catalog = new AppearanceCatalog(appearances, UNITS);
    const resolve: ResolveAppearance = (kind, s, id, slot, operatorMount, activeMount) =>
      catalog.resolve(kind, s, id, slot, operatorMount, activeMount);
    const muzzles = new DrawnMuzzles(appearances, resolve, UNITS);
    return {
      driver: createPoseDriver(rules, UNITS, appearances),
      feed: new ObservationFeed(side, UNITS),
      resolve,
      muzzles,
      source: drawnMuzzleSource(muzzles, side),
      models: [] as ModelInstance[],
      corpses: { version: -1, list: [] as CorpseInstance[], soldiers: [] as number[] },
    };
  }, [appearances, rules, side]);
  const ghostModel = useMemo<ModelInstance | null>(() => {
    if (!purchasing || !posing || !appearances) return null;
    const resolved = posing.resolve(purchasing, side, 0, 0);
    const bundle = resolved && appearances.appearances.get(resolved.appearance)?.bundle;
    if (!resolved || !bundle) return null;
    return {
      appearance: resolved.appearance,
      x: 0,
      y: 0,
      z: 0,
      yaw: 0,
      pose: restingModelPose(bundle),
      ghost: gameXray.selected,
    };
  }, [purchasing, posing, appearances, side]);
  const frameModels = useRef<ModelInstance[]>([]);
  // The trees the side knows have fallen, each from the start of the tick
  // it fell (the clock is ticks over tick_hz, as the effects' are). The
  // decoder keeps an unchanged list's reference, and so does this.
  const fallenBodies = observation?.fallenBodies;
  const felled = useMemo<readonly FelledTree[]>(
    () =>
      (fallenBodies ?? []).map((f) => ({
        prop: f.prop,
        toward: f.toward,
        fellAt: (f.tick - 1) / rules.tick_hz,
      })),
    [fallenBodies, rules.tick_hz],
  );
  const frame = useCallback(
    (now: number): ViewportFrame | null => {
      const interpolator = sim.interpolator.current;
      const time = interpolator?.time(now) ?? null;
      if (!interpolator || time === null || !observation) return null;
      const own = interpolator.sample(now);
      drawnPoses.current = own;
      const identified = interpolator.sampleIdentified(now);
      const d = sideInstances(own, identified, observation, rules.physics, UNITS);
      drawn.current = d;
      drawnAt.current = new Map(own.map((p) => [p.id, p.position]));
      drawnEnemyAt.current = new Map(identified.map((p) => [p.id, p.position]));
      drawnClock.current = time;
      drawnTick.current = observation.tick;
      const reveal = orderReveal.at(time, control.showOrders, observation.own);
      if (!sameReveal(reveal, revealedRef.current)) {
        revealedRef.current = reveal;
        setRevealed(reveal);
      }
      const ground = sim.ground.current;
      if (!posing) {
        effects.build(time, effectBatch);
        heard.current = { clock: time, motion: soundMotion(null, UNITS, side) };
        return { picks: d.picks, clock: time, effects: effectBatch, ground, felled };
      }
      const poses = posing.driver.update(posing.feed.frame(observation, own, identified, time));
      // Flashes sit on the muzzles as this frame draws them.
      posing.muzzles.update(poses);
      effects.build(time, effectBatch, posing.source);
      if (audio) {
        const reversing = new Set(observation.own.filter((u) => u.reversing).map((u) => u.id));
        const enemyReversing = new Set(
          observation.identified.filter((u) => u.reversing).map((u) => u.id),
        );
        heard.current = {
          clock: time,
          motion: soundMotion(poses, UNITS, side, reversing, enemyReversing),
        };
      }
      const posed = poseFrameInstances(posing.models, poses, posing.resolve, xrayOf.current);
      lastHulls.note(poses.vehicles, time);
      // Each cooking-off hull: whole until its ammunition goes, then its
      // moving wreck until a moment after it lies still, when the static
      // wreck takes over (the structures, rebuilt as the transition ends).
      const feel = gameEffects.cook_off;
      const done = transitions.filter((f) => time - f.hitAt > landedAfter(feel) + LANDED_HOLD_S);
      if (done.length) setTransitions((now) => now.filter((f) => !done.includes(f)));
      const models = transitions.length
        ? posed.concat(
            transitions.flatMap((f) => {
              if (!transitionHulls.current.has(f))
                transitionHulls.current.set(f, lastHulls.at(f.cookOff, time, posing.resolve));
              return cookOffModels(f, transitionHulls.current.get(f) ?? null, feel, time);
            }),
          )
        : posed;
      if (poses.corpsesVersion !== posing.corpses.version)
        posing.corpses = {
          version: poses.corpsesVersion,
          list: corpseInstances(poses, posing.resolve),
          soldiers: poses.corpses.map((c) => c.soldier),
        };
      const placement = purchaseGhost.current;
      const composed = frameModels.current;
      composed.length = 0;
      for (const model of models) composed.push(model);
      if (placement && ghostModel && inputEnabled) {
        ghostModel.x = placement.destination[0];
        ghostModel.y = placement.destination[1];
        ghostModel.yaw = placement.facing;
        ghostModel.z = surfaceZ(ghostModel.x, ghostModel.y);
        ghostModel.ghost = placement.valid === false ? PURCHASE_BLOCKED : gameXray.selected;
        composed.push(ghostModel);
      }
      return {
        picks: d.picks,
        models: composed,
        corpses: posing.corpses.list,
        felled,
        clock: time,
        effects: effectBatch,
        ground,
      };
    },
    [
      observation,
      felled,
      sim.interpolator,
      sim.ground,
      posing,
      rules,
      effects,
      effectBatch,
      audio,
      side,
      orderReveal,
      control.showOrders,
      transitions,
      lastHulls,
      ghostModel,
      inputEnabled,
      surfaceZ,
    ],
  );
  /** Sound for the last frame, heard from `camera`: call once a frame. */
  const hear = useCallback(
    (camera: Camera3DParams) => {
      const h = heard.current;
      if (audio && h) audio.update(h.clock, h.motion, camera);
    },
    [audio],
  );

  const semanticPick = useCallback(
    (pick: LabPick): PointerPick | null => {
      if (!world) return null;
      const pointer = pickToPointer(world, drawn.current, pick, observation?.contacts);
      const panel = readouts.current?.pick(pick.x, pick.y);
      if (!panel && pick.button === "left" && pointer.unit === null && pointer.enemy === null) {
        const ground = groundUnderRay(world.view, pick.ray);
        if (ground) {
          for (const u of observation?.own ?? []) {
            if (UNITS.hull(u.kind)) continue;
            const pose = orderView(u, true);
            const drawnPose = drawnPoses.current.find((p) => p.id === u.id);
            if (drawnPose) {
              pose.position = drawnPose.position;
              pose.members = drawnPose.members;
            }
            const circle = unitCircle(pose, gameOrderStyle);
            if (circle && circleContains(circle, [ground[0], ground[1]])) {
              pointer.unit = u.id;
              break;
            }
          }
        }
      }
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
        else if (inputEnabled) {
          if (pick.release && world) {
            const ground = groundUnderRay(world.view, pick.ray);
            const release = groundUnderRay(world.view, pick.release);
            if (ground && release) {
              purchasePlacement.at(
                [ground[0], ground[1]],
                sim.client,
                "placement",
                Math.atan2(release[1] - ground[1], release[0] - ground[0]),
              );
            }
          }
          void purchasePlacement.confirm(control.issue).then((accepted) => {
            if (accepted && !purchasePlacement.variant) {
              purchaseGhost.current = null;
              setPurchasing(null);
            }
          });
        }
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
    [semanticPick, control, world, sim.client, purchasePlacement, cancelPurchase, inputEnabled],
  );
  const eligibilityIdentity = JSON.stringify(
    (observation?.own ?? []).map((u) => [
      u.id,
      u.kind,
      u.members.length,
      u.garrison?.building,
      u.garrison?.phase,
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
  const onCursor = (pointer: ViewportPointer, camera: Camera3DParams): CursorAction | null => {
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
      const at = active && inputEnabled && groundUnderRay(world.view, pointer.ray!);
      const facingTo =
        active && inputEnabled && pointer.rightDragging && pointer.ray
          ? groundUnderRay(world.view, pointer.ray)
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
            reconcilePointerIntent(held.intent, observation),
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
    const queryIntent = waiting ? reconcilePointerIntent(released, observation) : intent;
    const marks = pointerPaint.resolvePreview(
      active ? previewForIntent(queryIntent) : null,
      observation?.own ?? [],
      active ? sim.client : null,
      `${observation?.tick ?? 0}:${control.acks[0]?.seq ?? 0}`,
      `${semanticRevision.current.version}:${held?.generation ?? (waiting ? pending.generation : 0)}`,
    );
    const showDestinations = queryIntent.kind === "move" || queryIntent.kind === "occupy_building";
    let preview = (held || waiting) && showDestinations ? marks : [];
    if (!held && accepted) {
      if ((observation?.tick ?? 0) >= accepted.applied_tick) pendingAction.current = null;
      else if (
        !accepted.error &&
        released &&
        (released.kind === "move" || released.kind === "occupy_building")
      )
        preview = pointerPaint.markers(
          accepted.placement?.destinations ?? accepted.building?.destinations ?? [],
          observation?.own ?? [],
          revealedRef.current,
        );
    }
    const ruler =
      active && control.showOrders
        ? rulerAt(pointer.ray, world!, control.selectedUnits, drawnAt.current, rules, surfaceZ)
        : null;
    pointerPaint.update(
      ruler,
      active ? preview : [],
      surfaceZ,
      metresPerPxAt(camera.distance, camera.fovY, window.innerHeight),
    );
    if (!active) return null;
    if (!held && pick?.unit != null) return "default";
    if (!held && accepted && released && (observation?.tick ?? 0) < accepted.applied_tick) {
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

  // The viewport's GPU allocation counter, once it is up.
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

  // Lab-only probes for the scene harness; rebuilt each render.
  const probes = {
    tick: () => sim.latest.current?.tick ?? 0,
    publicationBytes: () => sim.lastBytes.current,
    observation: () => sim.latest.current,
    /** The last drawn frame: the tick of the observation it presented, and
     *  its presentation clock in ticks. Null before the first frame. */
    presented: () =>
      drawnClock.current === null || drawnTick.current === null
        ? null
        : { tick: drawnTick.current, clock: drawnClock.current * rules.tick_hz },
    /** The soldiers the last drawn frame lays as static corpses. */
    lying: () => posing?.corpses.soldiers ?? [],
    digest: () => sim.digest.current,
    error: () => sim.error,
    status: () => sim.status,
    selected: () => control.selected,
    select: (ids: number[]) => control.setSelected(ids),
    pointerIntent: () => shownIntent.current,
    pointerResolution: () => ({ state: pointerPaint.state, building: pointerPaint.building }),
    movePreview: () => pointerPaint.preview,
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
      (props?.map ?? [])
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
      if (!drawnBuildings || !buildings || !knownBuildingsKey) return [];
      const state = new Map(buildings.fallen.map((f) => [f.building, f.state]));
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
    /** The boxes the fog is handed as what hides the ground behind them: the
     *  occluders the side knows stand. */
    fogOccluders: () => occluders,
    /** The side's known craters: marked cells, and the centre of the
     *  `binM`-square block holding the most (framing a shelled field). */
    craters: (binM = 16) => {
      const g = sim.ground.current;
      if (!g) return null;
      const per = Math.max(1, Math.round(binM / g.cellM));
      const bins = new Map<number, number>();
      let cells = 0;
      g.forEachMarked((i, j) => {
        if (g.cell(i, j).crater < 64) return;
        cells++;
        const key = Math.floor(j / per) * g.cols + Math.floor(i / per);
        bins.set(key, (bins.get(key) ?? 0) + 1);
      });
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
    effects: () => effects.stats(),
    /** The instances the effect pass drew last frame: shape (`SHAPE`), the
     *  point it sits on (a streak's tail), its size, and a glow's rays. */
    effectInstances: () =>
      Array.from({ length: effectBatch.count }, (_, i) => {
        const d = effectBatch.data.subarray(i * EFFECT_FLOATS, (i + 1) * EFFECT_FLOATS);
        return {
          shape: d[12],
          at: [d[0], d[1], d[2]],
          to: [d[4], d[5], d[6]],
          rgb: [d[8], d[9], d[10]],
          size: d[3],
          rays: d[6],
        };
      }),
    /** The lights the effects cast last frame (`light/castLights.ts`): where,
     *  how far, their colour × intensity, and what cast each. */
    castLights: () => offeredCastLights(effectBatch.lights),
    /** Every drawn model's muzzle sockets in the world, posed from the model
     *  instances as drawn (the workbench's socket gizmos), with each model's
     *  origin to select the firing body before its attachment is measured.
     *  Computed apart from the flashes' own `DrawnMuzzles`. */
    muzzleSockets: () =>
      (posing?.models ?? []).flatMap((m) => {
        const bundle = appearances?.appearances.get(m.appearance)?.bundle;
        if (!bundle) return [];
        const skeleton =
          bundle.kind === "skinned" ? (appearances?.skeletons.get(bundle.skeleton) ?? null) : null;
        const [c, s] = [Math.cos(m.yaw), Math.sin(m.yaw)];
        return posedSockets(bundle, skeleton, m.pose)
          .filter((k) => k.name.endsWith("muzzle"))
          .map(({ name, frame: f }) => ({
            appearance: m.appearance,
            origin: [m.x, m.y, m.z],
            name,
            at: [m.x + f[12] * c - f[13] * s, m.y + f[12] * s + f[13] * c, m.z + f[14]],
          }));
      }),
    /** The vehicles as last posed: appearance, placement and articulation. */
    vehicles: () =>
      (posing?.models ?? []).flatMap((m) =>
        m.pose.kind === "articulated"
          ? [
              {
                appearance: m.appearance,
                position: [m.x, m.y, m.z],
                yaw: m.yaw,
                tint: m.tint,
                articulation: { ...m.pose.articulation },
              },
            ]
          : [],
      ),
    /** Every body drawn as models, the static world's and those drawn from
     *  what the side knows: how many, how many of them a building draws (its
     *  parts), how many neither a model nor a building draws, and how many
     *  models draw the side's. */
    structureBodies: () => {
      if (!world || !appearances) return null;
      const statics = worldStructureBodies(
        world.exports,
        world.layout,
        apart,
        appearances,
        gameStandIns,
      );
      const drawnBodies = sideProps?.bodies() ?? [];
      const all: DrawnBody[] = [...statics, ...drawnBodies];
      const part = (b: DrawnBody) => drawnBy(world.layout, b.body.kind, "building");
      return {
        bodies: all.length,
        parts: all.filter(part).length,
        unmodelled: all.filter((b) => !part(b) && b.models.length === 0).length,
        sideModels: drawnBodies.reduce((n, b) => n + b.models.length, 0),
      };
    },
    /** The props drawn from what the side knows: appearance, state and placement. */
    structures: () =>
      drawnStructures(structures).map((m) => ({
        appearance: m.appearance,
        state: m.pose.kind === "static" ? m.pose.state : null,
        position: [m.x, m.y, m.z],
        yaw: m.yaw,
        scale: m.scale,
      })),
  };

  return {
    world,
    meshes,
    /** The props drawn from what the side knows (standing destroyable props
     *  when "apart", their remains, and wrecks), fed to the viewport. */
    structures: structuresFeed,
    /** The map's buildings, and those the side has seen
     *  fall, for the viewport. */
    buildingsFeed,
    /** What the camera keeps clear of, from what the side knows stands, for
     *  the viewport. */
    cameraObstaclesFeed,
    /** What renderer fog is drawn from; `fogFeed` carries it to the viewport. */
    fog,
    fogFeed,
    rules,
    contacts,
    sim,
    control,
    purchase: skirmish
      ? {
          faction: skirmish.factions[side === "blue" ? 0 : 1],
          cards: UNITS.cards,
          placing: purchasing,
          ghost: purchaseGhost,
          choose: choosePurchase,
          cancel: cancelPurchase,
          ready: () => void control.issue({ kind: "ready" }),
          cancelPending: (id: number) =>
            void control.issue({ kind: "cancel_pending", purchase: id }),
        }
      : null,
    /** Which own units' order marks show, at what opacity (`OrderReveal`):
     *  every unit's with Space held, an order's units' as it flashes. */
    revealed,
    pointerPaint,
    surfaceZ,
    /** The appearances the viewport's models layer installs. */
    appearances: modelAppearances,
    /** Every animation frame's drawn units and presentation clock, for the viewport. */
    frame,
    /** The battle's sound (null unless `sound`), and its per-frame listener. */
    audio,
    hear,
    drawnAt,
    drawnClock,
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
    onPick,
    onRightPress,
    onCursor,
    onBox,
    onReady,
    gpuAllocations,
    probes,
  };
}

export type BattleSession = ReturnType<typeof useBattleSession>;
