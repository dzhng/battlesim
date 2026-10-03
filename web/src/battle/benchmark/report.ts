// The benchmark's JSON report and its frame-cost.md row. The
// report is the one measurement record: the results screen reads it,
// the scene saves it as evidence, and the row is derived from it.
//
// Ported by technique from ~/dev/game/web/src/battle/benchmark/
// benchmarkReport.ts.
import type { BenchmarkRecording } from "./recording";
import { benchmarkFingerprint, type BenchmarkLength, type BenchmarkScenario } from "./presets";
import type { PreparationReport } from "../prepare/protocol";

export interface BenchmarkIdentity {
  userAgent: string;
  /** GPU adapter description, as the browser reports it. */
  adapter: string;
  timestampQuery: boolean;
  /** CSS pixels and device pixel ratio of the window. */
  viewport: [number, number];
  dpr: number;
}

export interface BenchmarkOutcome {
  status: "complete" | "cancelled" | "failed";
  reason: string;
}

export function createBenchmarkReport(run: {
  scenario: BenchmarkScenario;
  length: BenchmarkLength;
  outcome: BenchmarkOutcome;
  identity: BenchmarkIdentity | null;
  recording: BenchmarkRecording;
  tickHz: number;
  /** The tick timing started on, and the last published. */
  startTick: number;
  endTick: number;
  preparation?: PreparationReport;
}) {
  const { scenario, recording } = run;
  const measured = recording.report(scenario.tour.phases.map((p) => p.name));
  return {
    kind: "battle-benchmark" as const,
    schema: 1,
    createdAt: new Date().toISOString(),
    scenario: {
      id: scenario.id,
      version: scenario.version,
      fingerprint: benchmarkFingerprint(scenario),
      variant: scenario.variant,
      seed: scenario.seed,
      blue: scenario.blue,
      red: scenario.blue === "scenario-orders" ? "scenario-orders" : "defender",
      startTick: scenario.startTick,
      cameraScript: scenario.tour.version,
    },
    /** The camera tour flown: phases and keyframes, in fractions of the run. */
    tour: scenario.tour,
    length: run.length,
    durationMs: scenario.durationMs[run.length],
    recordedMs: recording.frames.at(-1)?.elapsedMs ?? 0,
    outcome: run.outcome,
    identity: run.identity,
    ...(run.preparation && { preparation: run.preparation }),
    measurement:
      "Frame interval: requestAnimationFrame cadence, redrawn every frame. CPU: the viewport's frame callback. GPU: the battle frame's own timestamp-query frame total, read every 2 s over its last 240 frames; passes overlap on tile GPUs, so there is no per-pass split.",
    simulation: {
      startTick: run.startTick,
      endTick: run.endTick,
      simulatedSeconds: (run.endTick - run.startTick) / run.tickHz,
    },
    ...measured,
    samples: { frames: recording.frames, ticks: recording.ticks, stats: recording.stats },
  };
}

export type BenchmarkReport = ReturnType<typeof createBenchmarkReport>;

const ms = (v: number | undefined) => (v === undefined ? "—" : v.toFixed(1));
const mib = (bytes: number) => (bytes / 2 ** 20).toFixed(1);

/** The report as one `frame-cost.md` row (the caller names the row). */
export function frameCostRow(report: BenchmarkReport, slice: string): string {
  const gpu = report.gpu;
  const gpuCell = gpu
    ? `frame total ${gpu.meanMs.toFixed(2)} ms mean, worst window p95 ${gpu.p95Ms.toFixed(2)} (${report.phases.map((p) => `${p.name} ${p.gpu?.meanMs.toFixed(2) ?? "—"}`).join(", ")})`
    : "— (no timestamp-query)";
  const last = report.memory.last;
  const bytes = report.ticks.publicationBytes;
  const s = report.simulation;
  const heap = report.memory.peakHeapBytes;
  const note = [
    `${report.scenario.id} v${report.scenario.version}, tour ${report.scenario.cameraScript}, ${report.identity?.viewport.join("×")} DPR ${report.identity?.dpr}`,
    `${report.averageFps?.toFixed(1)} FPS average, ${report.framesOver33ms} frames over 33 ms`,
    `CPU p50/p95 ${ms(report.cpuMs?.p50)}/${ms(report.cpuMs?.p95)} ms`,
    `sim ${(s.simulatedSeconds / (report.recordedMs / 1000)).toFixed(2)}× real time from tick ${s.startTick} to ${s.endTick}, step p50/p95 ${ms(report.ticks.stepMs?.p50)}/${ms(report.ticks.stepMs?.p95)} ms`,
    heap === null
      ? "heap not reported"
      : `main-thread heap ${mib(last?.heapBytes ?? heap)} MiB at the end, peak ${mib(heap)}`,
  ].join("; ");
  return [
    "",
    slice,
    `benchmark ${report.length} run (${report.durationMs / 1000} s)`,
    ms(report.frameMs?.p50),
    ms(report.frameMs?.p95),
    ms(report.frameMs?.p99),
    gpuCell,
    last ? `${mib(last.bufferBytes)} (${last.buffers} buffers)` : "—",
    last ? `${mib(last.textureBytes)} (${last.textures} textures)` : "—",
    bytes
      ? `${Math.round(bytes.mean).toLocaleString("en-US")} mean, ${bytes.max.toLocaleString("en-US")} max`
      : "—",
    note,
    "",
  ]
    .join(" | ")
    .trim();
}
