/** The page's presentation module (`crates/presentation-wasm`), 18b's
 * browser binding: one side's publications in; the game UI's observation,
 * and each drawn frame's records (`records.gen.ts`), out. Rust decodes,
 * interpolates, poses and makes the effects, and walks the static world;
 * the page hands the frame's views straight to the renderer and reads the
 * UI's facts from them. */
import init, {
  Presentation,
  fog_look,
  light_records,
} from "@wasm/presentation/presentation_wasm.js";
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
import type { FogStyle } from "@packages/battle-renderer/src/frame/fogStyle";
import type { Articulation } from "@packages/scene-assets/src/articulation";
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
import type {
  GroundMarks,
  ScarRegion,
  ScarUpload,
} from "@packages/battle-renderer/src/frame/scarTexture";
import type { GroundLayout, ObservationLayout, ObservationView } from "./observationView";

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

/** A `presentation.fog` style checked and resolved by the presentation crate
 *  (`presentation::fog`) into the `FogLook` record the fog mask passes
 *  upload as it is; throws naming a field out of range. Needs the module
 *  loaded, as it is once a frame is built (`resolveLight`). */
export function resolveFogLook(style: FogStyle): Float32Array {
  if (!memory) throw new Error("resolveFogLook: the presentation module is not loaded");
  return fog_look(JSON.stringify(style));
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

/** The side's learned ground as the presentation keeps it
 *  (`crates/presentation/src/scars`): what its publications taught it, cell
 *  by cell, and its scars as the renderer uploads them. */
export class LearnedGround implements GroundMarks {
  readonly cellM: number;
  readonly cols: number;
  readonly rows: number;
  private regionsFlat: Float64Array | null = null;
  private regionList: ScarRegion[] = [];

  constructor(
    private readonly rust: Presentation,
    grid: Pick<GroundLayout, "cellM" | "cols" | "rows">,
  ) {
    ({ cellM: this.cellM, cols: this.cols, rows: this.rows } = grid);
  }

  get epoch(): number {
    return this.rust.ground_epoch();
  }
  get revision(): number {
    return this.rust.ground_revision();
  }
  /** The side it learned for, once a snapshot came. */
  get side(): string | null {
    return this.rust.ground_side() ?? null;
  }
  /** Cells the side has seen cleared. */
  get clearedCount(): number {
    return this.rust.ground_cleared_count();
  }
  isCleared(i: number, j: number): boolean {
    return i >= 0 && j >= 0 && this.rust.ground_cleared(i, j);
  }
  /** The cleared spans in tile order: (tile, start + length × 256) pairs. */
  clearedRuns(): Uint32Array {
    return this.rust.ground_cleared_runs();
  }
  /** Every marked or cleared cell in row-major order: (cell index, marks as
   *  drawn: crater, scorch, tracks, trampled a byte each) pairs. */
  markedCells(): Uint32Array {
    return this.rust.ground_marked();
  }

  changed(): boolean {
    return this.rust.scars_changed();
  }
  regions(): readonly ScarRegion[] {
    const flat = this.rust.scar_regions();
    if (this.regionsFlat?.length !== flat.length) {
      this.regionsFlat = flat;
      this.regionList = [];
      for (let k = 0; k < flat.length; k += 4)
        this.regionList.push([flat[k], flat[k + 1], flat[k + 2], flat[k + 3]]);
    }
    return this.regionList;
  }
  prepare(region?: ScarRegion, clipped = false): ScarUpload {
    const upload = this.rust.prepare_scars(region && Float64Array.from(region), clipped);
    const words = upload.directory(),
      size = upload.directory_size();
    const out: ScarUpload = {
      directory: words ? { words, width: size[0], height: size[1] } : null,
      writes: upload.writes(),
      texels: upload.texels(),
      params: upload.params(),
    };
    upload.free();
    return out;
  }
  resend() {
    this.rust.resend_scars();
  }
  stats() {
    return JSON.parse(this.rust.scar_stats()) as ReturnType<GroundMarks["stats"]>;
  }
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

  /** The side's learned ground, from the publications decoded; none
   *  without a layout. */
  readonly ground: LearnedGround | null;

  /** `tickHz` paces the interpolation; decoding alone ignores it. */
  constructor(layout?: string | ObservationLayout, tickHz = 30) {
    const json =
      layout === undefined || typeof layout === "string" ? layout : JSON.stringify(layout);
    this.rust = new Presentation(json, tickHz);
    this.ground =
      json === undefined
        ? null
        : new LearnedGround(this.rust, (JSON.parse(json) as ObservationLayout).ground);
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

  /** How far a crater's bowl moves the scars a view ray reads, metres (the
   *  static ground's `scar_reach_m`): the scar cache loads a region this far
   *  past it. */
  setScarReach(reachM: number) {
    this.rust.set_scar_reach(reachM);
  }

  /** The cells the latest publication's ground patch covered. */
  groundPatchCells(): number {
    const runs = this.rust.ground_runs();
    let cells = 0;
    for (let at = 1; at < runs.length; at += 4) cells += Math.floor(runs[at] / 256);
    return cells;
  }

  /** Draws the units' tiers at `units` and the buildings' at `buildings`
   *  times their authored thresholds (a graphics quality; 1 as authored). */
  setDetailScale(units: number, buildings: number) {
    this.rust.set_detail_scale(units, buildings);
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

  /** The module's presentation, which battle sound hears
   *  (`battle-audio`'s `SoundFrame`). */
  get wasm(): Presentation {
    return this.rust;
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
