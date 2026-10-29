/** Ground tables and learned-ground observations over the presentation owner. */
import type { Presentation } from "@wasm/renderer_browser";
import type { PresentedGround } from "@packages/battle-renderer/src/terrain/terrainSurface";
import type { ScarStats } from "../render/sceneStats";
import type { GroundLayout } from "./observationView";

/** The ground `rust` holds, as the renderer reads its tables: a new object
 *  for each ground set, so a renderer knows to read them again. */
export function groundOf(rust: Presentation): PresentedGround {
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
    retune_grass: (rules) => rust.retune_grass(rules),
  };
}

/** The side's learned ground as the presentation keeps it
 *  (`crates/presentation/src/scars`): what its publications taught it, cell
 *  by cell. GPU scar preparation stays inside the shared renderer. */
export class LearnedGround {
  readonly cellM: number;
  readonly cols: number;
  readonly rows: number;

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

  stats() {
    return JSON.parse(this.rust.scar_stats()) as Omit<ScarStats, "textureBytes" | "directoryBytes">;
  }
}
