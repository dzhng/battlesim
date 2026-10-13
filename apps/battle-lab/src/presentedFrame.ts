// What the page reads of a presented frame (`crates/presentation`'s
// `Frame`, through `presentation.ts`) besides drawing it: the counts a
// capture's plates record. And what the page hands the presentation once
// per battle (`load_world`'s input).
import { Counts, EFFECT_WORDS } from "@packages/battle-renderer/src/records.gen";
import { MIN_TARGET_PX } from "@packages/battle-renderer/src/frame/battleFrame";
import type { PresentedCamera, PresentedFrame } from "@web/battle/sim/presentation";
import { liveCamera, type ViewportCamera } from "@packages/renderer-core/src/viewportCamera";
import { eyePosition } from "@packages/renderer-core/src/camera3d";
import { vec3, type Quat, type Vec3 } from "math";
import type { SideName } from "@web/battle/sim/protocol";

/** What a frame poses, by kind: living and dying soldiers, ground
 *  vehicles, aircraft (flying or falling), lying corpses and corpses
 *  sinking away (each with an appearance). */
export interface ModelCounts {
  soldiers: number;
  vehicles: number;
  aircraft: number;
  corpses: number;
  fading: number;
}

/** Each effect kind (`Counts.running`'s order) and instance shape (`Effect.misc.x`). */
export const EFFECT_KINDS = [
  "tracer",
  "flash",
  "impact",
  "sparks",
  "blast",
  "puff",
  "flame",
  "trail",
] as const;
export const SHAPE_NAMES = ["streak", "glow", "flipbook", "rotor"] as const;

/** What a frame drew of the effects: running by kind, instances by shape,
 *  and the lights they offered. */
export interface EffectCounts {
  running: Record<(typeof EFFECT_KINDS)[number], number>;
  instances: Record<(typeof SHAPE_NAMES)[number], number>;
  lights: number;
}

export const emptyModelCounts = (): ModelCounts => ({
  soldiers: 0,
  vehicles: 0,
  aircraft: 0,
  corpses: 0,
  fading: 0,
});

export const emptyEffectCounts = (): EffectCounts => ({
  running: Object.fromEntries(EFFECT_KINDS.map((k) => [k, 0])) as EffectCounts["running"],
  instances: Object.fromEntries(SHAPE_NAMES.map((k) => [k, 0])) as EffectCounts["instances"],
  lights: 0,
});

/** The presentation's camera for the viewport's orbit camera: what the
 *  frame's view, camera uniform and cascades are made for, drawn into
 *  targets the battle frame's size. */
export function presentedCamera(viewport: ViewportCamera): PresentedCamera {
  const camera = liveCamera(viewport);
  const s = Math.sin(camera.pitch);
  const px = (v: number) => Math.max(MIN_TARGET_PX, Math.floor(v));
  return {
    eye: eyePosition(vec3.create(), camera),
    target: camera.target,
    up: [-s * Math.cos(camera.yaw), -s * Math.sin(camera.yaw), Math.cos(camera.pitch)],
    fovY: camera.fovY,
    near: camera.near,
    width: px(viewport.width),
    height: px(viewport.height),
  };
}

/** The frame's counts (`Frame.counts`, and its effects by shape). */
export function frameCounts(frame: PresentedFrame): { models: ModelCounts; effects: EffectCounts } {
  const c = frame.counts.u32;
  const effects = emptyEffectCounts();
  EFFECT_KINDS.forEach((k, i) => (effects.running[k] = c[Counts.running + i] ?? 0));
  effects.lights = c[Counts.lights] ?? 0;
  const shapes = frame.effects.f32;
  for (let i = 0; i < frame.effects.count; i++)
    effects.instances[SHAPE_NAMES[shapes[i * EFFECT_WORDS + 12]]]++;
  return {
    models: {
      soldiers: c[Counts.soldiers] ?? 0,
      vehicles: c[Counts.vehicles] ?? 0,
      aircraft: c[Counts.aircraft] ?? 0,
      corpses: c[Counts.corpses] ?? 0,
      fading: c[Counts.fading] ?? 0,
    },
    effects,
  };
}

/** `load_world`'s input: the units and effects sections (`describeUnits`,
 *  `describeEffects`) with each clip table's floats apart, and the facts the
 *  presentation reads of the battle and the page. */
