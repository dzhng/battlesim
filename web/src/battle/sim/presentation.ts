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
  type FallenBuilding,
  type PlacedBuildings,
  type SideBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { TEMPLATE_STATES, templateRows } from "@packages/scene-assets/src/templateLibrary";
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
 *  by the world show through it, and the ghosts. */
export interface PresentedUi {
  selected: readonly number[];
  xray: boolean;
  ghosts: readonly GhostBody[];
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
/** One drawable the frame's batches name, by `World.appearances` row. */
export interface PresentedAppearance {
  name: string;
  state: string;
  lies: number;
}

const buffer = () => memory!.buffer;

/** A map's buildings as `load_world` takes them: where they stand, and the
 *  template art they are drawn from. */
export interface PresentedBuildings {
  placed: PlacedBuildings;
  art: BuildingArt;
}

/** One ordered side-publication stream, and its presentation; or, with no
 * layout, a presentation no battle publishes into (a world of statics).
 * Retained observations are immutable; unchanged fog and static groups
 * share their views. */
export class ObservationDecoder {
  private readonly rust: Presentation;
  private last: ObservationView | null = null;
  /** The last frame's header. */
  private framePtr = 0;

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
      const { placed, art } = buildings;
      const bounds = new Float32Array(art.bounds.length * 6).fill(Number.NaN);
      art.bounds.forEach((b, m) => b && bounds.set([...b.min, ...b.max], m * 6));
      this.rust.add_buildings(
        art.bytes,
        bounds,
        JSON.stringify(placed.templates),
        placed.template,
        placed.frames,
        placed.owners,
      );
    }
    // The call may grow memory: the views are taken after it.
    const ptr = this.rust.load_world(JSON.stringify(world));
    const views = worldViews(buffer(), ptr);
    const text = new TextDecoder();
    const rows = views.appearances.u32;
    const name = (at: number, len: number) => text.decode(views.strings.subarray(at, at + len));
    return Array.from({ length: views.appearances.count }, (_, r) => {
      const o = r * APPEARANCE_WORDS;
      return {
        name: name(rows[o + Appearance.name], rows[o + Appearance.name_len]),
        state: name(rows[o + Appearance.state], rows[o + Appearance.state_len]),
        lies: rows[o + Appearance.lies],
      };
    });
  }

  /** What the side knows destroyed of the loaded buildings; returns those
   *  whose template has no rows for their state, and then changes nothing. */
  setFallen(fallen: readonly FallenBuilding[]): number[] {
    return [
      ...this.rust.set_fallen(
        Uint32Array.from(fallen, (f) => f.building),
        Uint32Array.from(fallen, (f) => TEMPLATE_STATES.indexOf(f.state)),
      ),
    ];
  }

  /** The buildings' walk as the last frame left it: chunks, coarse rows,
   *  chunks resident, rows resident, rows expanded, chunks refused room,
   *  fallen rows, fallen rows drawn per tier. Empty without buildings. */
  buildingStats(): Uint32Array {
    return this.rust.building_stats();
  }

  free() {
    this.rust.free();
  }

  /** The presentation time at `nowMs` (sim seconds), or null before any publication. */
  clock(nowMs: number): number | null {
    return this.rust.clock(nowMs / 1000) ?? null;
  }

  /** What to draw at presentation time `time` for `camera`: views over the
   *  module's memory, valid until the next call into it. */
  frame(time: number, camera: PresentedCamera, ui: PresentedUi): PresentedFrame {
    const c = camera;
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
    );
    this.framePtr = ptr;
    return frameViews(buffer(), ptr);
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

/** A frame of a world no battle publishes into: no time, no units. */
const NO_UI: PresentedUi = { selected: [], xray: false, ghosts: [] };

/** The page's static world: a presentation no battle publishes into, the
 *  map's buildings loaded into it from their template art and walked in
 *  Rust for each frame's camera. The module must be loaded
 *  (`loadPresentation`). */
