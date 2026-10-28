/** Browser binding of Rust presentation: side publications in, observations
 * and frame views out. Rust owns decoding, poses, effects and static walks. */
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
import {
  setGround,
  type GroundInput,
  type PresentedGround,
} from "@packages/battle-renderer/src/terrain/terrainSurface";
import { tierMesh } from "@packages/battle-renderer/src/scenery/appearance";
import { concatMeshes, type Mesh } from "@packages/battle-renderer/src/mesh";
import { STUMP } from "@packages/battle-renderer/src/frame/sceneryLayer";
import { occluderBoxes, type FogOccluder } from "@packages/battle-renderer/src/frame/fogVisibility";
import type { WorldScenery } from "@packages/battle-renderer/src/scene";
import { propArt, type KnownProp } from "@packages/battle-renderer/src/models/propAppearance";
import {
  NO_SCENERY_WALK,
  type SceneryWalkStats,
} from "@packages/battle-renderer/src/frame/sceneryLayer";
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

/** Receiver traversal work, separate from terrain draw selection. */
export interface ReceiverFitStats {
  bounds_visited: number;
  triangle_tests: number;
  clip_tests: number;
  reused: boolean;
}

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
/** The camera a frame is drawn for: the eye and what it looks at (m, sim), the screen's up,
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
  /** Final world altitude, shared with the ghost's floating card. */
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
  /** The index of the player the UI commands as: the side's units another
   *  player owns draw as its ally's. Absent where units have no owner. */
  player?: number | null;
  ghosts: readonly GhostBody[];
  /** A lab's stand-in for the publication's known props: what the
   *  buildings read of each, and, for the props to draw it, its id and box
   *  (`KnownProp`); none reads the publication's. */
  known?: readonly (KnownPart & Partial<KnownProp>)[];
}

export type PresentedFrame = ReturnType<typeof frameViews> & { waves?: Float32Array };

import type { PresentationProbe } from "./presentationProbe";
export type { PresentationProbe };
/** One drawable the frame's batches name, by `World.appearances` row;
 *  `placed` for the static world's (`Appearance.placed`). */
export interface PresentedAppearance {
  name: string;
  state: string;
  lies: number;
  placed: boolean;
  /** What draws it (`Appearance.draws`): 0 a model or kit module, else the scenery. */
  draws: number;
  /** A felled tree's stump row's mesh (`draws` 6; `Presentation::stump_mesh`). */
  stump?: Mesh;
}

const buffer = () => memory!.buffer;

/** The ground `rust` holds, as the renderer reads its tables: a new object
 *  for each ground set, so a renderer knows to read them again. */
function groundOf(rust: Presentation): PresentedGround {
  return {
    prepare_fog: (camera, width, height, probes) => rust.prepare_fog(camera, width, height, probes),
    resend_fog: () => rust.resend_fog(),
    rebuild_fog: () => rust.rebuild_fog(),
    fog_stats: () => rust.fog_stats(),
    fog_occluders: () => rust.fog_occluders(),
    ground_terrain: () => rust.ground_terrain(),
    ground_plots: (textured) => rust.ground_plots(textured),
    ground_water_mesh: () => rust.ground_water_mesh(),
    grass_kinds: () => rust.grass_kinds(),
    grass_params: (shapes, density) => rust.grass_params(shapes, density),
    grass_window: () => rust.grass_window(),
    grass_frame_params: (words, window) => rust.grass_frame_params(words, window),
    grass_props: () => rust.grass_props(),
    retune_grass: (rules) => rust.retune_grass(rules),
  };
}

/** One own eye fog sees from (`presentation::fog::sight::Eye`). */
export interface FogEye {
  /** `<unit>:<eye>`: the same eye keeps its map while it stands still. */
  key: string;
  position: readonly [number, number, number];
  forward: number;
  shape: { front: number; side: number; rear: number };
  range: number;
}

/** What a scene asks of fog of war (`Presentation.set_fog`): the
 *  presentation sees from the side's own units at each publication, cut by
 *  the ground and what the side knows stands. */
export interface FogRequest {
  /** The rules' sensor numbers fog shares with the simulation (`sensors`). */
  sensors: object;
  /** The side's foliage less the ground it has seen cleared, once it has
   *  (`WorldView.foliage_cleared`); the map's until then. */
  foliage?: Float32Array;
  /** A lab's stand-in for the publication's eyes. */
  eyes?: readonly FogEye[];
  /** Only these own units' eyes (a lab looking through one unit). */
  units?: readonly number[];
}

/** A map's buildings as `load_world` takes them: where they stand, the
 *  template art they are drawn from, which building each part prop is, and
 *  the prop kinds a gutted building's parts become. */
export interface PresentedBuildings {
  placed: PlacedBuildings;
  art: BuildingArt;
  partBuilding?: ReadonlyMap<number, number>;
  shells?: Iterable<string>;
}

/** What a battle loads into its presentation (`WorldInput`'s battle, as
 *  Rust resolves it from the files the page reads: `fieldedBattle`), with
 *  each clip table's floats. */
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

