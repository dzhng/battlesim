/** The side's learned ground. Missing tiles are blank; epochs replace the
 * learned copy, and deltas retain exactly the cells the side last saw. */
import type { GroundLayout } from "./observation";
import {
  createGroundPage,
  createGroundPageEdits,
  applyGroundPageEdits,
  readGroundPage,
  groundPageCleared,
  groundPageMarked,
  projectedGroundUniform,
  clearedGroundSpans,
  type GroundPage,
} from "./groundPage";
export interface GroundRunsPatch {
  epoch: number;
  side: string;
  baseRevision: number;
  revision: number;
  full: boolean;
  /** tile ID; local start+length*256; crater+scorch*256; tracks+trampled*256+cleared*65536. */
  runs: Float32Array;
}

export const GROUND_CHANNELS = ["crater", "scorch", "tracks", "trampled"] as const;
export type GroundChannel = (typeof GROUND_CHANNELS)[number];
export type GroundChanges = { all: true } | { all: false; cells: Uint32Array };
export const GROUND_TILE = 16;

export class GroundView {
  readonly cellM: number;
  readonly cols: number;
  readonly rows: number;
  private readonly tilesX: number;
  private readonly tiles = new Map<number, GroundPage>();
  private bytes = 0;
  private markedPages = 0;
  private readonly sample = new Uint8Array(5);
  private readonly edits = createGroundPageEdits();
  clearedCount = 0;
  epoch = 0;
  side: string | null = null;
  revision = 0;
  private floor = 0;
  private changed: number[] = [];
  private changedAll = false;

  constructor(layout: Pick<GroundLayout, "cellM" | "cols" | "rows">) {
    this.cellM = layout.cellM;
    this.cols = layout.cols;
    this.rows = layout.rows;
    this.tilesX = Math.ceil(this.cols / GROUND_TILE);
  }

  get hasMarks(): boolean {
    return this.markedPages > 0;
  }

  /** Payload bytes; map bookkeeping is measured separately by resource probes. */
  get byteLength(): number {
    return this.bytes;
  }

