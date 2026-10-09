// What a benchmark run measured: every drawn frame's
// interval and CPU time, every simulation tick's step time and publication
// size, and periodic samples of the battle frame's own statistics (its GPU
// frame time and live memory). Recording starts at the first timed frame;
// the warm-up before it is never measured.
//
// Ported by technique from ~/dev/game/web/src/battle/benchmark/
// benchmarkRecording.ts and benchmarkMetrics.ts.
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import type { GpuAllocationCounts } from "@packages/renderer-core/src/gpuAllocations";

export interface Summary {
  count: number;
  mean: number;
  p50: number;
  p95: number;
  p99: number;
  max: number;
}

/** Nearest-rank percentiles; null for no samples (never a fake zero). */
export function summarize(values: readonly number[]): Summary | null {
  const sorted = values.filter(Number.isFinite).sort((a, b) => a - b);
  const n = sorted.length;
  if (n === 0) return null;
  const rank = (p: number) => sorted[Math.max(0, Math.ceil(n * p) - 1)];
  return {
    count: n,
    mean: sorted.reduce((sum, v) => sum + v, 0) / n,
    p50: rank(0.5),
    p95: rank(0.95),
    p99: rank(0.99),
    max: sorted[n - 1],
  };
}

/** Frame-rate summary derived from displayed-frame intervals. */
export interface FrameRateSummary {
  average: number;
  low1: number;
  minimum: number;
  maximum: number;
}

export function summarizeFrameRate(intervals: readonly number[]): FrameRateSummary | null {
  const rates = intervals
    .filter((ms) => Number.isFinite(ms) && ms > 0)
    .map((ms) => 1000 / ms);
  if (rates.length === 0) return null;
  const sorted = [...rates].sort((a, b) => a - b);
  const lowRank = Math.max(0, Math.ceil(sorted.length * 0.01) - 1);
  const total = intervals.reduce((sum, ms) => sum + ms, 0);
  return {
    average: (1000 * rates.length) / total,
    low1: sorted[lowRank],
    minimum: sorted[0],
    maximum: sorted[sorted.length - 1],
  };
}

export interface FrameSample {
  /** End of the frame, from the start of timing. */
  elapsedMs: number;
  /** Since the previous frame (the first: since timing started). */
  intervalMs: number;
  /** Main-thread time in the viewport's frame callback. */
  cpuMs: number;
  phase: string;
  /** The camera drawn, after the rig placed the tour's framing. */
  camera: CameraPose;
  /** The tour's framing. */
  intended: CameraPose;
}

export interface TickSample {
  tick: number;
  stepMs: number;
  /** Publication size on the wire. */
  bytes: number;
}

/** The battle frame's GPU frame time over its recent frames (`BattleFrame.stats().gpu`). */
export interface GpuWindow {
  frames: number;
  meanMs: number;
  p95Ms: number;
}

/** One reading of the battle frame's own statistics. */
export interface StatsSample {
  elapsedMs: number;
  phase: string;
  /** Null without `timestamp-query`, or before any frame resolved. */
  gpu: GpuWindow | null;
  memory: GpuAllocationCounts;
  /** Main-thread JS heap where the browser reports it (Chromium), else null. */
  heapBytes: number | null;
}

/** GPU frame time from stats samples: the mean of their window means, and
 *  the worst window p95. Null when no sample carried a GPU reading. */
function summarizeGpu(samples: readonly StatsSample[]) {
  const windows = samples.flatMap((s) => (s.gpu && s.gpu.frames > 0 ? [s.gpu] : []));
  if (windows.length === 0) return null;
  return {
    samples: windows.length,
    meanMs: windows.reduce((sum, w) => sum + w.meanMs, 0) / windows.length,
    p95Ms: Math.max(...windows.map((w) => w.p95Ms)),
  };
}

/** The recorder: bounded by the run's duration, fed by the viewport pilot. */
export class BenchmarkRecording {
  readonly frames: FrameSample[] = [];
  readonly ticks: TickSample[] = [];
  readonly stats: StatsSample[] = [];
  private startedAt: number | null = null;
  private previousAt = 0;
  private readonly capacity: number;

  constructor(durationMs: number) {
    // One sample per millisecond is beyond any display cadence.
    this.capacity = Math.ceil(durationMs) + 1;
  }

  get started(): boolean {
    return this.startedAt !== null;
  }

  start(now: number): void {
    this.startedAt = now;
    this.previousAt = now;
  }

  elapsed(now: number): number {
    return this.startedAt === null ? 0 : now - this.startedAt;
  }

  frame(f: {
    now: number;
    cpuMs: number;
    phase: string;
    camera: CameraPose;
    intended: CameraPose;
  }): void {
    if (this.startedAt === null) throw new Error("benchmark frame before timing started");
    if (this.frames.length >= this.capacity)
      throw new Error(`benchmark recording exceeded ${this.capacity} frames`);
    this.frames.push({
      elapsedMs: f.now - this.startedAt,
      intervalMs: f.now - this.previousAt,
      cpuMs: f.cpuMs,
      phase: f.phase,
      camera: f.camera,
      intended: f.intended,
    });
    this.previousAt = f.now;
  }

  tick(t: TickSample): void {
    if (this.startedAt !== null) this.ticks.push(t);
  }

  sample(s: StatsSample): void {
    if (this.startedAt !== null) this.stats.push(s);
  }

  /** The measured numbers, overall and per phase. */
  report(phases: readonly string[]) {
    const frameStats = (frames: readonly FrameSample[], stats: readonly StatsSample[]) => {
      const intervals = frames.map((f) => f.intervalMs);
      const total = intervals.reduce((sum, ms) => sum + ms, 0);
      return {
        frameMs: summarize(intervals),
        frameRate: summarizeFrameRate(intervals),
        cpuMs: summarize(frames.map((f) => f.cpuMs)),
        gpu: summarizeGpu(stats),
        averageFps: total > 0 ? (1000 * frames.length) / total : null,
        framesOver33ms: intervals.filter((ms) => ms > 1000 / 30).length,
      };
    };
    const peak = (pick: (s: StatsSample) => number | null) =>
      this.stats.reduce<number | null>((best, s) => {
        const v = pick(s);
        return v === null ? best : Math.max(best ?? v, v);
      }, null);
    const last = this.stats.at(-1) ?? null;
    return {
      ...frameStats(this.frames, this.stats),
      phases: phases.map((name) => ({
        name,
        ...frameStats(
          this.frames.filter((f) => f.phase === name),
          this.stats.filter((s) => s.phase === name),
        ),
      })),
      ticks: {
        count: this.ticks.length,
        first: this.ticks[0]?.tick ?? null,
        last: this.ticks.at(-1)?.tick ?? null,
        stepMs: summarize(this.ticks.map((t) => t.stepMs)),
        publicationBytes: summarize(this.ticks.map((t) => t.bytes)),
      },
      memory: {
        peakHeapBytes: peak((s) => s.heapBytes),
        peakBufferBytes: peak((s) => s.memory.bufferBytes),
        peakTextureBytes: peak((s) => s.memory.textureBytes),
        last: last && { ...last.memory, heapBytes: last.heapBytes },
      },
    };
  }
}