export function presentationWorld(input: {
  units: DrawnUnits;
  effects: EffectsSection;
  side: SideName;
  tickHz: number;
  sun: { azimuth: number; elevation: number };
  soldier: { soldier_radius_m: number; soldier_height_m: number };
  xray: { own: readonly number[]; selected: readonly number[] };
  blocked: readonly number[];
  cruiseAglM: number;
}): {
  world: object;
  tables: Map<string, Float32Array>;
  units: DrawnUnits;
  effects: EffectsSection;
} {
  const { tables, ...units } = input.units;
  return {
    units: input.units,
    effects: input.effects,
    world: {
      battle: {
        units,
        effects: input.effects,
        side: input.side,
        tick_hz: input.tickHz,
        soldier: {
          radius_m: input.soldier.soldier_radius_m,
          height_m: input.soldier.soldier_height_m,
        },
        xray: input.xray,
        blocked: input.blocked,
        cruise_agl_m: input.cruiseAglM,
      },
      sun_azimuth: input.sun.azimuth,
      sun_elevation: input.sun.elevation,
    },
    tables: new Map(
      tables.map(({ name, base64 }) => {
        const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
        return [name, new Float32Array(bytes.buffer)];
      }),
    ),
  };
}

/** A node's or joint's local transform (`scene-assets` `Trs`). */
export interface TrsRecord {
  t: Vec3;
  r: Quat;
  s: Vec3;
}

/** A clip at a phase (`PoseRef`). */
export interface PoseRecord {
  clip: string;
  phase: number;
}

/** A unit type as the pose driver and the appearance catalog read it. */
export interface UnitTypeRecord {
  /** Half the distance between its tracks or wheel rows (the pose driver's); null for a squad. */
  hull: { halfTrack: number } | null;
  /** Its vehicle class (`vehicleClass`), the sound row it is heard as; null for a squad. */
  vehicleClass: string | null;
  /** It flies (`airborne`). */
  airborne: boolean;
  /** How its model draws each mount, in mount order (`mountRoles`). */
  roles: string[];
  /** Per mount: its first weapon (the round a launch is matched by), the mount
   *  it rides (`on`, as an index; null: the hull), and the hold an operator
   *  takes with it active (`operator_appearance.active_pose`). */
  mounts: { weapon: string; on: number | null; activePose: PoseRecord | null }[];
  /** Per side, the installed appearances `AppearanceCatalog.resolve` picks
   *  from, each in the side's faction's look: a hull's model, each slot's
   *  soldier set, and each mount's operator's sets (null: no operator kit). */
  draws: Record<"blue" | "red", UnitDraws>;
}

export interface UnitDraws {
  hull: string[];
  slots: string[][];
  operated: ({ active: string[]; carried: string[] } | null)[];
}

/** A clip of a dense clip table (`TableClip`): its first float. */
export interface ClipRecord {
  offset: number;
  frames: number;
  duration: number;
  rate: number;
  loop: boolean;
  stride: number | null;
}

/** An articulated bundle's rig (`articulationRig`), its node indices into `nodes`. */
export interface RigRecord {
  turret: number;
  gun: number;
  hmg: number;
  hmgGun: number;
  pitch: { gun: [number, number]; hmg: [number, number] };
  /** How far the gun runs back, metres; null: unbounded. */
  stroke: number | null;
  wheels: { node: number; radius: number; left: boolean }[];
  tracks: { node: number; left: boolean; linkPitch: number }[];
  /** Each rotor, as the effects draw its blur (`RotorShape`). */
  rotors: {
    node: number;
    radius: number;
    blades: number;
    chord: number;
    rest: number;
    mast_below: number;
    mast_above: number;
    mast_radius: number;
  }[];
  deploy: { node: number; start: number; end: number; move: Vec3; turn: Vec3 }[];
  /** Each articulated mount's muzzle node (`MOUNT_NODES`), by role, where
   *  the model has it: a flash sits on it. */
  muzzles: Record<string, number>;
}

/** A body's weapon `muzzle` socket: the joint it hangs from and its offset
 *  in that joint's space; null where it has none. */
export type AppearanceMuzzle = { joint: number; offset: TrsRecord } | null;