  private begin(
    patch: Pick<GroundRunsPatch, "epoch" | "side" | "baseRevision" | "revision" | "full">,
  ): boolean {
    if (patch.epoch < this.epoch || patch.epoch <= this.floor) return false;
    if (patch.epoch > this.epoch) {
      if (!patch.full)
        throw new Error(`ground epoch ${patch.epoch} opened without a full snapshot`);
      this.tiles.clear();
      this.bytes = 0;
      this.markedPages = 0;
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
    return true;
  }

  private put(tile: number, start: number, len: number, word: number, clear: number) {
    let page = this.tiles.get(tile);
    if (!page) {
      page = createGroundPage();
      this.tiles.set(tile, page);
      this.bytes += page.data.byteLength;
    }
    if (this.edits.count === 256) this.finishTile(tile);
    const k = this.edits.count++;
    this.edits.starts[k] = start;
    this.edits.lens[k] = len;
    this.edits.words[k] = word >>> 0;
    this.edits.clears[k] = clear;
    if (!this.changedAll) {
      if (this.changed.length + len > Math.min(65536, (this.cols * this.rows) / 16)) {
        this.changed = [];
        this.changedAll = true;
        return;
      }
      const x = (tile % this.tilesX) * GROUND_TILE,
        y = Math.floor(tile / this.tilesX) * GROUND_TILE;
      for (let c = start; c < start + len; c++)
        this.changed.push((y + Math.floor(c / GROUND_TILE)) * this.cols + x + (c % GROUND_TILE));
    }
  }

  private finishTile(tile: number) {
    if (!this.edits.count) return;
    const page = this.tiles.get(tile)!;
    const before = page.data.byteLength;
    const marked = groundPageMarked(page);
    this.clearedCount += applyGroundPageEdits(page, this.edits);
    this.markedPages += Number(groundPageMarked(page)) - Number(marked);
    page.uniform = projectedGroundUniform(
      page,
      Math.min(16, this.cols - (tile % this.tilesX) * 16),
      Math.min(16, this.rows - Math.floor(tile / this.tilesX) * 16),
    );
    this.bytes += page.data.byteLength - before;
    this.edits.count = 0;
  }

  /** Consume validated packed runs directly; no per-cell publication copy. */
  applyRuns(patch: GroundRunsPatch): "applied" | "stale" {
    if (!this.begin(patch)) return "stale";
    let prior = -1;
    for (let at = 0; at < patch.runs.length; at += 4) {
      const tile = patch.runs[at];
      if (tile !== prior) this.finishTile(prior);
      prior = tile;
      const span = patch.runs[at + 1],
        craterScorch = patch.runs[at + 2],
        rest = patch.runs[at + 3];
      this.put(
        patch.runs[at],
        span % 256,
        Math.floor(span / 256),
        (craterScorch | ((rest & 65535) << 16)) >>> 0,
        rest >>> 16,
      );
    }
    this.finishTile(prior);
    this.revision = patch.revision;
    return "applied";
  }

  invalidate() {
    this.floor = this.epoch;
    this.tiles.clear();
    this.bytes = 0;
    this.markedPages = 0;
    this.clearedCount = 0;
    this.side = null;
    this.revision = 0;
    this.changed = [];
    this.changedAll = true;
  }

  /** Copy the four mark channels into caller-owned bytes. */
  readMarks(i: number, j: number, out: Uint8Array, offset = 0): void {
    const key = Math.floor(j / GROUND_TILE) * this.tilesX + Math.floor(i / GROUND_TILE);
    const tile = i >= 0 && j >= 0 && i < this.cols && j < this.rows ? this.tiles.get(key) : null;
    if (tile)
      readGroundPage(tile, (j % GROUND_TILE) * GROUND_TILE + (i % GROUND_TILE), this.sample);
    for (let c = 0; c < 4; c++) out[offset + c] = tile ? this.sample[c] : 0;
    // Clearing is presented as complete track wear; retained run bytes stay raw.
    if (tile) out[offset + 2] = Math.max(out[offset + 2], this.sample[4]);
  }

  /** Exact visible-channel uniformity for one tile; missing tiles are zero. */
  uniformMarks(i: number, j: number): number | null {
    const page = this.tiles.get(Math.floor(j / 16) * this.tilesX + Math.floor(i / 16));
    return page ? page.uniform : 0;
  }

  isCleared(i: number, j: number): boolean {
    if (i < 0 || j < 0 || i >= this.cols || j >= this.rows) return false;
    const key = Math.floor(j / GROUND_TILE) * this.tilesX + Math.floor(i / GROUND_TILE);
    const page = this.tiles.get(key);
    return !!page && groundPageCleared(page, (j % GROUND_TILE) * GROUND_TILE + (i % GROUND_TILE));
  }

  cell(i: number, j: number): Record<GroundChannel, number> {
    this.readMarks(i, j, this.sample);
    return {
      crater: this.sample[0],
      scorch: this.sample[1],
      tracks: this.sample[2],
      trampled: this.sample[3],
    };
  }

  at(x: number, y: number): Record<GroundChannel, number> | null {
    const [i, j] = [Math.floor(x / this.cellM), Math.floor(y / this.cellM)];
    return i < 0 || j < 0 || i >= this.cols || j >= this.rows ? null : this.cell(i, j);
  }

  /** Allocated tiles, in global tile order. Consumers never scan untouched cells. */
  forEachTile(
    visit: (i: number, j: number) => void,
    bounds?: readonly [number, number, number, number],
  ): void {
    if (bounds) {
      const x0 = Math.max(0, Math.floor(bounds[0] / GROUND_TILE)),
        y0 = Math.max(0, Math.floor(bounds[1] / GROUND_TILE));
      const x1 = Math.min(this.tilesX - 1, Math.floor(bounds[2] / GROUND_TILE)),
        y1 = Math.min(Math.ceil(this.rows / GROUND_TILE) - 1, Math.floor(bounds[3] / GROUND_TILE));
      for (let y = y0; y <= y1; y++)
        for (let x = x0; x <= x1; x++)
          if (this.tiles.has(y * this.tilesX + x)) visit(x * GROUND_TILE, y * GROUND_TILE);
      return;
    }
    for (const key of [...this.tiles.keys()].sort((a, b) => a - b))
      visit((key % this.tilesX) * GROUND_TILE, Math.floor(key / this.tilesX) * GROUND_TILE);
  }

  forEachMarked(visit: (i: number, j: number, index: number) => void): void {
    const keys = [...this.tiles.keys()]
      .filter((key) => groundPageMarked(this.tiles.get(key)!))
      .sort((a, b) => a - b);
    for (let lo = 0; lo < keys.length; ) {
      const row = Math.floor(keys[lo] / this.tilesX);
      let hi = lo + 1;
      while (hi < keys.length && Math.floor(keys[hi] / this.tilesX) === row) hi++;
      for (let y = 0; y < GROUND_TILE && row * GROUND_TILE + y < this.rows; y++)
        for (let k = lo; k < hi; k++) {
          const page = this.tiles.get(keys[k])!;
          const x0 = (keys[k] % this.tilesX) * GROUND_TILE,
            j = row * GROUND_TILE + y;
          for (let x = 0; x < GROUND_TILE && x0 + x < this.cols; x++) {
            readGroundPage(page, y * GROUND_TILE + x, this.sample);
            if (
              this.sample[0] ||
              this.sample[1] ||
              this.sample[2] ||
              this.sample[3] ||
              this.sample[4]
            )
              visit(x0 + x, j, j * this.cols + x0 + x);
          }
        }
      lo = hi;
    }
  }

  /** Tile/span pairs for the exact learned clear predicate. Forest queries
   * consume these spans directly rather than rebuilding every cleared cell. */
  clearedRuns(): Uint32Array {
    const keys = [...this.tiles.keys()].sort((a, b) => a - b);
    let count = 0;
    for (const key of keys) clearedGroundSpans(this.tiles.get(key)!, () => count++);
    const out = new Uint32Array(count * 2);
    let at = 0;
    for (const key of keys)
      clearedGroundSpans(this.tiles.get(key)!, (start, len) => {
        out[at++] = key;
        out[at++] = start + len * 256;
      });
    return out;
  }

  takeChanges(): GroundChanges {
    const out: GroundChanges = this.changedAll
      ? { all: true }
      : { all: false, cells: Uint32Array.from(this.changed) };
    this.changed = [];
    this.changedAll = false;
    return out;
  }
}
