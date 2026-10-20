import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import { BenchmarkRecording } from "@web/battle/benchmark/recording";
import type { ViewportPilot } from "../LabViewport";

type Attachment = Parameters<NonNullable<ViewportPilot["attach"]>>[0];
type DrawnFrame = Parameters<NonNullable<ViewportPilot["frame"]>>[0];

/** A harness-controlled hold through the production rig and frame recorder. */
export function createViewMeasurement() {
  let attached: Attachment | null = null;
  let requested: CameraPose | null = null;
  let recording: BenchmarkRecording | null = null;
  let drawn: DrawnFrame | null = null;
  let startSimTime = 0;
  const pilot: ViewportPilot = {
    attach: (frame) => {
      attached = frame;
    },
    pose: () => requested,
    frame(frame) {
      if (!recording || !requested) return;
      drawn = frame;
      if (!recording.started) {
        recording.start(frame.now);
        startSimTime = frame.simTime;
        return;
      }
      recording.frame({
        now: frame.now,
        cpuMs: frame.cpuMs,
        phase: "hold",
        camera: { ...frame.camera, target: [frame.camera.target[0], frame.camera.target[1]] },
        intended: requested,
      });
    },
  };
  return {
    pilot,
    place(pose: CameraPose) {
      requested = pose;
      recording = null;
      drawn = null;
    },
    begin() {
      if (!attached || !requested) throw new Error("view measurement is not ready");
      if (!attached.stats().gpu) throw new Error("GPU timestamp-query unavailable");
      attached.resetGpuTiming();
      recording = new BenchmarkRecording(60_000);
      drawn = null;
    },
    tick(tick: Parameters<BenchmarkRecording["tick"]>[0]) {
      recording?.tick(tick);
    },
    read() {
      if (!recording || !drawn || !attached) return null;
      return {
        requested,
        camera: drawn.camera,
        canvas: drawn.canvas,
        startSimTime,
        simTime: drawn.simTime,
        measurement: recording.report(["hold"]),
        stats: attached.stats(),
      };
    },
  };
}
