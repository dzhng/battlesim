// The GPU side of the side's learned scars. The presentation keeps the
// learned ground and its bounded cache (`crates/presentation/src/scars`) and
// says what to upload; this holds the two textures and writes what it is told.
import { scar_normalization } from "@wasm/presentation/presentation_wasm.js";
import type { GpuRegistry, GpuSlot } from "./registry";
import { SCAR_NORMALIZATION_WORDS, SCAR_POOL_HEIGHT, SCAR_POOL_WIDTH } from "../records.gen";

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

/** The pool (the normalization table, then the pages) and the directory. */
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
  device.queue.writeTexture(
    { texture: texture.current! },
    scar_normalization(),
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
  return {
    /** Writes `upload`; true when the directory texture was replaced. */
    upload({ directory: next, writes, texels }: ScarUpload): boolean {
      let replaced = false;
      if (next) {
        const { words, width, height } = next;
        if (
          width > device.limits.maxTextureDimension2D ||
          height > device.limits.maxTextureDimension2D
        )
          throw new Error("ground scar directory exceeds device dimensions");
        if (width > directoryWidth || height > directoryHeight) {
          directoryWidth = Math.max(directoryWidth, width);
          directoryHeight = Math.max(directoryHeight, height);
          allocateDirectory();
          replaced = true;
        }
        device.queue.writeTexture(
          { texture: directory.current! },
          words,
          { bytesPerRow: width * 8, rowsPerImage: height },
          [width, height],
        );
      }
      for (let k = 0, at = 0; k < writes.length; k += 2) {
        const [offset, n] = [writes[k], writes[k + 1]];
        device.queue.writeTexture(
          {
            texture: texture.current!,
            origin: [offset % SCAR_POOL_WIDTH, Math.floor(offset / SCAR_POOL_WIDTH), 0],
          },
          texels.subarray(at, at + n * 4),
          { bytesPerRow: n * 4, rowsPerImage: 1 },
          [n, 1],
        );
        at += n * 4;
      }
      return replaced;
    },
    get texture() {
      return texture.current!;
    },
    get directory() {
      return directory.current!;
    },
    bytes: () => ({
      textureBytes: SCAR_POOL_WIDTH * SCAR_POOL_HEIGHT * 4,
      directoryBytes: directoryWidth * directoryHeight * 8,
    }),
  };
}