/** What a scene's props draw (`ScenePresentation.propStats`). */
export interface PropStats {
  bodies: number;
  parts: number;
  unmodelled: number;
  models: number;
  known: number;
  /** Bounds tested by the last spatial query, including hierarchy branches. */
  visitedBounds: number;
}

export const NO_PROP_STATS: PropStats = {
  bodies: 0,
  parts: 0,
  unmodelled: 0,
  models: 0,
  known: 0,
  visitedBounds: 0,
};

/** A prop model the presentation placed. */
export interface PropModel {
  appearance: string;
  state: string;
  position: [number, number, number];
  yaw: number;
  scale: [number, number, number];
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
export class ScenePresentation {
  private readonly rust: Presentation;
  private last: ObservationView | null = null;
  /** The last frame's header. */
  private framePtr = 0;
  /** The wave field's frame the last `frame` took (`Presentation.waves`). */
  private waves: Float32Array = new Float32Array(0);
  private battle: PresentedBattle | null = null;
  private buildings: SceneBuildings | null = null;
  private installed: InstalledAppearances | null = null;
  private art: BuildingArt | null = null;
  private style: BuildingStyle | null = null;
  private sun: PresentedSun | null = null;
  /** The ground staged, the one set, and the scenery's art it was placed by. */
  private staged: GroundInput | null = null;
  private groundSet: GroundInput | null = null;
  /** The art the staged ground's scenery draws, and the art it was placed by. */
  private sceneryArt: WorldScenery | null = null;
  private placedBy: WorldScenery | null = null;
  /** The installed art the ground's props were placed from. */
  private proppedBy: InstalledAppearances | null = null;
  /** The ground the page reads tables from, new with each ground set. */
  private presented: PresentedGround | null = null;
  /** The fog asked for, the one set, and the foliage handed over. */
  private fogStaged: FogRequest | null = null;
  private fogSet: FogRequest | null | undefined = undefined;
  private foliageSet: Float32Array | null = null;
  /** The light and the ground mesh the cascades were last fitted under. */
  private shadow: [LightPresentation | null, Float32Array | null] = [null, null];
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
   *  `effects`, as Rust resolved them, with each clip table's
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
    const loaded: PresentedAppearance[] = Array.from(
      { length: views.appearances.count },
      (_, r) => {
        const o = r * APPEARANCE_WORDS;
        return {
          name: name(rows[o + Appearance.name], rows[o + Appearance.name_len]),
          state: name(rows[o + Appearance.state], rows[o + Appearance.state_len]),
          lies: rows[o + Appearance.lies],
          placed: rows[o + Appearance.placed] === 1,
          draws: rows[o + Appearance.draws],
        };
      },
    );
    // After the views: each call may grow memory.
    loaded.forEach((row, r) => {
      if (row.draws === STUMP) row.stump = this.rust.stump_mesh(r) as Mesh;
    });
    return (this.rows = loaded);
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
      // The props are placed from what the models layer installs.
      if (this.staged?.props) this.stale = true;
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

  /** Stage the scene's ground (null: none) and the art its scenery draws
   *  (`WorldLayers.scenery`; null: none placed): set, placed and loaded with
   *  the rest by the next frame when either is another. Cheap when nothing
   *  changed: call every frame. */
  setGround(ground: GroundInput | null, scenery: WorldScenery | null) {
    if (ground !== this.staged) [this.staged, this.stale] = [ground, true];
    if (scenery !== this.sceneryArt) [this.sceneryArt, this.stale] = [scenery, true];
  }

  /** The side's fog of war over the ground, or none; taken once the
   *  ground is set. */
  setFog(fog: FogRequest | null) {
    this.fogStaged = fog;
    if (!this.presented || fog === this.fogSet) return;
    this.fogSet = fog;
    const foliage = fog?.foliage !== this.foliageSet ? fog?.foliage : undefined;
    if (foliage) this.foliageSet = foliage;
    const { sensors, eyes, units } = fog ?? {};
    this.rust.set_fog(fog ? JSON.stringify({ sensors, eyes, units }) : undefined, foliage);
  }

  /** The occluders the side's fog knows stand, as boxes. */
  fogOccluders(): FogOccluder[] {
    return occluderBoxes(this.rust.fog_occluders());
  }

  /** The eyes fog sees from, as its last frame took them. */
  fogEyes(): FogEye[] {
    return JSON.parse(this.rust.fog_eyes()) as FogEye[];
  }

  /** The ground the scene's presentation resolved, for the terrain, the
   *  grass and the water; null before one is set. */
  presentedGround(): PresentedGround | null {
    return this.presented;
  }

  /** Install immutable receiver inputs. Same identities preserve the fit cache. */
  setShadow(light: LightPresentation, mesh: Float32Array | null) {
    if (light === this.shadow[0] && mesh === this.shadow[1]) return;
    this.shadow = [light, mesh];
    this.rust.set_shadow(JSON.stringify(light), mesh ?? new Float32Array(0));
  }

