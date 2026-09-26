// One side's live battle session: the static world and its meshes, the worker
// authority, the player command path, the drawn units (interpolated; soldiers
// and vehicles posed by the pose driver as models, each picked by the
// simulation's box for its body), the props the side knows stand (fitted
// appearances), the pick and box-select adapters over what is drawn, and the
// base lab probes. The battle view and every lab that plays a battle
// share it; routes add only what they show.
import { useCallback, useMemo, useRef } from "react";
import type { GpuAllocationCounts } from "@packages/renderer-core/src/gpuAllocations";
import { buildWorldLayers, FALLIBLE_KINDS } from "@packages/battle-renderer/src/worldMesh";
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
import { useStaticWorld } from "./useStaticWorld";

type P3 = readonly [number, number, number];

/** The unit kinds the battle draws as posed models. */
const UNITS: readonly UnitKind[] = ["rifle", "recon", "at", "tank", "supply"];

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
  /** "apart": buildings are drawn apart from the world, so one seen to fall
   *  leaves the map and its known ruin stands in its place. */
  buildings?: "apart";
}

/** The rule values the scenario runs under (only what views read). */
export interface ScenarioRules extends PoseRules {
  service: { radius_m: number; deploy_and_pack_s: number; stock: number };
  sensors: FogSensorRules;
  physics: PoseRules["physics"] & BodyRules;
}

export function useBattleSession({
  map,
  scenario,
  seed,
  onDecoded,
  replay,
  scripted,
  side = "blue",
  buildings,
}: BattleSessionOptions) {
  const world = useStaticWorld(map);
  const rules = useMemo(() => (JSON.parse(scenario) as { rules: ScenarioRules }).rules, [scenario]);
  const sim = useSimSession({ scenario, seed, onDecoded, replay, scripted });
  const { observation } = sim;
  const control = useUnitControl(replay || scripted ? null : sim.client, observation);

  const appearances = useVillageAppearances();
  const meshes = useMemo(
    () =>
      world &&
      appearances &&
      buildWorldLayers(
        world.exports,
        world.layout,
        villageBiome,
        "surface",
        buildings,
        appearances,
      ),
    [world, buildings, appearances],
  );
  // What the side knows stands, rebuilt only when knowledge changes: the
  // props it has learned (ruins, wrecks), and ("apart") the buildings it has
  // not seen fall, each a fitted appearance. A known ruin replaces its
  // building in the same list; an unseen collapse leaves the building standing.
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
        ? structureModels(
            props.map,
            JSON.parse(knownKey) as KnownPropView[],
            props.fit,
            (prop) => buildings === "apart" && FALLIBLE_KINDS.includes(prop.kind),
          )
        : [],
    [props, knownKey, buildings],
  );
  // Renderer fog: the side's eyes at the published tick over the static
  // world, cut by the occluders it knows (rebuilt only when knowledge changes).
  const fogStatic = useMemo(() => world && fogWorld(world.exports, rules.sensors), [world, rules]);
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
    const resolve: ResolveAppearance = (kind, s) => catalog.resolve(kind, s);
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
      if (!posing) return { picks: d.picks, clock: time };
      const poses = posing.driver.update(posing.feed.frame(observation, own, identified, time));
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
      };
    },
    [observation, sim.interpolator, posing, rules],
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
    acks: () => control.acks,
    command: (order: Order, queued = false) => control.issue(order, queued),
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
    reset: () => sim.reset(),
    surfaceZ,
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
    /** The props drawn from what the side knows (standing buildings when
     *  "apart", ruins and wrecks), for the viewport's `structures`. */
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
    drawnAt,
    onPick,
    onBox,
    onReady,
    gpuAllocations,
    probes,
  };
}

export type BattleSession = ReturnType<typeof useBattleSession>;
