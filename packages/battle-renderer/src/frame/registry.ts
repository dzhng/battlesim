// The battle frame's one owner of GPU lifetimes. TypeGPU's `root.destroy()`
// does not free what a root created, and the ported environment, shadow and
// post owners each hold their own root, so nothing is freed unless someone
// destroys it: every allocation the frame makes, directly or through those
// owners, is registered here and released with its scope.
//
// Scopes nest: the frame's registry holds the long-lived resources, and a
// child scope holds the size-dependent targets, released as a unit on resize.
// A slot holds one replaceable resource (a mesh buffer, the fog maps):
// setting it destroys the previous one. Numbers come from the device-wide
// `trackGpuAllocations`, so they include TypeGPU's internal allocations.
import {
  trackGpuAllocations,
  type GpuAllocationCounts,
} from "@packages/renderer-core/src/gpuAllocations";

export interface Releasable {
  destroy(): void;
}

/** One replaceable resource owned by a registry. */
export interface GpuSlot<T extends Releasable> {
  readonly current: T | null;
  /** Hold `next`, destroying what the slot held before. */
  set(next: T): T;
}

export class GpuRegistry {
  private owned: Releasable[] = [];
  private released = false;
  private readonly counts: () => GpuAllocationCounts;

  constructor(readonly device: GPUDevice) {
    this.counts = trackGpuAllocations(device);
  }

  /** Create and own a texture. */
  texture(descriptor: GPUTextureDescriptor): GPUTexture {
    this.live();
    return this.own(this.device.createTexture(descriptor));
  }

  /** Create and own a buffer. */
  buffer(descriptor: GPUBufferDescriptor): GPUBuffer {
    this.live();
    return this.own(this.device.createBuffer(descriptor));
  }

  /** Own a resource something else created; it is destroyed with this scope. */
  own<T extends Releasable>(resource: T): T {
    if (this.released) {
      // A late async result (a post chain finishing after dispose) is freed
      // at once instead of leaking.
      resource.destroy();
      throw new Error("GPU registry released");
    }
    this.owned.push(resource);
    return resource;
  }

  /** Own a disposer (an owner with `dispose()` rather than `destroy()`). */
  adopt(dispose: () => void): void {
    this.own({ destroy: dispose });
  }

  /** A child scope, released with this one or on its own. */
  scope(): GpuRegistry {
    const child = new GpuRegistry(this.device);
    const entry = this.own({ destroy: () => child.release() });
    // A child released on its own (targets on resize) leaves this scope's list.
    child.detach = () => {
      this.owned = this.owned.filter((r) => r !== entry);
    };
    return child;
  }

  private detach: () => void = () => {};

  /** One replaceable resource, destroyed with this scope. */
  slot<T extends Releasable>(): GpuSlot<T> {
    let current: T | null = null;
    this.own({ destroy: () => current?.destroy() });
    return {
      get current() {
        return current;
      },
      set: (next: T) => {
        this.live();
        current?.destroy();
        current = next;
        return next;
      },
    };
  }

  /** Destroy everything this scope owns, newest first. Idempotent. */
  release(): void {
    if (this.released) return;
    this.released = true;
    this.detach();
    const owned = this.owned;
    this.owned = [];
    for (let i = owned.length - 1; i >= 0; i--) owned[i].destroy();
  }

  get isReleased(): boolean {
    return this.released;
  }

  /** The device's live allocations, TypeGPU's included. */
  stats(): GpuAllocationCounts {
    return this.counts();
  }

  private live() {
    if (this.released) throw new Error("GPU registry released");
  }
}
