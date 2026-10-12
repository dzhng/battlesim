// What the page reads of a presented frame (`crates/presentation`'s
// `Frame`, through `presentation.ts`) besides drawing it: the counts a
// capture's plates record, and what audio hears move. And what the page
// hands the presentation once per battle (`load_world`'s input).
import type {
  SoundMotion,
  SoundSoldier,
  SoundVehicle,
} from "@packages/battle-audio/src/soundFrame";
import {
  Counts,
  EFFECT_WORDS,
  EMITTER_WORDS,
  Emitter,
  NONE,
} from "@packages/battle-renderer/src/records.gen";
import { vehicleClass, type UnitCatalog } from "@packages/scene-assets/src/units";
import type { PresentedCamera, PresentedFrame } from "@web/battle/sim/presentation";
import { liveCamera, type ViewportCamera } from "@packages/renderer-core/src/cameraUniform";
import { eyePosition } from "@packages/renderer-core/src/camera3d";
import { vec3 } from "math";
import type { ObservationView } from "@web/battle/sim/observationView";
import type { DrawnUnits } from "@web/battle/benchmark/unitManifest";
import type { EffectsManifest } from "@web/battle/benchmark/effectManifest";
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

/** The smallest drawing the battle frame renders, pixels (`battleFrame.ts`). */
const MIN_TARGET_PX = 64;

/** 18b's camera for the viewport's orbit camera, as the battle frame draws it. */
export function presentedCamera(viewport: ViewportCamera): PresentedCamera {
  const camera = liveCamera(viewport);
  const s = Math.sin(camera.pitch);
  return {
    eye: eyePosition(vec3.create(), camera),
    target: camera.target,
    up: [-s * Math.cos(camera.yaw), -s * Math.sin(camera.yaw), Math.cos(camera.pitch)],
    fovY: camera.fovY,
    near: camera.near,
    width: viewport.width,
    height: Math.max(MIN_TARGET_PX, Math.floor(viewport.height)),
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

/** What audio hears move this frame (`Frame.emitters`): each drawn vehicle
 *  as its class, keyed apart by side (own even, as the effects key them),
 *  and each drawn soldier. */
export function frameSound(
  frame: PresentedFrame,
  observation: ObservationView,
  units: UnitCatalog,
): SoundMotion {
  const kinds = new Map<number, string>();
  for (const u of observation.own) kinds.set(u.id * 2, u.kind);
  for (const e of observation.identified) kinds.set(e.id * 2 + 1, e.kind);
  for (const c of observation.crashes) kinds.set(c.id * 2 + (c.own ? 0 : 1), c.kind);
  const motion = { vehicles: [] as SoundVehicle[], soldiers: [] as SoundSoldier[] };
  const { f32, u32 } = frame.emitters;
  for (let i = 0; i < frame.emitters.count; i++) {
    const o = i * EMITTER_WORDS;
    const position = [
      f32[o + Emitter.position],
      f32[o + Emitter.position + 1],
      f32[o + Emitter.position + 2],
    ];
    const flags = u32[o + Emitter.flags];
    if (u32[o + Emitter.member] !== NONE) {
      motion.soldiers.push({ id: u32[o + Emitter.member], position });
      continue;
    }
    const key = u32[o + Emitter.unit] * 2 + (flags & 1);
    const kind = kinds.get(key);
    motion.vehicles.push({
      key,
      vehicleClass: (kind && vehicleClass(units.type(kind))) ?? "default",
      position,
      travelL: f32[o + Emitter.motion],
      travelR: f32[o + Emitter.motion + 1],
      turret: f32[o + Emitter.motion + 2],
      reverse: (flags & 2) !== 0,
    });
  }
  return motion;
}

/** `load_world`'s input: the units and effects sections (`describeUnits`,
 *  `describeEffects`) with each clip table's floats apart, and the facts the
 *  presentation reads of the battle and the page. */
export function presentationWorld(input: {
  units: DrawnUnits;
  effects: EffectsManifest;
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
  effects: EffectsManifest;
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
