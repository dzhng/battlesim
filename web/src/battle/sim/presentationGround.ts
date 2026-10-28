/** Ground table and learned-scar bridges over the presentation WASM owner. */
import type { Presentation } from "@wasm/presentation/presentation_wasm.js";
import type { PresentedGround } from "@packages/battle-renderer/src/terrain/terrainSurface";
import type {
  GroundMarks,
  ScarRegion,
  ScarUpload,
} from "@packages/battle-renderer/src/frame/scarTexture";
import type { GroundLayout } from "./observationView";

export type SceneGround = PresentedGround &
  Pick<
    Presentation,
    | "grass_capacity"
    | "grass_window_record"
    | "grass_params_from_facts"
    | "scars_changed"
    | "scar_regions"
    | "prepare_scars"
    | "resend_scars"
  >;

/** The ground `rust` holds, as the renderer reads its tables: a new object
 *  for each ground set, so a renderer knows to read them again. */
export function groundOf(rust: Presentation): SceneGround {
  return {
    prepare_fog: (camera, width, height, probes) => rust.prepare_fog(camera, width, height, probes),
    resend_fog: () => rust.resend_fog(),
    rebuild_fog: () => rust.rebuild_fog(),
    fog_stats: () => rust.fog_stats(),
    fog_occluders: () => rust.fog_occluders(),
    pages: () => rust.pages(),
    ground_terrain: () => rust.ground_terrain(),
    ground_plots: (textured) => rust.ground_plots(textured),
    ground_water_mesh: () => rust.ground_water_mesh(),
    grass_kinds: () => rust.grass_kinds(),
    grass_params: (shapes, density) => rust.grass_params(shapes, density),
    grass_window: () => rust.grass_window(),
    grass_frame_params: (words, window) => rust.grass_frame_params(words, window),
    grass_props: () => rust.grass_props(),
    grass_capacity: () => rust.grass_capacity(),
    grass_window_record: () => rust.grass_window_record(),
    grass_params_from_facts: (facts, density) => rust.grass_params_from_facts(facts, density),
    retune_grass: (rules) => rust.retune_grass(rules),
    scars_changed: () => rust.scars_changed(),
    scar_regions: () => rust.scar_regions(),
    prepare_scars: (region, clipped) => rust.prepare_scars(region, clipped),
    resend_scars: () => rust.resend_scars(),
  };
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
