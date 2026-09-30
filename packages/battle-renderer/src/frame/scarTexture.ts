// A bounded GPU cache of exact learned scar pages. CPU knowledge owns every
// learned cell; a draw loads its region and filter halo before sampling.
import type { GpuRegistry, GpuSlot } from "./registry";

export const SCAR_UNIFORM = 0x80000000;
export const SCAR_TILE = 16;
export const SCAR_HALO = 2;
export const SCAR_PAGE = SCAR_TILE + 2 * SCAR_HALO;
export const SCAR_CACHE_SIDE = 70;
export const SCAR_REGION_CELLS = 1024;
export type ScarRegion = readonly [number, number, number, number];
type GroundChanges = { all: true } | { all: false; cells: Uint32Array };

export interface GroundMarks {
  readonly cellM: number;
  readonly cols: number;
  readonly rows: number;
  readonly hasMarks: boolean;
  uniformMarks(i: number, j: number): number | null;
  scarDefault(): { word: number; exceptions: number };
  forEachScarException(
    word: number,
    visit: (i: number, j: number) => void,
    bounds?: ScarRegion,
  ): void;
  readMarks(i: number, j: number, out: Uint8Array, offset?: number): void;
  isCleared(i: number, j: number): boolean;
  forEachTile(visit: (i: number, j: number) => void, bounds?: ScarRegion): void;
  takeChanges(): GroundChanges;
}

export interface ScarTarget {
  resize(pagesSide: number, layers: number): void;
  directory(words: Uint32Array, defaultWord: number, bounds: ScarRegion): void;
  write(slot: number, marks: Uint8Array, pagesSide: number): void;
}

export interface ScarStats {
  cols: number;
  rows: number;
  cellM: number;
  lastBytes: number;
  totalBytes: number;
  uploads: number;
  fullUploads: number;
  pages: number;
  pagesSide: number;
  layers: number;
  textureBytes: number;
  directoryBytes: number;
  directoryEntries: number;
  lastTextureBytes: number;
  lastDirectoryBytes: number;
  defaultWord: number;
  cacheBounds: ScarRegion;
}

export function scarHash(key: number, mask: number): number {
  return (Math.imul(key, 0x9e3779b1) ^ (key >>> 16)) & mask;
}

