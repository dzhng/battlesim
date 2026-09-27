// One side's live battle session: the static world and its meshes, the worker
// authority, the player command path, the drawn units (interpolated; soldiers
// and vehicles posed by the pose driver as models, each picked by the
// simulation's box for its body), the props the side knows stand (fitted
// appearances), the pick and box-select adapters over what is drawn, and the
// base lab probes. The battle view and every lab that plays a battle
// share it; routes add only what they show.
import { useCallback, useEffect, useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { SoundMotion } from "@packages/battle-audio/src/soundFrame";
import type { GpuAllocationCounts } from "@packages/renderer-core/src/gpuAllocations";
import { apartKinds, buildWorldLayers } from "@packages/battle-renderer/src/worldMesh";
import {
  mapProps,
  PropAppearances,
  structureModels,
} from "@packages/battle-renderer/src/models/propAppearance";
import type { BodyRules } from "@packages/battle-renderer/src/picking";
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
import type { UnitKind } from "@packages/scene-assets/src/schema";
import {
  corpseInstances,
  poseFrameInstances,
  type CorpseInstance,
  type ModelInstance,
  type ResolveAppearance,
} from "@packages/battle-renderer/src/models/modelInstances";
import { useUnitControl } from "@web/battle/input/useUnitControl";
import type { KnownPropView, ObservationView } from "@web/battle/sim/observation";
import type { Order, SideName } from "@web/battle/sim/protocol";
import type { LabBox, LabPick, ViewportFrame, ViewportGpu } from "./LabViewport";
import { pickToPointer, sideInstances, type DrawnInstances } from "./sideInstances";
import { createPoseDriver, ObservationFeed, type PoseRules } from "./poseFeed";
import { useSimSession, type ScriptedSim } from "./useSimSession";
import {
  createEffectFrame,
  effectPublication,
  villageEffects,
  type EffectRules,
} from "./effectFeed";
import { createEffectBatch } from "@packages/battle-renderer/src/effects/effectFrame";
import { useStaticWorld } from "./useStaticWorld";
import { createBattleAudio, soundMotion } from "./soundFeed";

type P3 = readonly [number, number, number];

/** The unit kinds the battle draws as posed models. */
const UNITS: readonly UnitKind[] = ["rifle", "recon", "at", "tank", "supply", "jeep"];

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

/** The rule values the scenario runs under (only what views read). */
export interface ScenarioRules extends PoseRules, EffectRules {
  tick_hz: number;
  mounts: PoseRules["mounts"] & EffectRules["mounts"];
  physics: PoseRules["physics"] & BodyRules & EffectRules["physics"];
  service: { radius_m: number; deploy_and_pack_s: number; stock: number };
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
  const effects = useMemo(() => createEffectFrame(rules, rules.tick_hz), [rules]);
  const effectBatch = useMemo(() => createEffectBatch(villageEffects.capacity), []);
  // Sound reads the same publication, plus the side's hearing cues.
  const audio = useMemo(
    () => (sound ? createBattleAudio(rules, rules.tick_hz) : null),
    [sound, rules],
  );
  useEffect(() => () => audio?.dispose(), [audio]);
  const noteDecoded = useCallback(
    (o: ObservationView) => {
      const pub = effectPublication(o, rules);
      effects.note(pub);
      audio?.note({ effects: pub, audible: o.audible });
      onDecoded?.(o);
    },
    [effects, audio, rules, onDecoded],
  );
  const sim = useSimSession({ scenario, seed, onDecoded: noteDecoded, replay, scripted });
  const { observation } = sim;
  const control = useUnitControl(replay || scripted ? null : sim.client, observation);

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
        ? { map: mapProps(world.exports, world.layout), fit: new PropAppearances(appearances) }
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

  const surfaceZ = useCallback(
    (x: number, y: number) => world?.view.surface_at(x, y)[0] ?? 0,
    [world],
  );

  // What the last frame drew: which unit or enemy each pick box is, and each
  // own unit's drawn (interpolated) position.
  const drawn = useRef<DrawnInstances>({ picks: [], owners: [], enemies: [] });
  const drawnAt = useRef(new Map<number, P3>());
  // The last frame's clock and drawn motion, which sound hears at the camera.
  const heard = useRef<{ clock: number; motion: SoundMotion } | null>(null);
  const selectedRef = useRef(control.selected);
  selectedRef.current = control.selected;

  // The models layer installs only what the battle draws as models: its
  // soldiers and vehicles, the appearance each of the map's props takes, and
  // every wreck and ruin a battle can leave. Trees, hedgerows and grass are
  // the scenery layer's and the grass pass's, which hold their own buffers.
  const modelAppearances = useMemo<InstalledAppearances | null>(() => {
    if (!appearances || !props) return null;
    const drawn = props.fit.drawnFor(props.map.filter((p) => p.kind !== "trunk"));
    return {
      ...appearances,
      appearances: new Map(
        [...appearances.appearances].filter(
          ([name, a]) => UNITS.includes(a.unit) || drawn.has(name),
        ),
      ),
    };
  }, [appearances, props]);

  // Soldiers: the observation, fed per soldier to the pose driver, drawn as
  // the appearance for their kind and side. A new side or catalog starts over.
  const posing = useMemo(() => {
    if (!appearances) return null;
    const catalog = new AppearanceCatalog(appearances);
    const resolve: ResolveAppearance = (kind, s, id) => catalog.resolve(kind, s, id);
    return {
      driver: createPoseDriver(rules, appearances),
      feed: new ObservationFeed(side),
      resolve,
      models: [] as ModelInstance[],
      corpses: { version: -1, list: [] as CorpseInstance[] },
    };
  }, [appearances, rules, side]);
  const frame = useCallback(
    (now: number): ViewportFrame | null => {
      const interpolator = sim.interpolator.current;
      const time = interpolator?.time(now) ?? null;
      if (!interpolator || time === null || !observation) return null;
      const own = interpolator.sample(now);
      const identified = interpolator.sampleIdentified(now);
      const d = sideInstances(own, identified, observation, rules.physics);
      drawn.current = d;
      drawnAt.current = new Map(own.map((p) => [p.id, p.position]));
      effects.build(time, effectBatch);
      const ground = sim.ground.current;
      if (!posing) {
        heard.current = { clock: time, motion: soundMotion(null, side) };
        return { picks: d.picks, clock: time, effects: effectBatch, ground };
      }
      const poses = posing.driver.update(posing.feed.frame(observation, own, identified, time));
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
      const models = poseFrameInstances(
        posing.models,
        poses,
        posing.resolve,
        new Set(selectedRef.current),
      );
      if (poses.corpsesVersion !== posing.corpses.version)
        posing.corpses = {
          version: poses.corpsesVersion,
          list: corpseInstances(poses, posing.resolve),
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
    [observation, sim.interpolator, sim.ground, posing, rules, effects, effectBatch, audio, side],
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
      if (world) control.onPointer(pickToPointer(world, drawn.current, pick));
    },
    [world, control],
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
    /** Sound: voices, budget, holds and counts; null before audio starts. */
    sound: () => audio?.stats() ?? null,
    /** Combat effects: running, drawn last frame, dropped, and the last tick noted. */
    effects: () => effects.stats(),
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
    /** What renderer fog is drawn from, for the viewport's `fog`. */
    fog,
    rules,
    sim,
    control,
    surfaceZ,
    /** The appearances the viewport's models layer installs. */
    appearances: modelAppearances,
    /** Every animation frame's drawn units and presentation clock, for the viewport. */
    frame,
    /** The battle's sound (null unless `sound`), and its per-frame listener. */
    audio,
    hear,
    drawnAt,
    onPick,
    onBox,
    onReady,
    gpuAllocations,
    probes,
  };
}

export type BattleSession = ReturnType<typeof useBattleSession>;