export type AppearanceRecord =
  | {
      kind: "skinned";
      /** In the skeleton clips' joint order, parents first. */
      joints: { name: string; parent: number; bind: TrsRecord }[];
      /** The clip table (`tables`) its clips are sampled from. */
      table: string;
      clips: Record<string, ClipRecord>;
      farPose: PoseRecord;
      /** Each way the body lies dead (`INFANTRY_CORPSES`), in that order. */
      corpsePoses: PoseRecord[];
      /** Its weapon's `muzzle` socket: the joint it hangs from and its
       *  offset; null where it has none. A flash sits on it. */
      muzzle: AppearanceMuzzle;
      /** Each joint's inverse bind (column-major): a palette joint is its
       *  world times this. */
      inverseBinds: number[][];
      tiers: number;
      /** How big it is drawn (`boundsSize`), for its tier: the far pose,
       *  then each way it lies dead. */
      bounds: { size: number; radius: number }[];
    }
  | {
      kind: "articulated";
      nodes: { name: string; parent: number; bind: TrsRecord }[];
      rig: RigRecord;
      tiers: number;
      /** How big it is drawn at rest (`boundsSize`), for its tier. */
      bounds: { size: number; radius: number }[];
    };

/** `load_world`'s `units`: what poses and draws a battle's units
 *  (`describeUnits`), each clip table's floats as little-endian bytes in
 *  base64. */
export interface DrawnUnits {
  /** `presentation.pose`, as the pose driver read it. */
  pose: Record<string, unknown>;
  /** The rules' `cover.lean_hold_s`: how long a soldier holds his firing pose. */
  leanHold: number;
  /** `presentation.models`: mesh tiers by projected height, and the card size. */
  detail: { lod_px: [number, number, number]; impostor_px: number };
  /** Each side's tint (linear rgb) on tint-masked surfaces (the catalog's `sides`). */
  tints: Record<"blue" | "red", Vec3>;
  types: Record<string, UnitTypeRecord>;
  appearances: Record<string, AppearanceRecord>;
  tables: { name: string; base64: string }[];
}

/** A hull mount's muzzle model (`mountMuzzle.ts` `MountMuzzle`): the mount it
 *  rides (`on`, an index; null: the hull), its pivot and its carrier's ring
 *  in the hull's frame at rest, and its muzzle from the pivot. */
export interface MuzzleRecord {
  on: number | null;
  pivot: Vec3;
  ring: Vec3;
  muzzle: Vec3;
}

/** A thrown wreck's pieces, as a cook-off reads them (`cook_offs.rs`
 *  `transitionOf`), in the wreck's own frame: its turret piece's centre
 *  (null: it throws none), how high the piece's underside lies, and how tall
 *  its thrown debris lies (null: it throws none). */
export interface WreckRecord {
  lies: Vec3 | null;
  underside: number;
  debrisTop: number | null;
}

/** How a unit type's hull dies: its wreck's appearance as the props fit it,
 *  that wreck's footprint (half extents) and its whole state's resting
 *  bounds (min, max), and how hard the dead hull brakes to a stop, m/s². */
export interface WreckLookRecord {
  appearance: string;
  footprint: Vec3;
  bounds: [Vec3, Vec3];
  braking: number;
}

/** `load_world`'s `effects`: what turns a side's publications into its
 *  combat effects (`describeEffects`). */
export interface EffectsSection {
  /** `presentation.effects` as the effects read it: each weapon's rows
   *  resolved to its nearest styled ancestor's (`gameEffects`). */
  presentation: Record<string, unknown>;
  /** Per unit type: its hull's half extents (null: infantry, whose rifles
   *  raise no dust), and each mount's muzzle model (null: a hand weapon). */
  types: Record<string, { half: Vec3 | null; muzzles: (MuzzleRecord | null)[] }>;
  /** The prop kinds that smoke once the side knows them, each with its look
   *  (a `presentation.smoke` row): ruins, gutted shells and wrecks. */
  smokes: Record<string, string>;
  /** Per wreck appearance, the pieces a cook-off throws. */
  wrecks: Record<string, WreckRecord>;
  /** Per unit type, how its hull dies. */
  wreckLooks: Record<string, WreckLookRecord>;
}