/** Exact tile words share the directory with nonuniform atlas pages. */
export class ScarSync {
  private view: GroundMarks | null = null;
  private regionKey = "";
  private defaultWord = 0;
  private cacheBounds: ScarRegion = [0, 0, 0, 0];
  private pages = new Map<number, { word: number | null; slot: number }>();
  private free: number[] = [];
  private nextSlot = 0;
  private grid = { cols: 0, rows: 0, cellM: 1 };
  private pagesSide = 1;
  private words = new Uint32Array(2);
  private counts = {
    lastBytes: 0,
    totalBytes: 0,
    uploads: 0,
    fullUploads: 0,
    lastTextureBytes: 0,
    lastDirectoryBytes: 0,
  };
  private readonly pixels = new Uint8Array(SCAR_PAGE * SCAR_PAGE * 4);
  constructor(
    private readonly target: ScarTarget,
    private readonly maxPagesSide = SCAR_CACHE_SIDE,
    private readonly fixed = false,
  ) {
    if (fixed) this.pagesSide = maxPagesSide;
  }
  setGrid(ground: GroundMarks | null) {
    this.grid = ground
      ? { cols: ground.cols, rows: ground.rows, cellM: ground.cellM }
      : { cols: 0, rows: 0, cellM: 1 };
  }
  sync(
    ground: GroundMarks | null,
    region?: ScarRegion,
    margin = 2,
    published?: GroundChanges,
  ): boolean {
    const changes = published ?? ground?.takeChanges(),
      regionKey = region ? `${region.join(",")}/${margin}` : "all",
      defaultWord = ground?.scarDefault().word ?? 0;
    const reset =
      ground !== this.view ||
      changes?.all === true ||
      regionKey !== this.regionKey ||
      defaultWord !== this.defaultWord;
    if (!reset && (!ground || (changes && !changes.all && changes.cells.length === 0)))
      return false;
    this.view = ground;
    this.regionKey = regionKey;
    this.defaultWord = defaultWord;
    this.setGrid(ground);
    const tilesX = Math.ceil(this.grid.cols / SCAR_TILE),
      tilesY = Math.ceil(this.grid.rows / SCAR_TILE);
    const loX = region
      ? Math.max(0, Math.floor((region[0] / this.grid.cellM - margin) / SCAR_TILE))
      : 0;
    const loY = region
      ? Math.max(0, Math.floor((region[1] / this.grid.cellM - margin) / SCAR_TILE))
      : 0;
    const hiX = region
      ? Math.min(tilesX - 1, Math.floor((region[2] / this.grid.cellM + margin) / SCAR_TILE))
      : tilesX - 1;
    const hiY = region
      ? Math.min(tilesY - 1, Math.floor((region[3] / this.grid.cellM + margin) / SCAR_TILE))
      : tilesY - 1;
    this.cacheBounds = [loX, loY, Math.max(loX, hiX), Math.max(loY, hiY)];
    const dirty = new Set<number>();
    const neighbors = (x: number, y: number) => {
      for (let j = Math.max(loY, y - 1); j <= Math.min(hiY, y + 1); j++)
        for (let i = Math.max(loX, x - 1); i <= Math.min(hiX, x + 1); i++)
          dirty.add(j * tilesX + i);
    };
    if (reset) {
      this.pages.clear();
      this.free = [];
      this.nextSlot = 0;
      ground?.forEachScarException(
        defaultWord,
        (x, y) => dirty.add(Math.floor(y / 16) * tilesX + Math.floor(x / 16)),
        region ? [loX * 16, loY * 16, hiX * 16, hiY * 16] : undefined,
      );
    } else if (changes && !changes.all)
      for (const index of changes.cells)
        neighbors(
          Math.floor((index % this.grid.cols) / 16),
          Math.floor(Math.floor(index / this.grid.cols) / 16),
        );
    let directoryChanged = reset;
    const promoted: number[] = [];
    for (const key of [...dirty].sort((a, b) => a - b)) {
      const word =
          ground?.uniformMarks((key % tilesX) * 16, Math.floor(key / tilesX) * 16) ??
          (ground ? null : 0),
        old = this.pages.get(key);
      if (word === this.defaultWord) {
        if (old) {
          if (old.word === null) this.free.push(old.slot);
          this.pages.delete(key);
          directoryChanged = true;
        }
        continue;
      }
      if (word !== null) {
        if (old?.word === null) this.free.push(old.slot);
        if (!old || old.word !== word) {
          this.pages.set(key, { word, slot: -1 });
          directoryChanged = true;
        }
      } else if (!old || old.word !== null) {
        promoted.push(key);
        directoryChanged = true;
      }
    }
    // A publication may trade which pages vary. Reclaim all demoted slots
    // before assigning promoted pages, so tile order cannot cause overflow.
    for (const key of promoted) {
      const slot = this.free.pop() ?? this.nextSlot++;
      if (slot >= this.maxPagesSide * this.maxPagesSide)
        throw new Error("ground scar pages exceed bounded resident pool");
      this.pages.set(key, { word: null, slot });
    }
    const required = Math.min(
      this.maxPagesSide,
      2 ** Math.ceil(Math.log2(Math.max(1, Math.ceil(Math.sqrt(this.nextSlot))))),
    );
    const side = this.fixed ? this.maxPagesSide : Math.max(this.pagesSide, required),
      resized = side !== this.pagesSide;
    this.pagesSide = side;
    if (resized) this.target.resize(side, 1);
    let directoryBytes = 0;
    if (directoryChanged) {
      const capacity = 2 ** Math.ceil(Math.log2(Math.max(1, this.pages.size * 2)));
      this.words = new Uint32Array(capacity * 2);
      const mask = capacity - 1;
      for (const [key, page] of this.pages) {
        let at = scarHash(key, mask);
        while (this.words[at * 2]) at = (at + 1) & mask;
        this.words[at * 2] = ((key + 1) | (page.word === null ? 0 : SCAR_UNIFORM)) >>> 0;
        this.words[at * 2 + 1] = page.word === null ? page.slot + 1 : page.word;
      }
      this.target.directory(this.words, this.defaultWord, this.cacheBounds);
      directoryBytes = this.words.byteLength;
    }
    let textureBytes = 0;
    if (ground)
      for (const [key, page] of this.pages) {
        if (page.word !== null || (!resized && !reset && !dirty.has(key))) continue;
        const x = (key % tilesX) * 16,
          y = Math.floor(key / tilesX) * 16;
        for (let j = 0; j < SCAR_PAGE; j++)
          for (let i = 0; i < SCAR_PAGE; i++)
            ground.readMarks(
              Math.min(ground.cols - 1, Math.max(0, x + i - SCAR_HALO)),
              Math.min(ground.rows - 1, Math.max(0, y + j - SCAR_HALO)),
              this.pixels,
              (j * SCAR_PAGE + i) * 4,
            );
        this.target.write(page.slot, this.pixels, side);
        textureBytes += this.pixels.byteLength;
      }
    const bytes = textureBytes + directoryBytes;
    this.counts.lastBytes = bytes;
    this.counts.lastTextureBytes = textureBytes;
    this.counts.lastDirectoryBytes = directoryBytes;
    this.counts.totalBytes += bytes;
    this.counts.uploads++;
    if (reset || resized) this.counts.fullUploads++;
    return true;
  }
  directoryWords(): Uint32Array {
    return this.words;
  }
  stats(): ScarStats {
    return {
      ...this.grid,
      ...this.counts,
      defaultWord: this.defaultWord,
      cacheBounds: this.cacheBounds,
      pages: this.pages.size,
      pagesSide: this.pagesSide,
      layers: 1,
      textureBytes: (this.pagesSide * SCAR_PAGE) ** 2 * 4,
      directoryBytes: this.words.byteLength,
      directoryEntries: this.words.length / 2,
    };
  }
}

