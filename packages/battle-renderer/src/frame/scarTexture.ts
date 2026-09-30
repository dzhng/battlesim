// A bounded GPU cache of exact learned scar pages. CPU knowledge owns every
// learned cell; a draw loads its region and filter halo before sampling.
import type { GpuRegistry, GpuSlot } from "./registry";
import { ScarPool } from "./scarPool";

export const SCAR_UNIFORM = 0x80000000;
export const SCAR_RUNS = 0x40000000;
export const SCAR_DENSE = 0x20000000;
export const SCAR_KEY_MASK = 0x1fffffff;
export const SCAR_DIRECTORY_CAPACITY = 2 ** 22;
export const SCAR_POOL_WIDTH = 2048;
export const SCAR_POOL_HEIGHT = 1024;
export const SCAR_POOL_WORDS = SCAR_POOL_WIDTH * SCAR_POOL_HEIGHT;
export const SCAR_NORMALIZATION_WORDS = 4096;
// A returned f32 texel position retains every Q8 quantum up to 2^24 / 256.
// Wider grids need an integer cell/fraction result, not this sampling source.
const MAX_Q8_AXIS = 2 ** 24 / 256;
export const SCAR_TILE = 16;
export const SCAR_REGION_CELLS = 1024;
export type ScarRegion = readonly [number, number, number, number];
type GroundChanges = { all: true } | { all: false; cells: Uint32Array };

export interface GroundMarks {
  readonly cellM: number;
  readonly cols: number;
  readonly rows: number;
  readonly hasMarks: boolean;
  uniformMarks(i: number, j: number): number | null;
  scarDefault(): { word: number; exceptions: number; poolWords: number };
  readScarRuns(i: number, j: number, out: Uint32Array): number;
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
  resize(width: number, height: number): void;
  directory(words: Uint32Array, defaultWord: number, bounds: ScarRegion): void;
  write(offset: number, marks: Uint8Array, width: number): void;
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
  textureBytes: number;
  directoryBytes: number;
  directoryEntries: number;
  lastTextureBytes: number;
  lastDirectoryBytes: number;
  defaultWord: number;
  cacheBounds: ScarRegion;
  poolWidth: number;
  poolHeight: number;
  poolWords: number;
  initializationBytes: number;
}

export function scarHash(key: number, mask: number): number {
  return (Math.imul(key, 0x9e3779b1) ^ (key >>> 16)) & mask;
}

