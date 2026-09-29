// One side's live battle session: the static world and its meshes, the worker
// authority, the player command path, the drawn units (interpolated; soldiers
// and vehicles posed by the pose driver as models, each picked by the
// simulation's box for its body), the props the side knows stand (fitted
// appearances), the pick and box-select adapters over what is drawn, and the
// base lab probes. The battle view and every lab that plays a battle
// share it; routes add only what they show.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Project, ReadoutLayerHandle } from "@web/battle/present/readouts";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { SoundMotion } from "@packages/battle-audio/src/soundFrame";
import type { GpuAllocationCounts } from "@packages/renderer-core/src/gpuAllocations";
import { apartKinds, buildWorldLayers } from "@packages/battle-renderer/src/worldMesh";
import {
  mapProps,
  PropAppearances,
  structureModels,
} from "@packages/battle-renderer/src/models/propAppearance";
import type { SoldierBody } from "@packages/battle-renderer/src/picking";
import {
  fogEyes,
  fogWorld,
  knownOccluders,
  type FogInput,
  type FogSensorRules,
} from "@packages/battle-renderer/src/frame/fogInputs";
import { villageBiome } from "./villageBiome";
import { useVillageAppearances } from "./villageAppearances";
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import {
  corpseInstances,
  poseFrameInstances,
  type CorpseInstance,
  type ModelInstance,
  type ResolveAppearance,
  type XrayOf,
} from "@packages/battle-renderer/src/models/modelInstances";
import { villageOrderFlash, villageXray } from "./villageOverlay";
import {
  NOTHING_REVEALED,
  OrderReveal,
  sameReveal,
  type RevealedOrders,
} from "@web/battle/present/orderReveal";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import type { ReadoutRules } from "@web/battle/present/readouts";
import type { RulerRules } from "@web/battle/present/rangeRuler";
import type { KnownPropView, ObservationView } from "@web/battle/sim/observation";
import type { Order, SideName } from "@web/battle/sim/protocol";
import type { LabBox, LabPick, ViewportFrame, ViewportGpu } from "./LabViewport";
import { pickToPointer, sideInstances, type DrawnInstances } from "./sideInstances";
import { createPoseDriver, ObservationFeed, type PoseRules } from "./poseFeed";
import { DrawnMuzzles } from "@packages/battle-renderer/src/models/drawnMuzzles";
import { useSimSession, type ScriptedSim } from "./useSimSession";
import {
  createEffectFrame,
  drawnMuzzleSource,
  effectPublication,
  villageEffects,
} from "./effectFeed";
import {
  createEffectBatch,
  EFFECT_FLOATS,
} from "@packages/battle-renderer/src/effects/effectFrame";
import { CAST_LIGHT_FLOATS } from "@packages/battle-renderer/src/light/castLights";
import { useStaticWorld } from "./useStaticWorld";
import { createBattleAudio, soundMotion } from "./soundFeed";
import { useFeed } from "./feed";
import { posedSockets } from "./workbench/benchWorld";
import type { Vec3 } from "math";

export interface BattleSessionOptions {
  /** The map the scenario runs on (the scenario's own `map`). */
  map: unknown;
  /** The scenario JSON the authority runs. */
  scenario: string;
  seed: number;
  /** Called for every decoded frame, before its credit returns. */
  onDecoded?: (o: ObservationView) => void;
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
}

/** The rule values the scenario runs under (only what views read). Its
 *  units are the shipped catalog's (`UNITS`), as every lab scenario's are. */
export interface ScenarioRules extends PoseRules, ReadoutRules, RulerRules {
  tick_hz: number;
  weapons: ReadoutRules["weapons"] & RulerRules["weapons"];
  physics: SoldierBody & RulerRules["physics"];
  service: { radius_m: number };
  sensors: FogSensorRules;
}