export function createScarTexture(registry: GpuRegistry) {
  const device = registry.device;
  const texture: GpuSlot<GPUTexture> = registry.slot();
  const directory: GpuSlot<GPUTexture> = registry.slot();
  texture.set(
    device.createTexture({
      label: "ground-scar-pages",
      size: [SCAR_CACHE_SIDE * SCAR_PAGE, SCAR_CACHE_SIDE * SCAR_PAGE, 1],
      format: "rgba8unorm",
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
    }),
  );
  directory.set(
    device.createTexture({
      label: "ground-scar-directory",
      size: [1024, 16],
      format: "rg32uint",
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
    }),
  );
  const sync = new ScarSync(
    {
      resize() {},
      directory(words) {
        const capacity = words.length / 2;
        if (capacity > 16384) throw new Error("ground scar directory exceeds bounded cache");
        const width = Math.min(1024, capacity),
          height = capacity / width;
        device.queue.writeTexture(
          { texture: directory.current! },
          words,
          { bytesPerRow: width * 8, rowsPerImage: height },
          [width, height],
        );
      },
      write(slot, marks, side) {
        const perLayer = side * side,
          page = slot % perLayer;
        device.queue.writeTexture(
          {
            texture: texture.current!,
            origin: [
              (page % side) * SCAR_PAGE,
              Math.floor(page / side) * SCAR_PAGE,
              Math.floor(slot / perLayer),
            ],
          },
          marks,
          { bytesPerRow: SCAR_PAGE * 4, rowsPerImage: SCAR_PAGE },
          [SCAR_PAGE, SCAR_PAGE],
        );
      },
    },
    Math.min(SCAR_CACHE_SIDE, Math.floor(device.limits.maxTextureDimension2D / SCAR_PAGE)),
    true,
  );
  let ground: GroundMarks | null = null;
  let pending: GroundChanges = { all: false, cells: new Uint32Array() };
  let hasMarks = false;
  let globalEligible = true;
  return {
    setGround(next: GroundMarks | null) {
      const changes = next?.takeChanges();
      const changed =
        next !== ground ||
        changes?.all === true ||
        (!!changes && !changes.all && changes.cells.length > 0);
      const replaced = next !== ground;
      ground = next;
      sync.setGrid(next);
      if (changes?.all || replaced) pending = { all: true };
      else if (changes && !changes.all && !pending.all) {
        const cells = new Uint32Array(pending.cells.length + changes.cells.length);
        cells.set(pending.cells);
        cells.set(changes.cells, pending.cells.length);
        pending = cells.length > 65536 ? { all: true } : { all: false, cells };
      }
      if (changed) {
        hasMarks = next?.hasMarks ?? false;
        globalEligible = !next || next.scarDefault().exceptions <= 4096;
      }
      return changed;
    },
    prepare(region?: ScarRegion, margin = 2) {
      const changed = sync.sync(ground, globalEligible ? undefined : region, margin, pending);
      pending = { all: false, cells: new Uint32Array() };
      return changed;
    },
    hasMarks: () => hasMarks,
    globalEligible: () => globalEligible,
    get texture() {
      return texture.current!;
    },
    get directory() {
      return directory.current!;
    },
    stats: () => ({ ...sync.stats(), directoryBytes: 1024 * 16 * 8 }),
  };
}
export type ScarTexture = ReturnType<typeof createScarTexture>;