  /** Latest frame's executed receiver work. */
  shadowStats(): ReceiverFitStats {
    return JSON.parse(this.rust.shadow_stats()) as ReceiverFitStats;
  }

  /** The player's scenery settings: the trees' tier thresholds over the
   *  biome's, and whether the forest floor's dressing and the tree lines'
   *  shrubs draw. */
  setSceneryQuality(treeLodScale: number, dressing: boolean, understorey: boolean) {
    this.rust.set_scenery_quality(treeLodScale, dressing, understorey);
  }

  /** What the presentation says of the scenery it placed and walked. */
  sceneryWalk(): SceneryWalkStats {
    const [
      forest,
      understorey,
      backdrop,
      cells,
      visitedBounds,
      envelopeRowsTotal,
      snapshotRowsTotal,
      membershipChecks,
    ] = this.rust.scenery_stats();
    if (forest === undefined) return NO_SCENERY_WALK;
    return {
      placed: [forest, understorey, backdrop],
      visitedBounds,
      felledWork: { envelopeRowsTotal, snapshotRowsTotal, membershipChecks },
      cells,
      pending: this.rust.scenery_pending(),
      falling: this.rust.scenery_falling(),
    };
  }

  /** Sets the staged ground, places its props (in the surface view) from
   *  what the models layer installs, and its scenery once its art holds
   *  every appearance it names. */
  private loadGround() {
    const ground = this.staged;
    if (ground !== this.groundSet) {
      this.groundSet = ground;
      this.placedBy = null;
      this.proppedBy = null;
      this.presented = null;
      if (ground) {
        setGround(this.rust, ground);
        this.presented = groundOf(this.rust);
        [this.fogSet, this.foliageSet] = [undefined, null];
        this.setFog(this.fogStaged);
      }
    }
    const installed = this.installed;
    if (ground?.props && installed && installed !== this.proppedBy) {
      this.rust.place_props(ground.layout, propArt(installed, ground.family));
      this.proppedBy = installed;
    }
    const art = this.sceneryArt;
    if (!ground || !art || art === this.placedBy) return;
    const kinds = JSON.parse(this.rust.scenery_kinds()) as string[];
    const bundles = kinds.map((name) => art.appearances.get(name));
    if (!bundles.every((b) => b !== undefined)) return;
    const finest = bundles.map((b) => tierMesh(b!, 0));
    this.rust.place_scenery(
      concatMeshes(finest),
      Uint32Array.from(finest, (m) => m.length),
    );
    this.placedBy = art;
  }

  /** Load what is staged: the battle, the buildings and the ground, in one world. */
  private load() {
    this.stale = false;
    this.loadGround();
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

  /** What the props draw (`Presentation.prop_stats`): the bodies drawn,
   *  those a building draws, those no model draws, the models placed and
   *  those drawing what the side knows. */
  propStats(): PropStats {
    const [bodies = 0, parts = 0, unmodelled = 0, models = 0, known = 0, visitedBounds = 0] =
      this.rust.prop_stats();
    return { bodies, parts, unmodelled, models, known, visitedBounds };
  }

  /** Every prop model placed: appearance, state, position, yaw and scale. */
  propModels(): PropModel[] {
    return JSON.parse(this.rust.prop_models()) as PropModel[];
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
    const [
      chunks,
      coarse,
      resident,
      used,
      expanded,
      refused,
      ruins,
      t0,
      t1,
      t2,
      t3,
      visitedBounds,
    ] = this.rust.building_stats();
    const walk = this.walk;
    Object.assign(walk, {
      chunks: chunks ?? 0,
      coarse: coarse ?? 0,
      residentChunks: resident ?? 0,
      expandedRows: expanded ?? 0,
      refused: refused ?? 0,
      ruins: ruins ?? 0,
      ruinTiers: [t0 ?? 0, t1 ?? 0, t2 ?? 0, t3 ?? 0],
      visitedBounds: visitedBounds ?? 0,
      fallen: this.rust.fallen_buildings().length / 2,
    });
    walk.pool.used = used ?? 0;
    if (this.walked) walk.totalExpandedRows += walk.expandedRows;
    this.walked = false;
    return walk;
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
   *  call into it. `generation` is the renderer's, bumped whenever it holds
   *  no static batches (a new GPU frame, a new world): a new one is written
   *  every one. A building the side knows in a state its template lacks
   *  throws by name. */
  frame(
    time: number,
    camera: PresentedCamera,
    ui: PresentedUi,
    generation: number,
  ): PresentedFrame {
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
      ui.player ?? undefined,
      ui.ghosts.length ? JSON.stringify(ui.ghosts) : "",
      ui.known ? JSON.stringify(ui.known) : "",
      generation,
    );
    this.walk.selectMs = performance.now() - started;
    this.framePtr = ptr;
    // Before the views: it may grow the module's memory.
    this.waves = this.rust.waves();
    const views = { ...frameViews(buffer(), ptr), waves: this.waves };
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
    return { ...frameViews(buffer(), this.framePtr), waves: this.waves };
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
}
