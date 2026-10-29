export type ScarRegion = readonly [number, number, number, number];

/** What to write before drawing the scars (the presentation's `ScarUpload`). */
export interface ScarUpload {
  /** The directory's words (`rg32uint`), and its width and height, when it changed. */
  directory: { words: Uint32Array; width: number; height: number } | null;
  /** Pool writes, (offset, texels) pairs, each within one row of the pool. */
  writes: Uint32Array;
  /** The writes' texels in order, four bytes each. */
  texels: Uint8Array;
  /** `ScarParams`' words before the biome's look. */
  params: Uint32Array;
}

/** What the scar cache holds and what its uploads wrote. */
export interface ScarStats {
  cols: number;
  rows: number;
  cellM: number;
  uploads: number;
  fullUploads: number;
  lastBytes: number;
  totalBytes: number;
  pages: number;
  directoryEntries: number;
  poolWords: number;
  defaultWord: number;
  cacheBounds: ScarRegion;
  /** The textures' bytes on the GPU. */
  textureBytes: number;
  directoryBytes: number;
}

/** The observing side's learned ground, as the presentation caches its
 *  scars: a renderer draws it and decides nothing. */
export interface GroundMarks {
  readonly cellM: number;
  readonly cols: number;
  readonly rows: number;
  /** Whether the side has seen cell (i, j) cleared. */
  isCleared(i: number, j: number): boolean;
  /** Counts what was cleared: a change says the cleared ground changed. */
  readonly clearedCount: number;
  /** The cleared cells as tile and `start + length · 256` spans (16-cell
   *  tiles, their cells row-major). */
  clearedRuns(): Uint32Array;
  /** Whether the scars changed since the last call. */
  changed(): boolean;
  /** The regions to draw one by one, each after its own `prepare(region,
   *  true)`; none when one `prepare()` serves the view. */
  regions(): readonly ScarRegion[];
  prepare(region?: ScarRegion, clipped?: boolean): ScarUpload;
  /** This renderer holds nothing yet: the next upload writes everything. */
  resend(): void;
  stats(): Omit<ScarStats, "textureBytes" | "directoryBytes">;
}
