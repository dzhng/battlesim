/** Admission work shares a budget across fielding and renderer preparation.
 * The document timeline identifies the browser frame, including continuations
 * outside RAF callbacks; elapsed work excludes network and GPU waits. */
import { art_work_budget_ms } from "@wasm/renderer_browser";

class ArtWork {
  private frame: unknown;
  private spent = 0;

  exhausted(): boolean {
    this.refresh();
    return this.spent >= art_work_budget_ms();
  }

  run<T>(step: () => T): T {
    this.refresh();
    const start = performance.now();
    try {
      return step();
    } finally {
      this.spent += performance.now() - start;
    }
  }

  private refresh(): void {
    const frame = document.timeline.currentTime;
    if (frame !== this.frame) {
      this.frame = frame;
      this.spent = 0;
    }
  }
}
export const artWork = new ArtWork();

/** A frame boundary for fielding, which also runs before a viewport exists. */
export function artFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}