export function createStaticPresentation(sun: {
  azimuth: number;
  elevation: number;
  /** `light.cascades.max_far_m`. */
  farM: number;
}) {
  let presentation: ObservationDecoder | null = null;
  let world: PresentedAppearance[] = [];
  let installed: InstalledAppearances | null = null;
  let art: BuildingArt | null = null;
  let placed: PlacedBuildings | null = null;
  let style: BuildingStyle | null = null;
  let fallen: readonly FallenBuilding[] | null = null;
  let stale = false;
  const walk = emptyBuildingWalkStats();

  /** What `side` knows destroyed, into the presentation. A template without
   *  rows for a building's known state is refused by name. */
  function setFallen(side: SideBuildings) {
    fallen = side.fallen;
    for (const b of presentation!.setFallen(side.fallen))
      templateRows(
        art!.library,
        side.placed.templates[side.placed.template[b]],
        side.fallen.find((f) => f.building === b)!.state,
      );
    walk.fallen = side.fallen.length;
  }

  function load(side: SideBuildings | null) {
    presentation?.free();
    presentation = null;
    world = [];
    Object.assign(walk, emptyBuildingWalkStats());
    if (!art || !side || !style || side.placed.template.length === 0) return;
    if (!artCovers(side.placed, art)) return;
    const started = performance.now();
    const next = new ObservationDecoder();
    try {
      world = next.loadWorld(
        {
          sun_azimuth: sun.azimuth,
          sun_elevation: sun.elevation,
          cascade_far_m: sun.farM,
          buildings: style,
        },
        new Map(),
        { placed: side.placed, art },
      );
    } catch (error) {
      next.free();
      throw error;
    }
    presentation = next;
    setFallen(side);
    walk.buildings = side.placed.template.length;
    walk.pool.capacity = style.pool_records;
    walk.buildMs = performance.now() - started;
  }

  return {
    /** Draw `side`'s buildings from `nextInstalled`'s template art under
     *  `nextStyle` (`side` null: none). Cheap when nothing changed: call
     *  every frame. A new `placed`, style or art loads the buildings again;
     *  a new `fallen` alone tells the presentation what the side knows. */
    set(
      side: SideBuildings | null,
      nextInstalled: InstalledAppearances | null,
      nextStyle: BuildingStyle,
    ) {
      if (nextInstalled !== installed) {
        installed = nextInstalled;
        const next = buildingArt(installed);
        const same =
          art?.library === next?.library &&
          (next?.bounds ?? []).every((b, i) => (b === null) === (art!.bounds[i] === null));
        if (!same) [art, stale] = [next, true];
      }
      if (stale || side?.placed !== placed || nextStyle !== style) {
        [placed, style, stale] = [side?.placed ?? null, nextStyle, false];
        load(side);
      } else if (presentation && side && side.fallen !== fallen) setFallen(side);
    },
    /** The static batches for `camera`, the drawables they name and the
     *  walk's stats; null with no buildings. Views valid until the next call
     *  into the module. */
    frame(camera: PresentedCamera): {
      frame: PresentedFrame;
      world: readonly PresentedAppearance[];
      stats: BuildingWalkStats;
    } | null {
      if (!presentation) return null;
      const started = performance.now();
      presentation.frame(0, camera, NO_UI);
      walk.selectMs = performance.now() - started;
      const [chunks, coarse, resident, used, expanded, refused, ruins, ...ruinTiers] =
        presentation.buildingStats();
      Object.assign(walk, {
        chunks,
        coarse,
        residentChunks: resident,
        expandedRows: expanded,
        refused,
        ruins,
        ruinTiers,
      });
      walk.pool.used = used;
      // The stats call may grow memory: the views are taken after it.
      const views = presentation.lastFrame();
      const batches = views.batches.u32;
      for (let b = 0; b < views.batches.count; b++)
        if (batches[b * BATCH_WORDS + Batch.first] !== NONE) {
          walk.totalExpandedRows += expanded;
          break;
        }
      return { frame: views, world, stats: walk };
    },
    dispose() {
      presentation?.free();
      presentation = null;
    },
  };
}
