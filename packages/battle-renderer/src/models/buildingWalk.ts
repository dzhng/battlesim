import { TIER_COUNT } from "@packages/scene-assets/src/schema";
/** What the presentation says of its buildings and their walk
 *  (`Presentation::building_stats`). */
export interface BuildingWalkStats {
  /** Buildings drawn from template art, and those of them the side knows
   *  destroyed (collapsed or gutted). */
  buildings: number;
  fallen: number;
  /** Chunks they are bucketed in, their coarse rows, and the chunks resident
   *  in the pool. */
  chunks: number;
  coarse: number;
  residentChunks: number;
  /** Bounds tested by the last spatial query, including hierarchy branches. */
  visitedBounds: number;
  /** The pool: the rows it can hold, and holds now. */
  pool: { capacity: number; used: number };
  /** The fallen buildings' rows (a ruin's or a gutted shell's), and those
   *  drawn into the view per tier. */
  ruins: number;
  ruinTiers: number[];
  /** The last walk: rows expanded into the pool, near chunks drawn coarse
   *  for want of room, and the time the frame took in Rust. */
  expandedRows: number;
  refused: number;
  selectMs: number;
  /** Since the buildings were loaded: rows expanded, and how long loading took. */
  totalExpandedRows: number;
  buildMs: number;
}

export const emptyBuildingWalkStats = (): BuildingWalkStats => ({
  buildings: 0,
  fallen: 0,
  chunks: 0,
  coarse: 0,
  residentChunks: 0,
  visitedBounds: 0,
  pool: { capacity: 0, used: 0 },
  ruins: 0,
  ruinTiers: Array.from({ length: TIER_COUNT }, () => 0),
  expandedRows: 0,
  refused: 0,
  selectMs: 0,
  totalExpandedRows: 0,
  buildMs: 0,
});
