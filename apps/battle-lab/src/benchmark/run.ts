// One benchmark run: the viewport pilot that flies the tour and records every
// frame, the tick feed from the scripted simulation, periodic readings of the
// battle frame's own statistics, and the finish that turns the recording into
// a report. Timing starts on the first frame after the simulation stands at
// the scenario's start tick.
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import { sampleTour } from "@web/battle/benchmark/camera";
import { BenchmarkRecording } from "@web/battle/benchmark/recording";
import {
  createBenchmarkReport,
  type BenchmarkIdentity,
  type BenchmarkOutcome,
  type BenchmarkReport,
} from "@web/battle/benchmark/report";
import type { BenchmarkLength, BenchmarkScenario } from "@web/battle/benchmark/presets";
import type { ViewportPilot } from "../LabViewport";
import type { ScriptedSim } from "../useSimSession";
import type { PreparationReport } from "@web/battle/prepare/protocol";

export type RunStage = "preparing" | "running" | "finishing";

/** The battle frame's GPU time is a rolling window of its last 240 frames:
 *  about 2 s at 120 Hz, so readings this far apart barely overlap. */
const STATS_EVERY_MS = 2000;

const poseOf = (c: Camera3DParams): CameraPose => ({
  target: [c.target[0], c.target[1]],
  distance: c.distance,
  yaw: c.yaw,
  pitch: c.pitch,
});

const heapBytes = (): number | null =>
  (performance as Performance & { memory?: { usedJSHeapSize: number } }).memory?.usedJSHeapSize ??
  null;

type FrameStats = Parameters<NonNullable<ViewportPilot["attach"]>>[0]["stats"];

export function createBenchmarkRun(
  scenario: BenchmarkScenario,
  length: BenchmarkLength,
  tickHz: number,
  onDone: (report: BenchmarkReport) => void,
  preparation?: PreparationReport,
) {
  const durationMs = scenario.durationMs[length];
  const recording = new BenchmarkRecording(durationMs);
  let stage: RunStage = "preparing";
  let warm = false;
  let identity: BenchmarkIdentity | null = null;
  let stats: FrameStats | null = null;
  let lastTick = scenario.startTick;
  let lastStatsAt = 0;
  let elapsedMs = 0;

  const sampleStats = (now: number, phase: string) => {
    lastStatsAt = now;
    if (!stats) return;
    const s = stats();
    recording.sample({
      elapsedMs: recording.elapsed(now),
      phase,
      gpu: s.gpu,
      memory: s.memory,
      heapBytes: heapBytes(),
    });
  };

  const finish = (outcome: BenchmarkOutcome) => {
    if (stage === "finishing") return;
    stage = "finishing";
    if (identity) identity.timestampQuery = stats?.().gpu != null;
    onDone(
      createBenchmarkReport({
        scenario,
        length,
        outcome,
        identity,
        recording,
        tickHz,
        startTick: scenario.startTick,
        endTick: lastTick,
        preparation,
      }),
    );
  };

  const pilot: ViewportPilot = {
    attach(frame) {
      stats = frame.stats;
      identity = {
        userAgent: navigator.userAgent,
        adapter: frame.adapter,
        timestampQuery: frame.stats().gpu !== null,
        viewport: [window.innerWidth, window.innerHeight],
        dpr: window.devicePixelRatio,
      };
    },
    pose: (now) => sampleTour(scenario.tour, recording.elapsed(now), durationMs).pose,
    frame({ now, cpuMs, camera }) {
      if (stage === "preparing") {
        if (!warm) return;
        // This frame opens the window; the next one is the first interval.
        // The frame's GPU window still holds warm-up frames: first reading
        // is one window later.
        recording.start(now);
        stage = "running";
        lastStatsAt = now;
        return;
      }
      if (stage !== "running") return;
      elapsedMs = recording.elapsed(now);
      const intended = sampleTour(scenario.tour, elapsedMs, durationMs);
      recording.frame({
        now,
        cpuMs,
        phase: intended.phase,
        camera: poseOf(camera),
        intended: intended.pose,
      });
      const done = elapsedMs >= durationMs;
      if (done || now - lastStatsAt >= STATS_EVERY_MS) sampleStats(now, intended.phase);
      if (done) finish({ status: "complete", reason: "Timed window complete" });
    },
  };

  const scripted: ScriptedSim & { pilot: ViewportPilot } = {
    warmTo: scenario.startTick,
    onWarm: () => (warm = true),
    onTick(t) {
      lastTick = t.tick;
      recording.tick(t);
    },
    pilot,
  };

  return {
    scripted,
    /** Where the run stands, for the progress panel. */
    status: () => ({
      stage,
      elapsedMs,
      durationMs,
      phase: sampleTour(scenario.tour, elapsedMs, durationMs).phase,
    }),
    cancel: () => finish({ status: "cancelled", reason: "Cancelled: partial run" }),
    fail: (reason: string) => finish({ status: "failed", reason }),
  };
}

export type BenchmarkRun = ReturnType<typeof createBenchmarkRun>;
