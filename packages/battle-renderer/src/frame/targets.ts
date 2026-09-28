// The battle frame's size-dependent render targets, owned by a registry scope
// and swapped whole on resize.
import { d } from "typegpu";
import { GPU_DEPTH_FORMAT } from "@packages/renderer-core/src/depthContract";
import type { GpuRegistry } from "./registry";

/** 4× is the sample count WebGPU core guarantees for every renderable format. */
export const FRAME_MSAA = 4;
/** The lit world before post. */
export const HDR_FORMAT = "rgba16float" as const;
/** Overlays keep display values, quantised like the canvas they land on. */
export const OVERLAY_FORMAT = "rgba8unorm" as const;
/** The fog mask: per-sample (unseen, seen, ground, alpha) coverage, resolved
 *  to fractions (`fogCoverage`). */
export const FOG_MASK_FORMAT = "rgba8unorm" as const;
/** Distances to each side of the fog edge on the ground, over
 *  `FOG_DISTANCE_CAP_PX`. */
export const FOG_DISTANCE_FORMAT = "rg8unorm" as const;

/** What every world colour-pass fragment writes: its lit colour and its fog
 *  mask sample (`fogCoverage`). */
export const WORLD_OUT = { color: d.vec4f, fog: d.vec4f };

/** The world colour pass's targets, both blended alike when `blend` is given. */
export function worldTargets(blend?: GPUBlendState) {
  return {
    color: { format: HDR_FORMAT, ...(blend && { blend }) },
    fog: { format: FOG_MASK_FORMAT, ...(blend && { blend }) },
  };
}

export interface FrameTargets {
  width: number;
  height: number;
  /** The lit HDR world, multisampled; resolved into `lit`. */
  hdrMsaa: GPUTexture;
  lit: GPUTexture;
  /** The fog mask beside it, multisampled; resolved into `fogMask`. */
  fogMaskMsaa: GPUTexture;
  fogMask: GPUTexture;
  /** The fog mask pass's distances to the other side: along rows, then the
   *  Euclidean result. */
  fogDistanceRows: GPUTexture;
  fogDistance: GPUTexture;
  /** The world with fog applied: post's input. */
  hdr: GPUTexture;
  /** World depth: written by the depth prepass before any colour, then read
   *  by the world colour pass (which adds the grass's blades). */
  depth: GPUTexture;
  /** The overlays' depth: the prepass's, copied before the grass writes its
   *  blades, so ground cues lie over the grass and under every solid thing. */
  overlayDepth: GPUTexture;
  /** Display-space overlays over a transparent clear (the units' x-ray first),
   *  resolved into `overlay` and composited over post's output. */
  overlayMsaa: GPUTexture;
  overlay: GPUTexture;
  /** The ground paint (`paintedMarks.ts`): the marks drawn at the ground in
   *  view (through `overlayMsaa`, before the x-ray), which the painted
   *  ground layers read at their pixel. */
  paint: GPUTexture;
  /** The overlays' halo at half resolution: blurred along rows, then down
   *  columns (`overlayPass.ts`). */
  overlayGlowRows: GPUTexture;
  overlayGlow: GPUTexture;
}

const RENDER = 0x10; // GPUTextureUsage.RENDER_ATTACHMENT
const SAMPLED = 0x04; // GPUTextureUsage.TEXTURE_BINDING
const COPY_SRC = 0x01; // GPUTextureUsage.COPY_SRC
const COPY_DST = 0x02; // GPUTextureUsage.COPY_DST

export function allocateFrameTargets(
  scope: GpuRegistry,
  width: number,
  height: number,
): FrameTargets {
  const size = [width, height];
  const half = [Math.ceil(width / 2), Math.ceil(height / 2)];
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
    lit: scope.texture({
      label: "frame-lit",
      size,
      format: HDR_FORMAT,
      usage: RENDER | SAMPLED,
    }),
    fogMaskMsaa: scope.texture({
      label: "frame-fog-mask-msaa",
      size,
      format: FOG_MASK_FORMAT,
      sampleCount: FRAME_MSAA,
      usage: RENDER,
    }),
    fogMask: scope.texture({
      label: "frame-fog-mask",
      size,
      format: FOG_MASK_FORMAT,
      usage: RENDER | SAMPLED,
    }),
    fogDistanceRows: scope.texture({
      label: "frame-fog-distance-rows",
      size,
      format: FOG_DISTANCE_FORMAT,
      usage: RENDER | SAMPLED,
    }),
    fogDistance: scope.texture({
      label: "frame-fog-distance",
      size,
      format: FOG_DISTANCE_FORMAT,
      usage: RENDER | SAMPLED,
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
      // Sampled by slice 14's tile cull, one sample per pixel; copied into
      // the overlays' depth.
      usage: RENDER | SAMPLED | COPY_SRC,
    }),
    overlayDepth: scope.texture({
      label: "frame-overlay-depth",
      size,
      format: GPU_DEPTH_FORMAT,
      sampleCount: FRAME_MSAA,
      usage: RENDER | COPY_DST,
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
    paint: scope.texture({
      label: "frame-ground-paint",
      size,
      format: OVERLAY_FORMAT,
      usage: RENDER | SAMPLED,
    }),
    overlayGlowRows: scope.texture({
      label: "frame-overlay-glow-rows",
      size: half,
      format: OVERLAY_FORMAT,
      usage: RENDER | SAMPLED,
    }),
    overlayGlow: scope.texture({
      label: "frame-overlay-glow",
      size: half,
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
