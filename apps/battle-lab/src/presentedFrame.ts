// What the page reads of a presented frame (`crates/presentation`'s
// `Frame`, through `presentation.ts`) besides drawing it: the counts a
// capture's plates record. And what the page hands the presentation once
// per battle (`load_world`'s input).
import { Counts, EFFECT_WORDS } from "@packages/battle-renderer/src/records.gen";
import { MIN_TARGET_PX } from "@packages/battle-renderer/src/frame/battleFrame";
import type { PresentedCamera, PresentedFrame } from "@web/battle/sim/presentation";
import { liveCamera, type ViewportCamera } from "@packages/renderer-core/src/viewportCamera";
import { eyePosition } from "@packages/renderer-core/src/camera3d";
import { vec3 } from "math";
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