/** Exact uniform directory words and lossless runs/dense words share one cache. */
export class ScarSync {
  private view: GroundMarks | null = null;
  private regionKey = "";
  private defaultWord = 0;
  private cacheBounds: ScarRegion = [0, 0, 0, 0];
  private pages = new Map<
    number,
    { word: number | null; offset: number; words: number; count: number; dense: boolean }
  >();
  private pool: ScarPool;
  private grid = { cols: 0, rows: 0, cellM: 1 };
  private width = 4;
  private height = 4;
  private residentWords = 0;
  private extent = 16;
  private words = new Uint32Array(2);
  private readonly capacity: number;
  private readonly maxWidth: number;
  private readonly maxHeight: number;
  private readonly runs = new Uint32Array(512);
  private readonly pixels = new Uint8Array(256 * 4);
  private counts = {
    lastBytes: 0,
    totalBytes: 0,
    uploads: 0,
    fullUploads: 0,
    lastTextureBytes: 0,
    lastDirectoryBytes: 0,
  };
  constructor(
    private readonly target: ScarTarget,
    maxWords = SCAR_POOL_WORDS,
    private readonly reservedWords = 0,
    private readonly fixed = false,
  ) {
    this.capacity = maxWords;
    this.maxWidth = 2 ** Math.ceil(Math.log2(Math.sqrt(this.capacity)));
    this.maxHeight = this.capacity / this.maxWidth;
    this.pool = this.freshPool();
    if (fixed) {
      this.width = this.maxWidth;
      this.height = this.maxHeight;
    }
  }
  private freshPool(): ScarPool {
    const pool = new ScarPool(this.capacity);
    if (this.reservedWords && pool.allocate(this.reservedWords) !== 0)
      throw new Error("ground normalization reservation must start the word pool");
    this.residentWords = this.reservedWords;
    this.extent = Math.max(16, this.reservedWords);
    return pool;
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
    if (!reset && (!ground || (changes && !changes.all && !changes.cells.length))) return false;
    this.view = ground;
    this.regionKey = regionKey;
    this.defaultWord = defaultWord;
    this.setGrid(ground);
    const tilesX = Math.ceil(this.grid.cols / 16),
      tilesY = Math.ceil(this.grid.rows / 16);
    const loX = region ? Math.max(0, Math.floor((region[0] / this.grid.cellM - margin) / 16)) : 0,
      loY = region ? Math.max(0, Math.floor((region[1] / this.grid.cellM - margin) / 16)) : 0;
    const hiX = region
        ? Math.min(tilesX - 1, Math.floor((region[2] / this.grid.cellM + margin) / 16))
        : tilesX - 1,
      hiY = region
        ? Math.min(tilesY - 1, Math.floor((region[3] / this.grid.cellM + margin) / 16))
        : tilesY - 1;
    this.cacheBounds = [loX, loY, Math.max(loX, hiX), Math.max(loY, hiY)];
    const dirty = new Set<number>();
    if (reset) {
      this.pages.clear();
      this.pool = this.freshPool();
      ground?.forEachScarException(
        defaultWord,
        (x, y) => dirty.add(Math.floor(y / 16) * tilesX + Math.floor(x / 16)),
        region ? [loX * 16, loY * 16, hiX * 16, hiY * 16] : undefined,
      );
    } else if (changes && !changes.all)
      for (const index of changes.cells) {
        const x = Math.floor((index % this.grid.cols) / 16),
          y = Math.floor(Math.floor(index / this.grid.cols) / 16);
        if (x >= loX && x <= hiX && y >= loY && y <= hiY) dirty.add(y * tilesX + x);
      }
    let directoryChanged = reset;
    const promoted: number[] = [];
    for (const key of [...dirty].sort((a, b) => a - b)) {
      const word =
          ground?.uniformMarks((key % tilesX) * 16, Math.floor(key / tilesX) * 16) ??
          (ground ? null : 0),
        old = this.pages.get(key);
      if (word === defaultWord) {
        if (old) {
          if (old.word === null) this.release(old.offset, old.words);
          this.pages.delete(key);
          directoryChanged = true;
        }
        continue;
      }
      if (word !== null) {
        if (old?.word === null) this.release(old.offset, old.words);
        if (!old || old.word !== word) {
          this.pages.set(key, { word, offset: 0, words: 0, count: 0, dense: false });
          directoryChanged = true;
        }
        continue;
      }
      const count = ground!.readScarRuns(
          (key % tilesX) * 16,
          Math.floor(key / tilesX) * 16,
          this.runs,
        ),
        dense = count > 128,
        words = 2 ** Math.ceil(Math.log2(Math.max(16, dense ? 256 : count * 2)));
      if (old?.word === null && old.words === words) {
        if (old.count !== count || old.dense !== dense) directoryChanged = true;
        old.count = count;
        old.dense = dense;
      } else {
        if (old?.word === null) this.release(old.offset, old.words);
        this.pages.set(key, { word: null, offset: -1, words, count, dense });
        promoted.push(key);
        directoryChanged = true;
      }
    }
    let compacted = false;
    if (
      this.residentWords + promoted.reduce((n, key) => n + this.pages.get(key)!.words, 0) >
      this.capacity
    )
      throw new Error("ground scar words exceed bounded resident pool");
    for (const key of promoted) {
      const page = this.pages.get(key)!;
      page.offset = this.pool.allocate(page.words);
      if (page.offset < 0) {
        // Free words may be scattered by churn. Repack largest blocks first:
        // power-of-two sizes fit exactly whenever their sum fits this pool.
        this.pool = this.freshPool();
        const buckets = Array.from({ length: this.pool.maxLevel - 3 }, () => [] as (typeof page)[]);
        for (const next of this.pages.values())
          if (next.word === null) buckets[Math.log2(next.words) - 4].push(next);
        for (let level = buckets.length - 1; level >= 0; level--)
          for (const next of buckets[level]) {
            next.offset = this.pool.allocate(next.words);
            if (next.offset < 0)
              throw new Error("ground scar compaction exceeded admitted word capacity");
            this.residentWords += next.words;
            this.extent = Math.max(this.extent, next.offset + next.words);
          }
        compacted = true;
        directoryChanged = true;
        break;
      }
      this.residentWords += page.words;
      this.extent = Math.max(this.extent, page.offset + page.words);
    }
    const capacity = 2 ** Math.ceil(Math.log2(this.extent)),
      width = this.fixed ? this.maxWidth : 2 ** Math.ceil(Math.log2(Math.sqrt(capacity))),
      height = this.fixed ? this.maxHeight : capacity / width,
      resized = width !== this.width || height !== this.height;
    this.width = width;
    this.height = height;
    if (resized) this.target.resize(width, height);
    let directoryBytes = 0;
    if (directoryChanged) {
      const entries = 2 ** Math.ceil(Math.log2(Math.max(1, this.pages.size * 2)));
      if (entries > SCAR_DIRECTORY_CAPACITY)
        throw new Error("ground scar directory exceeds fixed-extent capacity");
      this.words = new Uint32Array(entries * 2);
      const mask = entries - 1;
      for (const [key, page] of this.pages) {
        let at = scarHash(key, mask);
        while (this.words[at * 2]) at = (at + 1) & mask;
        this.words[at * 2] =
          ((key + 1) |
            (page.word !== null ? SCAR_UNIFORM : page.dense ? SCAR_DENSE : SCAR_RUNS)) >>>
          0;
        this.words[at * 2 + 1] =
          page.word !== null
            ? page.word
            : page.dense
              ? page.offset
              : page.offset * 256 + page.count;
      }
      this.target.directory(this.words, defaultWord, this.cacheBounds);
      directoryBytes = this.words.byteLength;
    }
    let textureBytes = 0;
    if (ground)
      for (const [key, page] of this.pages) {
        if (page.word !== null || (!resized && !reset && !compacted && !dirty.has(key))) continue;
        const count = ground.readScarRuns(
          (key % tilesX) * 16,
          Math.floor(key / tilesX) * 16,
          this.runs,
        );
        let pixels = 0;
        if (page.dense) {
          let prior = 0;
          for (let k = 0; k < count; k++) {
            for (let c = prior; c < this.runs[k * 2]; c++)
              putScarWord(this.pixels, c * 4, this.runs[k * 2 + 1]);
            prior = this.runs[k * 2];
          }
          pixels = 256;
        } else {
          for (let k = 0; k < count; k++) {
            putScarWord(this.pixels, k * 8, this.runs[k * 2]);
            putScarWord(this.pixels, k * 8 + 4, this.runs[k * 2 + 1]);
          }
          pixels = count * 2;
        }
        this.target.write(page.offset, this.pixels.subarray(0, pixels * 4), width);
        textureBytes += pixels * 4;
      }
    const bytes = textureBytes + directoryBytes;
    this.counts.lastBytes = bytes;
    this.counts.lastTextureBytes = textureBytes;
    this.counts.lastDirectoryBytes = directoryBytes;
    this.counts.totalBytes += bytes;
    this.counts.uploads++;
    if (reset || resized || compacted) this.counts.fullUploads++;
    return true;
  }
  directoryWords() {
    return this.words;
  }
  private release(offset: number, words: number) {
    this.pool.release(offset, words);
    this.residentWords -= words;
  }
  stats(): ScarStats {
    return {
      ...this.grid,
      ...this.counts,
      defaultWord: this.defaultWord,
      cacheBounds: this.cacheBounds,
      pages: this.pages.size,
      textureBytes: this.width * this.height * 4,
      directoryBytes: this.words.byteLength,
      directoryEntries: this.words.length / 2,
      poolWidth: this.width,
      poolHeight: this.height,
      poolWords: this.residentWords,
      initializationBytes: this.reservedWords * 4,
    };
  }
}
function putScarWord(out: Uint8Array, at: number, word: number) {
  for (let c = 0; c < 4; c++) out[at + c] = word >>> (c * 8);
}

