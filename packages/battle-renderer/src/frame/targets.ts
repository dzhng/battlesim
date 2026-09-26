// The battle frame's size-dependent render targets, owned by a registry scope
// and swapped whole on resize.
import { GPU_DEPTH_FORMAT } from "@packages/renderer-core/src/depthContract";
import type { GpuRegistry } from "./registry";

/** 4× is the sample count WebGPU core guarantees for every renderable format. */
export const FRAME_MSAA = 4;
/** The lit world before post. */
export const HDR_FORMAT = "rgba16float" as const;
/** Overlays keep display values, quantised like the canvas they land on. */
export const OVERLAY_FORMAT = "rgba8unorm" as const;

export interface FrameTargets {
  width: number;
  height: number;
  /** The HDR world, multisampled; resolved into `hdr` for post. */
  hdrMsaa: GPUTexture;
  hdr: GPUTexture;
  /** World depth: written by the depth prepass before any colour, then read
   *  by the world colour pass and the overlay pass. */
  depth: GPUTexture;
  /** Display-space overlays over a transparent clear, resolved into `overlay`
   *  and composited over post's output. */
  overlayMsaa: GPUTexture;
  overlay: GPUTexture;
}

const RENDER = 0x10; // GPUTextureUsage.RENDER_ATTACHMENT
const SAMPLED = 0x04; // GPUTextureUsage.TEXTURE_BINDING

export function allocateFrameTargets(
  scope: GpuRegistry,
  width: number,
  height: number,
): FrameTargets {
  const size = [width, height];
  return {
    width,
    height,
    hdrMsaa: scope.texture({
      label: "frame-hdr-msaa",
      size,
      format: HDR_FORMAT,
      sampleCount: FRAME_MSAA,
      usage: RENDER,
    }),
    hdr: scope.texture({
      label: "frame-hdr",
      size,
      format: HDR_FORMAT,
      usage: RENDER | SAMPLED,
    }),
    depth: scope.texture({
      label: "frame-depth",
      size,
      format: GPU_DEPTH_FORMAT,
      sampleCount: FRAME_MSAA,
      // Sampled by slice 14's tile cull, one sample per pixel.
      usage: RENDER | SAMPLED,
    }),
    overlayMsaa: scope.texture({
      label: "frame-overlay-msaa",
      size,
      format: OVERLAY_FORMAT,
      sampleCount: FRAME_MSAA,
      usage: RENDER,
    }),
    overlay: scope.texture({
      label: "frame-overlay",
      size,
      format: OVERLAY_FORMAT,
      usage: RENDER | SAMPLED,
    }),
  };
}

type Sized<T> = T & { width: number; height: number };

/** Size-dependent resources built asynchronously (post's pipelines) and
 *  swapped in whole once the new set exists. A build that a newer size
 *  overtakes is freed; one that finishes after disposal is freed too. */
export class SizedTargets<T extends { width: number; height: number }> {
  private installed: { value: Sized<T>; scope: GpuRegistry } | null = null;
  private latest = 0;
  private pending: { width: number; height: number; done: Promise<void> } | null = null;

  constructor(
    private readonly registry: GpuRegistry,
    private readonly build: (scope: GpuRegistry, width: number, height: number) => Promise<T>,
  ) {}

  get current(): Sized<T> | null {
    return this.installed?.value ?? null;
  }

  /** Resolves once no rebuild is under way; true if one was. */
  async settled(): Promise<boolean> {
    if (!this.pending) return false;
    while (this.pending) await this.pending.done.catch(() => {});
    return true;
  }

  /** Resolve once targets of this size are current. */
  ensure(width: number, height: number): Promise<void> {
    const now = this.installed?.value;
    if (now && now.width === width && now.height === height && !this.pending) {
      return Promise.resolve();
    }
    if (this.pending && this.pending.width === width && this.pending.height === height) {
      return this.pending.done;
    }
    const generation = ++this.latest;
    const scope = this.registry.scope();
    const done = this.build(scope, width, height).then(
      (value) => {
        if (generation !== this.latest) {
          scope.release();
          return;
        }
        this.pending = null;
        if (this.registry.isReleased) {
          scope.release();
          return;
        }
        this.installed?.scope.release();
        this.installed = { value, scope };
      },
      (error) => {
        scope.release();
        if (generation === this.latest) this.pending = null;
        throw error;
      },
    );
    this.pending = { width, height, done };
    return done;
  }
}
