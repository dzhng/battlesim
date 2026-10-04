import { requestGpuDevice, type GpuDeviceInfo } from "@packages/renderer-core/src/device";

/** Page resources survive client navigation; scene and worker owners do not. */
export class AppResources {
  private device: GpuDeviceInfo | null = null;
  private pending: Promise<GpuDeviceInfo> | null = null;
  private released = false;
  private failure: string | null = null;
  private readonly listeners = new Set<() => void>();

  readonly error = () => this.failure;
  readonly subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  refuse(error: unknown) {
    this.failure = error instanceof Error ? error.message : String(error);
    for (const listener of this.listeners) listener();
  }

  async gpu(): Promise<GpuDeviceInfo> {
    if (this.failure) throw new Error(this.failure);
    this.pending ??= requestGpuDevice({
      callbacks: {
        onDeviceLost: (report) => this.refuse(`WebGPU device lost: ${report.message}`),
        onUncapturedError: (report) => this.refuse(`WebGPU error: ${report.message}`),
      },
    }).then(
      (info) => {
        if (this.released) {
          info.device.destroy();
          throw new Error("Page resources have been released");
        }
        this.device = info;
        return info;
      },
      (error: unknown) => {
        this.refuse(error);
        throw error;
      },
    );
    return this.pending;
  }

  async warm(): Promise<void> {
    try {
      await Promise.all([
        this.gpu(),
        import("@web/battle/sim/module").then(({ loadWasm }) => loadWasm()),
        import("./gameAppearances").then(({ gameAppearances }) => gameAppearances()),
      ]);
    } catch (error) {
      if (!this.released) this.refuse(error);
    }
  }

  dispose() {
    if (this.released) return;
    this.released = true;
    this.device?.device.destroy();
    this.device = null;
    this.refuse("The page's resources were released. Reload to continue.");
  }
}

export const appResources = new AppResources();
