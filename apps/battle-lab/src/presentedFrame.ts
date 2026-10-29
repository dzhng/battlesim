// What the page reads of a presented frame (`crates/presentation`'s
// `Frame`, through `presentation.ts`) besides drawing it: the counts a
// capture's plates record. And what the page hands the presentation once
// per battle (`load_world`'s battle), which Rust resolves from the files the
// page reads.
import {
  Counts,
  EFFECT_KINDS,
  EFFECT_WORDS,
  SHAPE_NAMES,
  MIN_TARGET_PX,
} from "@packages/battle-renderer/src/records.gen";
import game from "@fixtures/game.json";
import type { AppearanceLibrary } from "@packages/scene-assets/src/loader";
import type { CatalogView } from "@packages/scene-assets/src/units";
import { Fielding } from "@wasm/renderer_browser";
import type {
  PresentedBattle,
  PresentedCamera,
  PresentedFrame,
} from "@web/battle/sim/presentation";
import { liveCamera, type ViewportCamera } from "@packages/renderer-core/src/viewportCamera";
import { eyePosition } from "@packages/renderer-core/src/camera3d";
import { vec3 } from "math";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import type { PickBox } from "@packages/battle-renderer/src/picking";
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

/** The runtime files a battle's art is read from: the runtime catalog the
 *  page's appearances were loaded from, and its bundle contents by hash
 *  (`AppearanceLibrary`). */
export type RuntimeFiles = Pick<AppearanceLibrary, "catalog" | "contents">;

/** `load_world`'s battle for a battle fielding `kinds` of the session's unit
 *  `catalog` under `rules` (its scenario's rules, JSON), each side fighting
 *  for `factions` (blue, red).
 *  Rust resolves what its units and effects draw from
 *  (`presentation::fielded`); the page only reads the files it names
 *  (`fixtures/game.json`, the runtime catalog, its bundles and skeleton
 *  clips) and adds the facts the presentation reads of the battle and the
 *  page. */
export async function fieldedBattle(input: {
  rules: string;
  catalog: CatalogView;
  kinds: readonly string[];
  factions?: readonly [string, string];
  runtime: RuntimeFiles;
  side: SideName;
  tickHz: number;
  sun: { azimuth: number; elevation: number };
  soldier: { soldier_radius_m: number; soldier_height_m: number };
  xray: { own: readonly number[]; selected: readonly number[]; ally: readonly number[] };
  blocked: readonly number[];
}): Promise<PresentedBattle> {
  const catalog = input.runtime.catalog;
  if (!catalog) throw new Error("a battle was fielded before the runtime catalog was loaded");
  const fielding = new Fielding(
    JSON.stringify(game),
    input.rules,
    JSON.stringify(input.catalog),
    JSON.stringify(catalog),
    JSON.stringify(input.kinds),
    input.factions && JSON.stringify(input.factions),
  );
  try {
    const files = await input.runtime.contents(fielding.contents());
    const lengths = Uint32Array.from(files, (f) => f.length);
    const bytes = new Uint8Array(lengths.reduce((n, l) => n + l, 0));
    let at = 0;
    for (const f of files) {
      bytes.set(f, at);
      at += f.length;
    }
    const { units, effects, tables } = JSON.parse(fielding.resolve(bytes, lengths)) as {
      units: object;
      effects: object;
      tables: string[];
    };
    return {
      world: {
        battle: {
          units,
          effects,
          side: input.side,
          tick_hz: input.tickHz,
          soldier: {
            radius_m: input.soldier.soldier_radius_m,
            height_m: input.soldier.soldier_height_m,
          },
          xray: input.xray,
          blocked: input.blocked,
        },
        sun_azimuth: input.sun.azimuth,
        sun_elevation: input.sun.elevation,
      },
      tables: new Map(tables.map((name, i) => [name, fielding.table(i)])),
    };
  } finally {
    fielding.free();
  }
}

/** One animation frame's drawn units. Anything omitted keeps its last value. */
export interface ViewportFrame {
  /** Commit this candidate observation only after successful GPU submission. */
  submitted?: () => void;
  /** Proxies to draw (lab markers). */
  instances?: readonly SceneInstance[];
  /** What a click can pick, as boxes (every drawn soldier and vehicle, by
   *  the simulation's bodies); the drawn proxies when omitted. Picks report
   *  an index into this list. */
  picks?: readonly PickBox[];
  /** The presentation's frame (`crates/presentation`): the units, corpses,
   *  ghosts and effects it draws, and the static world's batches, read
   *  before the next call into the module. */
  presented?: PresentedFrame;
  /** How many models of each kind the frame poses. */
  counts?: ModelCounts;
  /** The presentation clock, in seconds (the pose driver's and the wind's). */
  clock?: number;
  /** What the frame's effects hold, by kind and shape. */
  effectCounts?: EffectCounts;
}
