// @vitest-environment node
import { expect, test } from "vitest";
import { BenchmarkRecording } from "@web/battle/benchmark/recording";
import { createReelReport } from "@apps/battle-lab/src/benchmark/reelReport";
import type { ReelResult } from "@apps/battle-lab/src/benchmark/reelRun";

const pose = { target: [0, 0] as const, distance: 40, yaw: 0, pitch: 0.2 };
function result(map: string, started: number, intervals: number[]): ReelResult {
  const recording = new BenchmarkRecording(1000);
  recording.start(started);
  let now = started;
  for (const interval of intervals) {
    now += interval;
    recording.frame({ now, cpuMs: 1, camera: pose, intended: pose, phase: map });
  }
  recording.tick({ tick: 61, stepMs: 2, bytes: 128 });
  return {
    scene: { map, encounter: "fake", seed: 1, warm_s: 2, reel: { fade_s: 0, shots: [{ seconds: 1, from: pose, to: pose }] } },
    recording, outcome: { status: "complete", reason: "done" }, startTick: 60, endTick: 61, tickHz: 30, layout: "{\"schema\":\"test\"}", adapter: "fake", capture: [], frames: [],
  };
}

test("the reel combines measured work without counting preparation gaps or losing simulation samples", () => {
  const report = createReelReport([result("first", 1000, [10, 20]), result("second", 100000, [30])]);
  expect(report.samples.frames.map((f) => f.elapsedMs)).toEqual([10, 30, 60]);
  expect(report.frameRate?.average).toBe(50);
  expect(report.recordedMs).toBe(60);
  expect(report.capture.presentation).toEqual({ clock: "wall_elapsed", subjectTrackingLagSeconds: 0.5, plateComposition: "standalone_center" });
  expect(report.ticks).toMatchObject({ count: 2, stepMs: { mean: 2 }, publicationBytes: { mean: 128 } });
});

test("exports each completed scene as a native presentation capture", () => {
  const report = createReelReport([result("first", 1000, [10])]);
  expect(report.presentationCaptures[0]).toMatchObject({ schema: "battle-presentation-capture/v1", workload: { id: "menu-reel", scene: "first" }, tickHz: 30, warmTick: 60 });
  expect(JSON.parse(report.presentationCaptureJson[0]).schema).toBe("battle-presentation-capture/v1");
});

test("a preparation failure stays failed in the exported report even before a scene can start", () => {
  const report = createReelReport([], { status: "failed", reason: "Map download failed" });
  expect(report.outcome).toEqual({ status: "failed", reason: "Map download failed" });
  expect(report.frameRate).toBeNull();
});

test("exports a frozen workload identity and host capture metadata", () => {
  const report = createReelReport([result("first", 1000, [10])]);
  expect(report.canonical).toEqual({
    workloadId: "menu-reel",
    workloadVersion: 1,
    fingerprint: "c905ce8c",
    subjectTrackingLagSeconds: 0.5,
    plateComposition: "standalone_center",
    sceneIds: ["market-town", "paris-corner"],
  });
  expect(report.identity).toMatchObject({
    userAgent: expect.any(String),
    viewport: expect.any(Object),
    dpr: expect.any(Number),
  });
});