export function createScarTexture(registry: GpuRegistry) {
  const device = registry.device,
    texture: GpuSlot<GPUTexture> = registry.slot(),
    directory: GpuSlot<GPUTexture> = registry.slot();
  texture.set(
    device.createTexture({
      label: "ground-scar-words",
      size: [SCAR_POOL_WIDTH, SCAR_POOL_HEIGHT, 1],
      format: "rgba8unorm",
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
    }),
  );
  const normalization = new Float32Array(SCAR_NORMALIZATION_WORDS);
  for (let q = 0; q < normalization.length; q++) normalization[q] = Math.min(q, 4080) / 4080;
  device.queue.writeTexture(
    { texture: texture.current! },
    new Uint8Array(normalization.buffer),
    { bytesPerRow: SCAR_POOL_WIDTH * 4, rowsPerImage: SCAR_NORMALIZATION_WORDS / SCAR_POOL_WIDTH },
    [SCAR_POOL_WIDTH, SCAR_NORMALIZATION_WORDS / SCAR_POOL_WIDTH],
  );
  let directoryWidth = 1,
    directoryHeight = 1;
  const allocateDirectory = () =>
    directory.set(
      device.createTexture({
        label: "ground-scar-directory",
        size: [directoryWidth, directoryHeight],
        format: "rg32uint",
        usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
      }),
    );
  allocateDirectory();
  const sync = new ScarSync(
    {
      resize() {},
      directory(words) {
        const capacity = words.length / 2,
          width = Math.min(1024, capacity),
          height = capacity / width;
        if (
          width > device.limits.maxTextureDimension2D ||
          height > device.limits.maxTextureDimension2D
        )
          throw new Error("ground scar directory exceeds device dimensions");
        if (width > directoryWidth || height > directoryHeight) {
          directoryWidth = Math.max(directoryWidth, width);
          directoryHeight = Math.max(directoryHeight, height);
          allocateDirectory();
        }
        device.queue.writeTexture(
          { texture: directory.current! },
          words,
          { bytesPerRow: width * 8, rowsPerImage: height },
          [width, height],
        );
      },
      write(offset, marks, width) {
        let source = 0,
          count = marks.byteLength / 4;
        while (count) {
          const x = offset % width,
            n = Math.min(count, width - x);
          device.queue.writeTexture(
            { texture: texture.current!, origin: [x, Math.floor(offset / width), 0] },
            marks.subarray(source * 4, (source + n) * 4),
            { bytesPerRow: n * 4, rowsPerImage: 1 },
            [n, 1],
          );
          offset += n;
          source += n;
          count -= n;
        }
      },
    },
    SCAR_POOL_WORDS,
    SCAR_NORMALIZATION_WORDS,
    true,
  );
  let ground: GroundMarks | null = null,
    pending: GroundChanges = { all: false, cells: new Uint32Array() },
    hasMarks = false,
    globalEligible = true;
  return {
    setGround(next: GroundMarks | null) {
      if (next && (next.cols > MAX_Q8_AXIS || next.rows > MAX_Q8_AXIS))
        throw new Error("ground grid exceeds this source's exact Q8 coordinate admission");
      const changes = next?.takeChanges(),
        replaced = next !== ground,
        changed =
          replaced ||
          changes?.all === true ||
          (!!changes && !changes.all && changes.cells.length > 0);
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
        const shape = next?.scarDefault();
        globalEligible =
          !shape ||
          (shape.exceptions * 2 <= SCAR_DIRECTORY_CAPACITY &&
            shape.poolWords + SCAR_NORMALIZATION_WORDS <= SCAR_POOL_WORDS);
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
    stats: () => ({ ...sync.stats(), directoryBytes: directoryWidth * directoryHeight * 8 }),
  };
}
export type ScarTexture = ReturnType<typeof createScarTexture>;
