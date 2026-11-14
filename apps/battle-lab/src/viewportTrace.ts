// What the viewport's frame loop did around a visible glitch (the 3D going
// blank for a moment): scene rebuilds, canvas resizes, art swaps and runs of
// frames it wanted to draw but could not, on the diagnostics timeline.
import { diagnostics, diagnosticsEvent, diagnosticsSource } from "@web/diagnostics";
import type { BrowserScene } from "@web/battle/render/scene";

/** How often the canvas check samples what the canvas shows: a blank that
 *  lasts a second or more is caught, and a sample is one small readback. */
const CHECK_MS = 1_000;
/** A run of skipped frames shorter than this is not worth a line. */
const SKIP_EVENT_FRAMES = 2;

export class ViewportTrace {
  private skipping: { since: number; frames: number; reason: string } | null = null;
  private rebuilding: { since: number; reason: string } | null = null;
  private blankSince: number | null = null;
  private checkedAt = 0;
  private lastDrawn = 0;
  private probe: CanvasRenderingContext2D | null = null;
  /** What the check has cost the page, for the report's live section. */
  private cost = { samples: 0, totalMs: 0, maxMs: 0 };

  constructor(
    private readonly canvas: HTMLCanvasElement,
    signal: AbortSignal,
  ) {
    signal.addEventListener(
      "abort",
      diagnosticsSource("canvas check", () => ({
        everyMs: CHECK_MS,
        samples: this.cost.samples,
        meanMs: this.cost.samples
          ? Math.round((this.cost.totalMs / this.cost.samples) * 100) / 100
          : null,
        maxMs: Math.round(this.cost.maxMs * 100) / 100,
      })),
    );
  }

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
    });
  }

  /** New unit or scenery art went live: the generation and GPU memory after. */
  artCommitted(scene: BrowserScene) {
    diagnosticsEvent("art committed", {
      generation: scene.appearances()?.generation ?? null,
      gpu: scene.stats().memory,
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
    this.lastDrawn = now;
  }

  /** While diagnostics collect: what the canvas shows now, before this frame
   *  draws. A uniform picture is a blank one: no battle frame is one colour. */
  check(now: number, state: () => Record<string, unknown>) {
    if (now - this.checkedAt < CHECK_MS || !diagnostics.enabled()) return;
    this.checkedAt = now;
    const started = performance.now();
    const probe = (this.probe ??= sampleContext());
    probe.clearRect(0, 0, 16, 8);
    probe.drawImage(this.canvas, 0, 0, 16, 8);
    const px = probe.getImageData(0, 0, 16, 8).data;
    let spread = 0;
    for (let i = 4; i < px.length; i += 4)
      for (let c = 0; c < 4; c++) spread = Math.max(spread, Math.abs(px[i + c] - px[c]));
    const blank = spread < 3;
    const ms = performance.now() - started;
    this.cost.samples++;
    this.cost.totalMs += ms;
    this.cost.maxMs = Math.max(this.cost.maxMs, ms);
    if (blank && this.blankSince === null) {
      this.blankSince = now;
      diagnosticsEvent("canvas blank", {
        rgba: [px[0], px[1], px[2], px[3]],
        sinceDrawnMs: Math.round(now - this.lastDrawn),
        skipping: this.skipping && {
          reason: this.skipping.reason,
          frames: this.skipping.frames,
          ms: Math.round(now - this.skipping.since),
        },
        rebuilding: this.rebuilding?.reason ?? null,
        ...state(),
      });
    } else if (!blank && this.blankSince !== null) {
      diagnosticsEvent("canvas shows again", { blankMs: Math.round(now - this.blankSince) });
      this.blankSince = null;
    }
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

/** A 16 x 8 CPU canvas: the canvas is scaled into it, then read back. */
function sampleContext(): CanvasRenderingContext2D {
  const sample = document.createElement("canvas");
  [sample.width, sample.height] = [16, 8];
  return sample.getContext("2d", { willReadFrequently: true })!;
}
