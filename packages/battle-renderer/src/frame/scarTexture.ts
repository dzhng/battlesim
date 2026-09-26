// The side's learned ground on the GPU (battle-look slice 17): one rgba8
// texel per ground cell (crater, scorch, tracks, trampled), a copy of the
// client's `GroundView` that the terrain material and the grass build sample.
// It is only ever what the side has learned: the view holds nothing else.
//
// Uploads follow the view's changes: a new view (a new client, a rebuilt
// frame) or a snapshot (a new epoch, a side switch) writes the whole grid
// once; a delta writes only the 16 × 16-cell tiles holding a changed cell,
// merged along each tile row. Nothing is re-uploaded while nothing changes.
import type { GpuRegistry, GpuSlot } from "./registry";

/** Cells per upload tile edge (the simulation's ground tile). */
export const SCAR_TILE = 16;

/** A side's learned ground as the frame draws it. The client's `GroundView`
 *  is one: the frame reads its marks and takes its changes. */
export interface GroundMarks {
  readonly cellM: number;
  readonly cols: number;
  readonly rows: number;
  /** Four bytes per cell, row-major (`j * cols + i`): crater, scorch,
   *  tracks, trampled, each in [0, 255]. */
  readonly marks: Uint8Array;
  /** Cells changed since the last call: everything, or the exact list. */
  takeChanges(): { all: true } | { all: false; cells: Uint32Array };
}

/** A rect of cells: `x, y` its first column and row, `w × h` its size. */
export interface ScarRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** The rects that carry `cells` (row-major indices on a `cols × rows` grid):
 *  every tile holding one, merged with its neighbours along a tile row, in
 *  row order. */
export function scarUploadRects(cells: Uint32Array, cols: number, rows: number): ScarRect[] {
  const tilesX = Math.ceil(cols / SCAR_TILE);
  const dirty = new Set<number>();
  for (const cell of cells) {
    const i = cell % cols;
    const j = (cell - i) / cols;
    dirty.add(Math.floor(j / SCAR_TILE) * tilesX + Math.floor(i / SCAR_TILE));
  }
  const tiles = [...dirty].sort((a, b) => a - b);
  const rects: ScarRect[] = [];
  for (let k = 0; k < tiles.length; ) {
    const ty = Math.floor(tiles[k] / tilesX);
    const tx = tiles[k] % tilesX;
    let run = 1;
    while (k + run < tiles.length && tiles[k + run] === tiles[k] + run && tx + run < tilesX) run++;
    const x = tx * SCAR_TILE;
    const y = ty * SCAR_TILE;
    rects.push({
      x,
      y,
      w: Math.min(cols, x + run * SCAR_TILE) - x,
      h: Math.min(rows, y + SCAR_TILE) - y,
    });
    k += run;
  }
  return rects;
}

/** Where scar uploads land: the texture, sized for a grid. */
export interface ScarTarget {
  /** A texture for a `cols × rows` grid, every texel zero. */
  resize(cols: number, rows: number): void;
  /** Copy `rect` of `marks` (row-major, `cols` cells wide) into the texture. */
  write(marks: Uint8Array, cols: number, rect: ScarRect): void;
}

export interface ScarStats {
  /** The grid the texture holds (0 × 0 without ground), and its cell size. */
  cols: number;
  rows: number;
  cellM: number;
  /** Bytes the last upload wrote, and every upload so far. */
  lastBytes: number;
  totalBytes: number;
  /** Uploads so far, and how many of them were the whole grid. */
  uploads: number;
  fullUploads: number;
}

/** Keeps a target in step with the side's learned ground. */
export class ScarSync {
  private view: GroundMarks | null = null;
  private grid = { cols: 0, rows: 0, cellM: 1 };
  private counts = { lastBytes: 0, totalBytes: 0, uploads: 0, fullUploads: 0 };

  constructor(private readonly target: ScarTarget) {}

  /** Follow `ground` (null: none). Returns whether the texture changed. */
  sync(ground: GroundMarks | null): boolean {
    if (ground !== this.view) {
      this.view = ground;
      const { cols, rows, cellM } = ground ?? { cols: 0, rows: 0, cellM: 1 };
      this.grid = { cols, rows, cellM };
      this.target.resize(cols, rows);
      if (!ground) return true;
      // A view new to this texture: whatever it reports is already covered.
      ground.takeChanges();
      this.upload(ground, [{ x: 0, y: 0, w: cols, h: rows }], true);
      return true;
    }
    if (!ground) return false;
    const changes = ground.takeChanges();
    if (changes.all) {
      this.upload(ground, [{ x: 0, y: 0, w: ground.cols, h: ground.rows }], true);
      return true;
    }
    if (changes.cells.length === 0) return false;
    this.upload(ground, scarUploadRects(changes.cells, ground.cols, ground.rows), false);
    return true;
  }

  stats(): ScarStats {
    return { ...this.grid, ...this.counts };
  }

  private upload(ground: GroundMarks, rects: ScarRect[], full: boolean) {
    let bytes = 0;
    for (const rect of rects) {
      if (rect.w <= 0 || rect.h <= 0) continue;
      this.target.write(ground.marks, ground.cols, rect);
      bytes += rect.w * rect.h * 4;
    }
    this.counts.lastBytes = bytes;
    this.counts.totalBytes += bytes;
    this.counts.uploads++;
    if (full) this.counts.fullUploads++;
  }
}

/** The scar texture on the device: rgba8unorm, one texel per cell, at least
 *  1 × 1 (a binding cannot be empty). `onResize` runs when the texture is
 *  replaced, so its bind groups can follow. */
export function createScarTexture(registry: GpuRegistry, onResize: () => void) {
  const device = registry.device;
  const slot: GpuSlot<GPUTexture> = registry.slot();
  const allocate = (cols: number, rows: number) =>
    slot.set(
      device.createTexture({
        label: "ground-scars",
        size: [Math.max(1, cols), Math.max(1, rows)],
        format: "rgba8unorm",
        usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
      }),
    );
  allocate(1, 1);
  const sync = new ScarSync({
    resize(cols, rows) {
      allocate(cols, rows);
      onResize();
    },
    write(marks, cols, rect) {
      device.queue.writeTexture(
        { texture: slot.current!, origin: [rect.x, rect.y] },
        marks,
        { offset: (rect.y * cols + rect.x) * 4, bytesPerRow: cols * 4, rowsPerImage: rect.h },
        [rect.w, rect.h],
      );
    },
  });
  return {
    get texture() {
      return slot.current!;
    },
    sync: (ground: GroundMarks | null) => sync.sync(ground),
    stats: () => sync.stats(),
  };
}
export type ScarTexture = ReturnType<typeof createScarTexture>;
