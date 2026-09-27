/** A side's learned ground, rebuilt on the main thread from the ground
 * patches its publications carry (slice 08).
 *
 * The simulation keeps the authoritative ground layer (craters, scorch, track
 * wear, trampling in 1 m cells) and, per side, the cells that side has seen.
 * Each publication carries a patch of that side's learned cells: a stream
 * (an epoch) opens with a full snapshot, then carries only cells changed
 * since the previous patch. The client applies every patch here before the
 * publication's buffer goes back to the producer, so this view is always the
 * side's knowledge as of the latest publication. */
import type { GroundLayout, GroundPatchView } from "./observation";

/** The four mark channels, in the byte order of `GroundView.marks`. */
export const GROUND_CHANNELS = ["crater", "scorch", "tracks", "trampled"] as const;
export type GroundChannel = (typeof GROUND_CHANNELS)[number];

/** Changed cells since the last `takeChanges`: every cell after a snapshot,
 *  otherwise the exact list (possibly repeating a cell). */
export type GroundChanges = { all: true } | { all: false; cells: Uint32Array };

export class GroundView {
  readonly cellM: number;
  readonly cols: number;
  readonly rows: number;
  /** Four bytes per cell, row-major (index `j * cols + i`): crater, scorch,
   *  tracks, trampled, each in [0, 255]. Laid out as an RGBA8 texture. */
  readonly marks: Uint8Array;
  /** One byte per cell: 255 where the side has seen trees knocked flat. */
  readonly cleared: Uint8Array;
  /** How many cells `cleared` marks: a cheap key for what follows it (the
   *  drawn fog's foliage). */
  clearedCount = 0;
  /** The stream the view follows; 0 before the first snapshot. */
  epoch = 0;
  /** The side whose ground this is, once a snapshot arrived. */
  side: string | null = null;
  /** The side's knowledge revision the view holds. */
  revision = 0;
  /** Epochs at or below this are stale (set by `invalidate`). */
  private floor = 0;
  private changed: number[] = [];
  private changedAll = false;

  constructor(layout: GroundLayout) {
    this.cellM = layout.cellM;
    this.cols = layout.cols;
    this.rows = layout.rows;
    this.marks = new Uint8Array(layout.cols * layout.rows * 4);
    this.cleared = new Uint8Array(layout.cols * layout.rows);
  }

  /** Apply the next patch. A patch from an older (or invalidated) epoch is
   *  dropped as "stale". A new epoch must open with a full snapshot, and a
   *  delta must start at the view's revision: anything else means the
   *  stream lost a patch, which the ordered transport never does, so it
   *  throws. */
  apply(patch: GroundPatchView): "applied" | "stale" {
    if (patch.epoch < this.epoch || patch.epoch <= this.floor) return "stale";
    if (patch.epoch > this.epoch) {
      if (!patch.full)
        throw new Error(`ground epoch ${patch.epoch} opened without a full snapshot`);
      this.marks.fill(0);
      this.cleared.fill(0);
      this.clearedCount = 0;
      this.epoch = patch.epoch;
      this.side = patch.side;
      this.changed = [];
      this.changedAll = true;
    } else if (patch.full || patch.baseRevision !== this.revision) {
      throw new Error(
        `ground patch ${patch.baseRevision}→${patch.revision} does not follow revision ${this.revision}`,
      );
    }
    for (let n = 0; n < patch.cells.length; n++) {
      this.marks.set(patch.marks.subarray(n * 4, n * 4 + 4), patch.cells[n] * 4);
      const was = this.cleared[patch.cells[n]] > 0;
      this.cleared[patch.cells[n]] = patch.cleared[n];
      this.clearedCount += Number(patch.cleared[n] > 0) - Number(was);
    }
    if (!this.changedAll) {
      for (const cell of patch.cells) this.changed.push(cell);
      // Bounded: past a sixteenth of the map, re-reading everything is cheaper.
      if (this.changed.length > (this.cols * this.rows) / 16) {
        this.changed = [];
        this.changedAll = true;
      }
    }
    this.revision = patch.revision;
    return "applied";
  }

  /** Forget everything and wait for a new epoch's snapshot: patches still
   *  in flight from the current epoch are dropped as stale. */
  invalidate() {
    this.floor = this.epoch;
    this.marks.fill(0);
    this.cleared.fill(0);
    this.clearedCount = 0;
    this.side = null;
    this.revision = 0;
    this.changed = [];
    this.changedAll = true;
  }

  /** The marks of cell (i, j), or of the cell holding world (x, y). */
  cell(i: number, j: number): Record<GroundChannel, number> {
    const k = (j * this.cols + i) * 4;
    const m = this.marks;
    return { crater: m[k], scorch: m[k + 1], tracks: m[k + 2], trampled: m[k + 3] };
  }

  at(x: number, y: number): Record<GroundChannel, number> | null {
    const [i, j] = [Math.floor(x / this.cellM), Math.floor(y / this.cellM)];
    return i < 0 || j < 0 || i >= this.cols || j >= this.rows ? null : this.cell(i, j);
  }

  /** Visit every cell with any mark, row by row. */
  forEachMarked(visit: (i: number, j: number, index: number) => void) {
    const words = new Uint32Array(this.marks.buffer, this.marks.byteOffset, this.cols * this.rows);
    for (let k = 0; k < words.length; k++) {
      if (words[k] !== 0) visit(k % this.cols, Math.floor(k / this.cols), k);
    }
  }

  /** Cells changed since the last call (a renderer's dirty uploads). */
  takeChanges(): GroundChanges {
    const out: GroundChanges = this.changedAll
      ? { all: true }
      : { all: false, cells: Uint32Array.from(this.changed) };
    this.changed = [];
    this.changedAll = false;
    return out;
  }
}
