/** The page's presentation module (`crates/presentation-wasm`), 18b's
 * browser binding: one side's publications in; the game UI's observation,
 * and each drawn frame's records (`records.gen.ts`), out. Rust decodes,
 * interpolates, poses and makes the effects, and walks the static world;
 * the page hands the frame's views straight to the renderer and reads the
 * UI's facts from them. */
import init, { Presentation, light_records } from "@wasm/presentation/presentation_wasm.js";
import wasmUrl from "@wasm/presentation/presentation_wasm_bg.wasm?url";
import {
  APPEARANCE_WORDS,
  Appearance,
  Batch,
  BATCH_WORDS,
  NONE,
  frameViews,
  worldViews,
} from "@packages/battle-renderer/src/records.gen";
import {
  resolvedLight,
  type LightPresentation,
  type ResolvedLight,
} from "@packages/battle-renderer/src/light/sceneLight";
import type { Articulation } from "@packages/scene-assets/src/articulation";
import type { HeardPublication } from "@packages/battle-audio/src/soundFrame";
import {
  emptyBuildingWalkStats,
  type BuildingWalkStats,
} from "@packages/battle-renderer/src/models/buildingLayer";
import {
  artCovers,
  buildingArt,
  type BuildingArt,
  type BuildingStyle,
  type KnownPart,
  type PlacedBuildings,
  type SceneBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { TEMPLATE_STATES, type TemplateState } from "@packages/scene-assets/src/templateLibrary";
import { countedFetch } from "../../downloads";
import type { ObservationLayout, ObservationView } from "./observationView";

let loaded: Promise<{ memory: WebAssembly.Memory }> | null = null;
let memory: WebAssembly.Memory | null = null;

/** Initialises the module once per realm; decoders need it first. */
export function loadPresentation(): Promise<unknown> {
  // Vitest runs in Node, where Vite's asset URL cannot be fetched (module.ts).
  loaded ??= (
    typeof process !== "undefined" && process.versions?.node
      ? import("node:fs/promises")
          .then(({ readFile }) =>
            readFile(`${import.meta.dirname}/../../wasm/presentation/presentation_wasm_bg.wasm`),
          )
          .then((bytes) => init({ module_or_path: bytes }))
      : init({ module_or_path: countedFetch(wasmUrl) })
  ).then((output) => {
    memory = output.memory;
    return output;
  });
  return loaded;
}

/** `presentation.light` resolved by the presentation crate
 *  (`presentation::light`) into the records the renderer uploads as they
 *  are. A frame is built with it before any battle loads, and for scenes
 *  with none, so it needs no `Presentation`, only the module. */
export async function resolveLight(settings: LightPresentation): Promise<ResolvedLight> {
  await loadPresentation();
  return resolvedLight(settings, light_records(JSON.stringify(settings)));
}
/** 18b's camera: the eye and what it looks at (m, sim), the screen's up,
 *  the vertical field of view (rad), the near plane (m) and the drawing
 *  size in device pixels. */
export interface PresentedCamera {
  eye: readonly number[];
  target: readonly number[];
  up: readonly number[];
  fovY: number;
  near: number;
  width: number;
  height: number;
}

/** A ghost the UI places: one body of a unit where it will stand. */
export interface GhostBody {
  kind: string;
  slot: number;
  id: number;
  x: number;
  y: number;
  /** The ground under it (m); an aircraft's ghost flies its cruise height above. */
  z: number;
  yaw: number;
  /** 0 the side's own colour, 1 the selection's, 2 a purchase refused. */
  look: number;
}

/** What the UI decides of a frame: the selection, whether own units hidden
 *  by the world show through it, and the ghosts; and a lab's stand-in for
 *  the publication's known props, where no battle publishes (a switch that
 *  shows a building destroyed). */
export interface PresentedUi {
  selected: readonly number[];
  xray: boolean;
  ghosts: readonly GhostBody[];
  known?: readonly KnownPart[];
}

export type PresentedFrame = ReturnType<typeof frameViews>;

type P3 = [number, number, number];

/** The last frame as the labs' scenes probe it. */
export interface PresentationProbe {
  effects: { live: number; instances: number; dropped: number; sources: number; tick: number };
  effectInstances: { shape: number; at: P3; to: P3; rgb: P3; size: number; rays: number }[];
  castLights: { at: P3; radius: number; rgb: P3; cause: string }[];
  lying: number[];
  cookOffPieces: { appearance: string; state: string; sunk: number }[];
  soldiers: { appearance: string; position: P3; yaw: number; clip: string; phase: number }[];
  vehicles: {
    appearance: string;
    position: P3;
    yaw: number;
    tint: P3 | null;
    articulation: Articulation;
  }[];
}
/** One drawable the frame's batches name, by `World.appearances` row;
 *  `placed` for the static world's (`Appearance.placed`). */
export interface PresentedAppearance {
  name: string;
  state: string;
  lies: number;
  placed: boolean;
}

const buffer = () => memory!.buffer;

/** A map's buildings as `load_world` takes them: where they stand, the
 *  template art they are drawn from, which building each part prop is, and
 *  the prop kinds a gutted building's parts become. */
export interface PresentedBuildings {
  placed: PlacedBuildings;
  art: BuildingArt;
  partBuilding?: ReadonlyMap<number, number>;
  shells?: Iterable<string>;
}

/** What a battle loads into its presentation (`WorldInput`'s battle, as the
 *  native manifest carries it, with each clip table's floats). */
export interface PresentedBattle {
  world: object;
  tables: ReadonlyMap<string, Float32Array>;
}

/** The sun the static world casts under: azimuth and elevation (rad), and
 *  the cascades' far plane (`light.cascades.max_far_m`). */
export interface PresentedSun {
  azimuth: number;
  elevation: number;
  farM: number;
}

/** A building a side knows destroyed, and the state it is drawn in. */
export interface FallenBuilding {
  building: number;
  state: Exclude<TemplateState, "intact">;
}

/** One ordered side-publication stream, and its presentation; or, with no
 * layout, a presentation no battle publishes into (a world of statics).
 * Retained observations are immutable; unchanged fog and static groups
 * share their views.
 *
 * A scene has one: what it draws is staged (`setBattle`, `setBuildings`)
 * and loaded together, in one `load_world`, by the next frame after any of
 * it changed. */
export class ObservationDecoder {
  private readonly rust: Presentation;
  private last: ObservationView | null = null;
  /** The last frame's header. */
  private framePtr = 0;
  private battle: PresentedBattle | null = null;
  private buildings: SceneBuildings | null = null;
  private installed: InstalledAppearances | null = null;
  private art: BuildingArt | null = null;
  private style: BuildingStyle | null = null;
  private sun: PresentedSun | null = null;
  /** Something staged changed since the last load. */
  private stale = false;
  /** The drawables of the last load, and those the page has not taken yet. */
  private rows: readonly PresentedAppearance[] = [];
  private loaded: PresentedAppearance[] | null = null;
  /** The last frame wrote the static batches: its walk changed something. */
  private walked = false;
  private readonly walk = emptyBuildingWalkStats();

  /** `tickHz` paces the interpolation; decoding alone ignores it. */
  constructor(layout?: string | ObservationLayout, tickHz = 30) {
    this.rust = new Presentation(
      layout === undefined || typeof layout === "string" ? layout : JSON.stringify(layout),
      tickHz,
    );
  }

  /** The next record's observation, or null for a stale one (in flight from
   * before `invalidate`). A malformed record throws and changes nothing.
   * `arrivedMs` is the page's clock when it came (`performance.now()`). */
  decode(data: Float32Array | Uint32Array, arrivedMs = 0): ObservationView | null {
    const words = new Uint32Array(data.buffer, data.byteOffset, data.length);
    if (!this.rust.push_publication(words, arrivedMs / 1000)) return null;
    const view = JSON.parse(this.rust.observation()) as ObservationView;
    const last = this.last;
    view.corpses ??= last!.corpses;
    view.knownProps ??= last!.knownProps;
    view.fallenBodies ??= last!.fallenBodies;
    view.fog.bits = this.rust.fog_bits() ?? last!.fog.bits;
    view.groundPatch.runs = this.rust.ground_runs();
    return (this.last = view);
  }

  /** Records still in flight are read as stale; the next must open a new
   * epoch (for `side`, when named). */
  invalidate(side?: string) {
    this.rust.invalidate(side);
    this.last = null;
  }

  /** Load what the world draws (`WorldInput`: a battle's `units` and
   *  `effects`, as the native manifest carries them, with each clip table's
   *  floats; the sun; the buildings' style, with the buildings); returns
   *  the drawables its frames' batches name (`World.appearances`), by row.
   *  A template the art lacks is refused by name. */
  loadWorld(
    world: object,
    tables: ReadonlyMap<string, Float32Array> = new Map(),
    buildings?: PresentedBuildings,
  ): PresentedAppearance[] {
    for (const [name, floats] of tables) this.rust.add_clip_table(name, floats);
    if (buildings) {
      const { placed, art, partBuilding = new Map(), shells = [] } = buildings;
      const bounds = new Float32Array(art.bounds.length * 6).fill(Number.NaN);
      art.bounds.forEach((b, m) => b && bounds.set([...b.min, ...b.max], m * 6));
      this.rust.add_buildings(
        art.bytes,
        bounds,
        JSON.stringify(placed.templates),
        placed.template,
        placed.frames,
        placed.owners,
        Uint32Array.from(partBuilding.keys()),
        Uint32Array.from(partBuilding.values()),
        JSON.stringify([...shells].sort()),
      );
    }
    // The call may grow memory: the views are taken after it.
    const ptr = this.rust.load_world(JSON.stringify(world));
    const views = worldViews(buffer(), ptr);
    const text = new TextDecoder();
    const rows = views.appearances.u32;
    const name = (at: number, len: number) => text.decode(views.strings.subarray(at, at + len));
    return (this.rows = Array.from({ length: views.appearances.count }, (_, r) => {
      const o = r * APPEARANCE_WORDS;
      return {
        name: name(rows[o + Appearance.name], rows[o + Appearance.name_len]),
        state: name(rows[o + Appearance.state], rows[o + Appearance.state_len]),
        lies: rows[o + Appearance.lies],
        placed: rows[o + Appearance.placed] === 1,
      };
    }));
  }

  /** Stage what the battle draws (null: none); loaded with the rest by the
   *  next frame when it is another. */
  setBattle(battle: PresentedBattle | null) {
    if (battle === this.battle) return;
    this.battle = battle;
    this.stale = true;
  }

  /** Stage the scene's buildings (null: none) drawn from `installed`'s
   *  template art under `style`, casting under `sun`: loaded with the rest
   *  by the next frame when any of it is another, and once the art covers
   *  every kit they need. Cheap when nothing changed: call every frame. */
  setBuildings(
    buildings: SceneBuildings | null,
    installed: InstalledAppearances | null,
    style: BuildingStyle,
    sun: PresentedSun,
  ) {
    if (installed !== this.installed) {
      this.installed = installed;
      const next = buildingArt(installed);
      const art = this.art;
      const same =
        art?.library === next?.library &&
        (next?.bounds ?? []).every((b, i) => (b === null) === (art!.bounds[i] === null));
      if (!same) [this.art, this.stale] = [next, true];
    }
    const last = this.buildings;
    if (
      buildings?.placed !== last?.placed ||
      buildings?.partBuilding !== last?.partBuilding ||
      buildings?.shells !== last?.shells ||
      style !== this.style ||
      sun.azimuth !== this.sun?.azimuth ||
      sun.elevation !== this.sun?.elevation ||
      sun.farM !== this.sun?.farM
    )
      this.stale = true;
    [this.buildings, this.style, this.sun] = [buildings, style, sun];
  }

  /** Load what is staged: the battle and the buildings, in one world. */
  private load() {
    this.stale = false;
    const { battle, buildings, art, style, sun } = this;
    const drawn =
      buildings && art && style && buildings.placed.template.length > 0
        ? artCovers(buildings.placed, art) && buildings
        : null;
    if (!battle && !sun) return;
    const started = performance.now();
    const lit = sun && {
      sun_azimuth: sun.azimuth,
      sun_elevation: sun.elevation,
      cascade_far_m: sun.farM,
    };
    this.loaded = this.loadWorld(
      { ...battle?.world, ...lit, buildings: drawn ? style : undefined },
      battle?.tables,
      drawn
        ? {
            placed: drawn.placed,
            art: art!,
            partBuilding: drawn.partBuilding,
            shells: drawn.shells,
          }
        : undefined,
    );
    Object.assign(this.walk, emptyBuildingWalkStats());
    this.walk.buildings = drawn ? drawn.placed.template.length : 0;
    this.walk.pool.capacity = drawn ? style!.pool_records : 0;
    this.walk.buildMs = performance.now() - started;
  }

  /** The drawables of the world last loaded (null before any): the same
   *  array until another load, so a viewport built after the load (a lab
   *  rebuilding its frame) installs them too. */
  world(): PresentedAppearance[] | null {
    return this.loaded;
  }

  /** The buildings the side knows destroyed, as the last frame drew them. */
  fallenBuildings(): FallenBuilding[] {
    const pairs = this.rust.fallen_buildings();
    return Array.from({ length: pairs.length / 2 }, (_, i) => ({
      building: pairs[i * 2],
      state: TEMPLATE_STATES[pairs[i * 2 + 1]] as FallenBuilding["state"],
    }));
  }

  /** The buildings' walk as the last frame left it. Calls into the module:
   *  take the frame's views again after it (`lastFrame`). */
  buildingWalk(): BuildingWalkStats {
    const [chunks, coarse, resident, used, expanded, refused, ruins, ...ruinTiers] =
      this.rust.building_stats();
    const walk = this.walk;
    Object.assign(walk, {
      chunks: chunks ?? 0,
      coarse: coarse ?? 0,
      residentChunks: resident ?? 0,
      expandedRows: expanded ?? 0,
      refused: refused ?? 0,
      ruins: ruins ?? 0,
      ruinTiers: ruinTiers.length ? ruinTiers : walk.ruinTiers.map(() => 0),
      fallen: this.rust.fallen_buildings().length / 2,
    });
    walk.pool.used = used ?? 0;
    if (this.walked) walk.totalExpandedRows += walk.expandedRows;
    this.walked = false;
    return walk;
  }

  /** The renderer holds no static batches (a new one): the next frame
   *  writes every one. */
  resendStatics() {
    this.rust.resend_statics();
  }

  free() {
    this.rust.free();
  }

  /** The presentation time at `nowMs` (sim seconds), or null before any publication. */
  clock(nowMs: number): number | null {
    return this.rust.clock(nowMs / 1000) ?? null;
  }

  /** What to draw at presentation time `time` for `camera`, loading first
   *  what was staged: views over the module's memory, valid until the next
   *  call into it. A building the side knows in a state its template lacks
   *  throws by name. */
  frame(time: number, camera: PresentedCamera, ui: PresentedUi): PresentedFrame {
    if (this.stale) this.load();
    const c = camera;
    const started = performance.now();
    const ptr = this.rust.frame(
      time,
      Float64Array.of(
        c.eye[0],
        c.eye[1],
        c.eye[2],
        c.target[0],
        c.target[1],
        c.target[2],
        c.up[0],
        c.up[1],
        c.up[2],
        c.fovY,
        c.near,
        c.width,
        c.height,
      ),
      Uint32Array.from(ui.selected),
      ui.xray,
      ui.ghosts.length ? JSON.stringify(ui.ghosts) : "",
      ui.known
        ? JSON.stringify(
            ui.known.map((k) => ({
              authoredProp: k.authoredProp,
              kind: k.kind,
              destroyed: k.destroyed ?? false,
            })),
          )
        : "",
    );
    this.walk.selectMs = performance.now() - started;
    this.framePtr = ptr;
    const views = frameViews(buffer(), ptr);
    const batches = views.batches.u32;
    this.walked = false;
    for (let b = 0; b < views.batches.count && !this.walked; b++) {
      const o = b * BATCH_WORDS;
      this.walked =
        batches[o + Batch.first] !== NONE && !!this.rows[batches[o + Batch.appearance]]?.placed;
    }
    return views;
  }

  /** The last frame's views, taken again after another call into the module. */
  lastFrame(): PresentedFrame {
    return frameViews(buffer(), this.framePtr);
  }

  /** The last publication as sound reads it: its effect publication (with
   *  the hulls the side watched brew up) and the launches it shows. */
  soundPublication(): HeardPublication | null {
    return JSON.parse(this.rust.sound_publication()) as HeardPublication | null;
  }

  /** The last frame as the labs' scenes probe it (`Presentation::probe_json`). */
  probe(): PresentationProbe {
    return JSON.parse(this.rust.probe()) as PresentationProbe;
  }

  /** Where the last frame draws the presented side's vehicle `unit`'s mount
   *  `mount`'s muzzle; null when no rig draws it. */
  drawnMuzzle(unit: number, mount: number): [number, number, number] | null {
    const at = this.rust.drawn_muzzle(unit, mount);
    return at ? [at[0], at[1], at[2]] : null;
  }

  /** The known props whose wreck the presentation draws moving (a cook-off),
   *  which the side's static structures leave out. */
  movingWrecks(): Uint32Array {
    return this.rust.moving_wrecks();
  }
}
