/** Ground queries and learned-ground observations over the presentation owner. */
import type { Presentation } from "@wasm/renderer_browser";
import type { PresentedGround } from "@packages/battle-renderer/src/terrain/terrainSurface";
import type { ScarStats } from "../render/sceneStats";
import type { GroundLayout } from "./observationView";

/** Read-only ground queries and explicit lab controls over the Rust owner. */
export function groundOf(rust: Presentation): PresentedGround {
  return {
    rebuild_fog: () => rust.rebuild_fog(),
    grass_kinds: () => rust.grass_kinds(),
    scenery_kinds: () => rust.scenery_kinds(),
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
