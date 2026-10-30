import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import { sampleTour, type BenchmarkTour, type TourKeyframe } from "@web/battle/benchmark/camera";
import { BenchmarkRecording } from "@web/battle/benchmark/recording";
import type { ViewportPilot } from "../viewportRender";

type Attachment = Parameters<NonNullable<ViewportPilot["attach"]>>[0];
type DrawnFrame = Parameters<NonNullable<ViewportPilot["frame"]>>[0];
const key = (at: number, pose: CameraPose): TourKeyframe => [
  at,
  ...pose.target,
  pose.distance,
  pose.yaw,
  pose.pitch,
];

/** Holds and moving windows through the production rig and frame recorder. */
export function createViewMeasurement() {
  let attached: Attachment | null = null;
  let requested: CameraPose | null = null;
  let recording: BenchmarkRecording | null = null;
  let drawn: DrawnFrame | null = null;
  let startSimTime = 0;
  let live: DrawnFrame | null = null;
  let lastGpuAt = 0;
  let tour: BenchmarkTour | null = null;
  let durationMs = 0;
  let complete = false;
  let finalStats: ReturnType<Attachment["stats"]> | null = null;
  let samples: {
    elapsedMs: number;
    camera: DrawnFrame["camera"];
    stats: ReturnType<Attachment["stats"]>;
  }[] = [];
  const phase = () => (tour ? "transition" : "hold");
  const begin = () => {
    if (!attached || !requested) throw new Error("view measurement is not ready");
    if (!attached.stats().gpu) throw new Error("GPU timestamp-query unavailable");
    attached.resetGpuTiming();
    recording = new BenchmarkRecording(60_000);
    drawn = null;
    complete = false;
    finalStats = null;
    samples = [];
    lastGpuAt = 0;
  };
  const recordFrame = (now: number, frame: DrawnFrame) => {
    recording!.frame({
      now,
      cpuMs: frame.cpuMs,
      receiverFit: frame.receiverFit,
      phase: phase(),
      camera: { ...frame.camera, target: [frame.camera.target[0], frame.camera.target[1]] },
      intended: requested!,
    });
  };
  const pilot: ViewportPilot = {
    attach: (frame) => {
      attached = frame;
    },
    pose(now) {
      if (tour && recording && !complete) {
        if (!recording.started) {
          // Reset at the first moving frame, not when the browser command arrived.
          attached!.resetGpuTiming();
          recording.start(now);
        }
        // Attribute the interval through this callback to the preceding moving
        // frame, including a stall that carries us past the nominal endpoint.
        if (drawn) recordFrame(now, drawn);
        const elapsed = recording.elapsed(now);
        if (elapsed - lastGpuAt >= 1000 || elapsed >= durationMs) {
          const stats = attached!.stats();
          recording.sample({
            elapsedMs: elapsed,
            phase: phase(),
            gpu: stats.gpu,
            memory: stats.memory,
            heapBytes: null,
          });
          lastGpuAt = elapsed;
          finalStats = structuredClone(stats);
          attached!.resetGpuTiming();
        }
        if (elapsed >= durationMs) {
          // Freeze before drawing a stationary endpoint. Outstanding readbacks are
          // omitted, never replaced by samples from the following hold.
          complete = true;
        }
        requested = sampleTour(tour, elapsed, durationMs).pose;
      }
      return requested;
    },
    frame(frame) {
      live = frame;
      if (!recording || !requested || complete) return;
      if (!drawn) startSimTime = frame.simTime;
      drawn = frame;
      if (!recording.started) {
        recording.start(frame.now);
        return;
      }
      if (!tour) recordFrame(frame.now, frame);
      const elapsedMs = recording.elapsed(frame.now);
      if (!samples.length || elapsedMs - samples.at(-1)!.elapsedMs >= 250) {
        const stats = structuredClone(attached!.stats());
        samples.push({ elapsedMs, camera: structuredClone(frame.camera), stats });
        recording.sample({
          elapsedMs,
          phase: phase(),
          gpu: null,
          memory: stats.memory,
          heapBytes:
            (performance as Performance & { memory?: { usedJSHeapSize: number } }).memory
              ?.usedJSHeapSize ?? null,
        });
      }
    },
  };
  return {
    pilot,
    current: () => live && { now: live.now, camera: live.camera, simTime: live.simTime },
    place(pose: CameraPose) {
      requested = pose;
      recording = null;
      drawn = null;
      tour = null;
    },
    begin,
    transition(to: CameraPose, milliseconds: number) {
      if (!requested) throw new Error("view measurement is not ready");
      if (!(milliseconds > 0 && milliseconds <= 60_000))
        throw new Error("transition duration must be in (0, 60000]");
      tour = {
        version: "view-transition",
        phases: [{ name: "combined", from: 0, to: 1 }],
        keyframes: [key(0, requested), key(1, to)],
      };
      durationMs = milliseconds;
      begin();
    },
    tick(tick: Parameters<BenchmarkRecording["tick"]>[0]) {
      if (!complete) recording?.tick(tick);
    },
    read() {
      if (!recording || !drawn || !attached) return null;
      const stats = complete ? finalStats! : attached.stats();
      const gpuWindows = recording.stats.filter((sample) => sample.gpu !== null);
      return {
        requested,
        camera: drawn.camera,
        canvas: drawn.canvas,
        startSimTime,
        simTime: drawn.simTime,
        complete,
        durationMs: tour ? durationMs : null,
        tour,
        measurement: recording.report([phase()]),
        frames: recording.frames,
        gpuWindows,
        validGpuSamples: gpuWindows.reduce((sum, sample) => sum + sample.gpu!.frames, 0),
        samples,
        stats,
        currentCounts: drawn.counts(),
      };
    },
  };
}