export function useBattleSession({
  map,
  scenario,
  seed,
  onDecoded,
  replay,
  scripted,
  side = "blue",
  destroyable,
  sound = false,
}: BattleSessionOptions) {
  const world = useStaticWorld(map);
  const rules = useMemo(() => (JSON.parse(scenario) as { rules: ScenarioRules }).rules, [scenario]);
  // Combat effects: every decoded publication noted (the frame dedupes),
  // drawn at each animation frame's presentation clock.
  const effects = useMemo(() => createEffectFrame(rules.tick_hz), [rules.tick_hz]);
  const effectBatch = useMemo(() => createEffectBatch(villageEffects.capacity), []);
  // Sound reads the same publication, plus the side's hearing cues.
  const audio = useMemo(
    () => (sound ? createBattleAudio(rules.tick_hz) : null),
    [sound, rules.tick_hz],
  );
  useEffect(() => () => audio?.dispose(), [audio]);
  const noteDecoded = useCallback(
    (o: ObservationView) => {
      const pub = effectPublication(o, side, UNITS);
      effects.note(pub);
      audio?.note({ effects: pub, audible: o.audible });
      onDecoded?.(o);
    },
    [effects, audio, side, onDecoded],
  );
  const sim = useSimSession({ scenario, seed, onDecoded: noteDecoded, replay, scripted });
  const { observation } = sim;
  // The last drawn frame's presentation clock: the callouts' nudges ease on
  // it, and an order's flash starts at it.
  const drawnClock = useRef<number | null>(null);
  // The tick of the observation the last drawn frame presented: React state,
  // so it can lag the clock, which follows each publication as it arrives.
  const drawnTick = useRef<number | null>(null);
  // Which units' order marks show (Space, or an order's flash), refreshed
  // each frame and kept as state only when it changes.
  const orderReveal = useMemo(() => new OrderReveal(villageOrderFlash), []);
  const [revealed, setRevealed] = useState<RevealedOrders>(NOTHING_REVEALED);
  const revealedRef = useRef(revealed);
  const noteOrder = useCallback(
    (order: Order) => {
      if (drawnClock.current !== null) orderReveal.noteOrder(order, drawnClock.current);
    },
    [orderReveal],
  );
  const control = useUnitControl(replay || scripted ? null : sim.client, observation, noteOrder);
  // A new battle carries no flash over.
  useEffect(() => orderReveal.clear(), [sim.client, orderReveal]);

  const appearances = useVillageAppearances();
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
      buildWorldLayers(world.exports, world.layout, villageBiome, "surface", apart, appearances),
    [world, apart, appearances],
  );
  // What the side knows stands, rebuilt only when knowledge changes: the
  // props it has learned (ruins, wrecks, shoved bodies where it last saw
  // them), and the map's props drawn apart that it has not seen fall or move,
  // each a fitted appearance. A known ruin replaces its building in the same
  // list, a known shoved body its own map pose; an unseen collapse or shove
  // leaves the map's prop standing.
  const knownKey = JSON.stringify(observation?.knownProps ?? []);
  const props = useMemo(
    () =>
      world && appearances
        ? {
            map: mapProps(world.exports, world.layout),
            fit: new PropAppearances(appearances, world.layout),
          }
        : null,
    [world, appearances],
  );
  const structures = useMemo(
    () =>
      props
        ? structureModels(props.map, JSON.parse(knownKey) as KnownPropView[], props.fit, (prop) =>
            apart.includes(prop.kind),
          )
        : [],
    [props, knownKey, apart],
  );
  // Renderer fog: the side's eyes at the published tick over the static
  // world, cut by the occluders it knows (rebuilt only when knowledge changes).
  // Its foliage is the side's: less the trees on ground it has seen cleared
  // (a lane knocked, a patch shelled), re-exported when that ground grows.
  const clearedCount = observation ? (sim.ground.current?.clearedCount ?? 0) : 0;
  const foliage = useMemo(() => {
    const g = sim.ground.current;
    if (!world) return null;
    return clearedCount > 0 && g
      ? world.view.foliage_cleared(g.cleared, g.cols, g.cellM)
      : world.exports.foliage;
  }, [world, clearedCount, sim.ground]);
  const fogMap = useMemo(() => world && fogWorld(world.exports, rules.sensors), [world, rules]);
  const fogStatic = useMemo(
    () => fogMap && foliage && (foliage === fogMap.foliage ? fogMap : { ...fogMap, foliage }),
    [fogMap, foliage],
  );
  const occluders = useMemo(
    () =>
      world
        ? knownOccluders(world.exports, world.layout, JSON.parse(knownKey) as KnownPropView[])
        : [],
    [world, knownKey],
  );
  const fog = useMemo<FogInput | null>(
    () =>
      fogStatic && observation
        ? { world: fogStatic, sight: { eyes: fogEyes(observation.own), occluders } }
        : null,
    [fogStatic, observation, occluders],
  );

  const fogFeed = useFeed(fog);

  const surfaceZ = useCallback(
    (x: number, y: number) => world?.view.surface_at(x, y)[0] ?? 0,
    [world],
  );

  // What the last frame drew: which unit or enemy each pick box is, and each
  // own unit's drawn (interpolated) position.
  const drawn = useRef<DrawnInstances>({ picks: [], owners: [], enemies: [] });
  const drawnAt = useRef(new Map<number, Readonly<Vec3>>());
  const drawnEnemyAt = useRef(new Map<number, Readonly<Vec3>>());
  const readouts = useRef<ReadoutLayerHandle>(null);
  // The last frame's clock and drawn motion, which sound hears at the camera.
  const heard = useRef<{ clock: number; motion: SoundMotion } | null>(null);
  // The observing side's units are x-rayed where the world hides them: the
  // selection in its colour, so a selected unit behind a house still reads
  // as selected, the rest in the side's. Their visible parts are never tinted:
  // the selection's marker is on the ground.
  const xrayOf = useRef<XrayOf>(() => null);
  xrayOf.current = (unitSide, unit) =>
    unitSide !== side
      ? null
      : control.selected.includes(unit)
        ? villageXray.selected
        : villageXray.own;

  // The models layer installs only what the battle draws as models: its
  // soldiers and vehicles, the appearance each of the map's props takes, and
  // every wreck and ruin a battle can leave. Trees, hedgerows and grass are
  // the scenery layer's and the grass pass's, which hold their own buffers.
  const modelAppearances = useMemo<InstalledAppearances | null>(() => {
    if (!appearances || !props) return null;
    const drawn = props.fit.drawnFor(props.map.filter((p) => !props.fit.drawsTree(p.kind)));
    return {
      ...appearances,
      appearances: new Map(
        [...appearances.appearances].filter(
          ([name, a]) => a.unit === "soldier" || a.unit === "vehicle" || drawn.has(name),
        ),
      ),
    };
  }, [appearances, props]);

  // Soldiers: the observation, fed per soldier to the pose driver, drawn as
  // the appearance for their kind and side. A new side or catalog starts over.
  const posing = useMemo(() => {
    if (!appearances) return null;
    const catalog = new AppearanceCatalog(appearances, UNITS);
    const resolve: ResolveAppearance = (kind, s, id, slot) => catalog.resolve(kind, s, id, slot);
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
  const frame = useCallback(
    (now: number): ViewportFrame | null => {
      const interpolator = sim.interpolator.current;
      const time = interpolator?.time(now) ?? null;
      if (!interpolator || time === null || !observation) return null;
      const own = interpolator.sample(now);
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
        heard.current = { clock: time, motion: soundMotion(null, side) };
        return { picks: d.picks, clock: time, effects: effectBatch, ground };
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
          motion: soundMotion(poses, side, reversing, enemyReversing),
        };
      }
      const models = poseFrameInstances(posing.models, poses, posing.resolve, xrayOf.current);
      if (poses.corpsesVersion !== posing.corpses.version)
        posing.corpses = {
          version: poses.corpsesVersion,
          list: corpseInstances(poses, posing.resolve),
          soldiers: poses.corpses.map((c) => c.soldier),
        };
      return {
        picks: d.picks,
        models,
        corpses: posing.corpses.list,
        clock: time,
        effects: effectBatch,
        ground,
      };
    },
    [
      observation,
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

  const onPick = useCallback(
    (pick: LabPick) => {
      if (world)
        control.onPointer(pickToPointer(world, drawn.current, pick, observation?.contacts));
    },
    [world, control, observation],
  );
  /** Drag-select own units whose drawn position falls in the rectangle. */
  const onBox = useCallback(
    (box: LabBox) =>
      control.selectInRect((u) => {
        const at = drawnAt.current.get(u.id) ?? u.position;
        const p = box.project(at[0], at[1], at[2] + 1);
        return !!p && p[0] >= box.x0 && p[0] <= box.x1 && p[1] >= box.y0 && p[1] <= box.y1;
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
    digest: (tick: number) => sim.digests.current.get(tick),
    error: () => sim.error,
    status: () => sim.status,
    selected: () => control.selected,
    select: (ids: number[]) => control.setSelected(ids),
    /** Space held: the order overlay shows every own unit (D2+). */
    showOrders: () => control.showOrders,
    acks: () => control.acks,
    command: (order: Order, queued = false) => control.issue(order, queued),
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
    reset: () => sim.reset(),
    surfaceZ,
    /** The side's known craters: marked cells, and the centre of the
     *  `binM`-square block holding the most (framing a shelled field). */
    craters: (binM = 16) => {
      const g = sim.ground.current;
      if (!g) return null;
      const per = Math.max(1, Math.round(binM / g.cellM));
      const bins = new Map<number, number>();
      let cells = 0;
      for (let j = 0; j < g.rows; j++)
        for (let i = 0; i < g.cols; i++) {
          if (g.marks[(j * g.cols + i) * 4] < 64) continue;
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
    effects: () => effects.stats(),
    /** The instances the effect pass drew last frame: shape (`SHAPE`), the
     *  point it sits on (a streak's tail), its size, and a glow's rays. */
    effectInstances: () =>
      Array.from({ length: effectBatch.count }, (_, i) => {
        const d = effectBatch.data.subarray(i * EFFECT_FLOATS, (i + 1) * EFFECT_FLOATS);
        return { shape: d[12], at: [d[0], d[1], d[2]], size: d[3], rays: d[6] };
      }),
    /** The lights the effects cast last frame (`light/castLights.ts`): where,
     *  how far, their colour × intensity, and what cast each. */
    castLights: () =>
      Array.from({ length: effectBatch.lights.count }, (_, i) => {
        const d = effectBatch.lights.data.subarray(
          i * CAST_LIGHT_FLOATS,
          (i + 1) * CAST_LIGHT_FLOATS,
        );
        return {
          at: [d[0], d[1], d[2]],
          radius: d[3],
          rgb: [d[4], d[5], d[6]],
          cause: effectBatch.lights.causes[i],
        };
      }),
    /** Every drawn model's muzzle sockets in the world, posed from the model
     *  instances as drawn (the workbench's socket gizmos): what a flash must
     *  sit on, measured apart from the flashes' own `DrawnMuzzles`. */
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
    /** The props drawn from what the side knows: appearance, state and placement. */
    structures: () =>
      structures.map((m) => ({
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
     *  when "apart", their remains, and wrecks), for the viewport's `structures`. */
    structures,
    /** What renderer fog is drawn from; `fogFeed` carries it to the viewport. */
    fog,
    fogFeed,
    rules,
    sim,
    control,
    /** Which own units' order marks show, at what opacity (`OrderReveal`):
     *  every unit's with Space held, an order's units' as it flashes. */
    revealed,
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
    placePanels: (project: Project, distance: number) =>
      readouts.current?.place(
        project,
        distance,
        { own: drawnAt.current, enemies: drawnEnemyAt.current, ground: surfaceZ },
        drawnClock.current,
      ),
    onPick,
    onBox,
    onReady,
    gpuAllocations,
    probes,
  };
}

export type BattleSession = ReturnType<typeof useBattleSession>;
