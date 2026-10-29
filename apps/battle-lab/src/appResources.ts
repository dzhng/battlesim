import { BrowserGpu } from "@wasm/renderer_browser";
import { loadPresentation } from "@web/battle/sim/presentation";

/** Page resources survive client navigation; scene and worker owners do not. */
export class AppResources {
  private device: BrowserGpu | null = null;
  private pending: Promise<BrowserGpu> | null = null;
  private released = false;
  private failure: string | null = null;
  private readonly listeners = new Set<() => void>();
  private readonly departure = new AbortController();
  /** Aborted when the page's resources are released (the document leaves):
   *  page-lifetime work, such as the background prefetch, ends with it. */
  readonly signal = this.departure.signal;

  readonly error = () => this.failure;
  readonly subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  refuse(error: unknown) {
    this.failure = error instanceof Error ? error.message : String(error);
    for (const listener of this.listeners) listener();
  }

  async gpu(): Promise<BrowserGpu> {
    if (this.failure) throw new Error(this.failure);
    this.pending ??= loadPresentation()
      .then(() => BrowserGpu.new())
      .then(
        (info) => {
          if (this.released) {
            info.dispose();
            info.free();
            throw new Error("Page resources have been released");
          }
          this.device = info;
          void info.failure().then((reason) => {
            if (reason && !this.released) this.refuse(reason);
          });
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
      ]);
    } catch (error) {
      if (!this.released) this.refuse(error);
    }
  }

  dispose() {
    if (this.released) return;
    this.released = true;
    this.departure.abort();
    this.device?.dispose();
    this.device?.free();
    this.device = null;
    this.refuse("The page's resources were released. Reload to continue.");
  }
}

export const appResources = new AppResources();

/** Required rendering failures carry the browser admission remedy. */
export function gpuFailureMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return `${message} Enable WebGPU or use a browser/GPU combination that exposes navigator.gpu.`;
}
