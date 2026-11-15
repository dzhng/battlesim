// What the viewport's frame loop did around a visible glitch (the 3D going
// blank for a moment): scene rebuilds, canvas resizes, art swaps and runs of
// frames it wanted to draw but could not, on the diagnostics timeline.
import { diagnosticsEvent } from "@web/diagnostics";
import type { BrowserScene } from "@web/battle/render/scene";
import { gameRuntime } from "./gameAppearances";

/** A run of skipped frames shorter than this is not worth a line. */
const SKIP_EVENT_FRAMES = 2;

export class ViewportTrace {
  private skipping: { since: number; frames: number; reason: string } | null = null;
  private rebuilding: { since: number; reason: string } | null = null;

  constructor(private readonly canvas: HTMLCanvasElement) {}

  resized(from: [number, number], client: [number, number]) {
    diagnosticsEvent("canvas resized", {
      from,
      to: [this.canvas.width, this.canvas.height],
      client,
    });
  }

  /** A complete scene replacement starts; the old scene stops drawing. */
  rebuild(reason: string) {
    if (this.rebuilding) return;
    this.rebuilding = { since: performance.now(), reason };
    diagnosticsEvent("scene rebuild", { reason });
  }

  installed() {
    const started = this.rebuilding;
    this.rebuilding = null;
    diagnosticsEvent("scene installed", {
      reason: started?.reason ?? "requested",
      ms: started ? Math.round(performance.now() - started.since) : null,
      work: { page: gameRuntime.takeAdmissionWork() },
    });
  }

  /** New unit or scenery art went live: the generation, GPU memory after, and the work it took. */
  artCommitted(scene: BrowserScene) {
    const { core, transport } = scene.admissionWork();
    const page = gameRuntime.takeAdmissionWork();
    diagnosticsEvent("art committed", {
      generation: scene.appearances()?.generation ?? null,
      gpu: scene.stats().memory,
      // What this admission cost: Rust's report, and the page's own work
      // since the last one.
      work: { core, page: { ...transport, ...page } },
    });
  }

  /** The loop wanted a frame on screen and could not draw it. */
  skipped(now: number, reason: string) {
    if (this.skipping?.reason === reason) this.skipping.frames++;
    else {
      this.endSkip(now);
      this.skipping = { since: now, frames: 1, reason };
    }
  }

  drawn(now: number) {
    this.endSkip(now);
  }

  private endSkip(now: number) {
    const run = this.skipping;
    this.skipping = null;
    if (run && run.frames >= SKIP_EVENT_FRAMES)
      diagnosticsEvent("frames skipped", {
        reason: run.reason,
        frames: run.frames,
        ms: Math.round(now - run.since),
      });
  }
}
