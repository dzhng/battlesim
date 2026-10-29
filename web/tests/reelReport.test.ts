// @vitest-environment node
import { expect, test } from "vitest";
import { BenchmarkRecording } from "@web/battle/benchmark/recording";
import { createReelReport } from "@apps/battle-lab/src/benchmark/reelReport";
import { MENU_REEL_WORKLOAD, reelFingerprint } from "@web/battle/benchmark/reels";
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
    scene: {
      map,
      encounter: "fake",
      seed: 1,
      warm_s: 2,
      reel: { fade_s: 0, shots: [{ seconds: 1, from: pose, to: pose }] },
    },
    recording,
    outcome: { status: "complete", reason: "done" },
    startTick: 60,
    endTick: 61,
    tickHz: 30,
    layout: '{"schema":"test"}',
    adapter: "fake",
    capture: [],
    frames: [
      {
        index: 0,
        reelMs: 10,
        tick: 61,
        simTime: 2,
        eye: [40, 0, 8],
        target: [0, 0, 0],
        up: [0, 0, 1],
        canvas: [1280, 720],
        plate: null,
      },
    ],
    lens: { fovY: 0.8, near: 1, depth: "reverse-z", far: "infinite" },
    plates: [],
    counts: [],
  };
}

test("the reel combines measured work without counting preparation gaps or losing simulation samples", () => {
  const report = createReelReport([
    result("first", 1000, [10, 20]),
    result("second", 100000, [30]),
  ]);
  expect(report.samples.frames.map((f) => f.elapsedMs)).toEqual([10, 30, 60]);
  expect(report.frameRate?.average).toBe(50);
  expect(report.recordedMs).toBe(60);
  expect(report.capture.presentation).toEqual({
    clock: "wall_elapsed",
    subjectTrackingLagSeconds: 0.5,
    plateComposition: "standalone_center",
  });
  expect(report.ticks).toMatchObject({
    count: 2,
    stepMs: { mean: 2 },
    publicationBytes: { mean: 128 },
  });
});

test("exports each scene's capture with the frames it drew and what it drew them with", () => {
  const report = createReelReport([result("first", 1000, [10])], { layers: ["sky", "terrain"] });
  const capture = JSON.parse(report.presentationCaptureJson[0]);
  expect(capture).toMatchObject({
    schema: "battle-presentation-capture",
    workload: { id: "menu-reel", scene: "first" },
    tickHz: 30,
    warmTick: 60,
    frames: [{ index: 0, plate: null }],
  });
  expect(report.drawnPresentations).toEqual([
    {
      lens: { fovY: 0.8, near: 1, depth: "reverse-z", far: "infinite" },
      layers: ["sky", "terrain"],
    },
  ]);
  expect(report.capture.frameTimingComparable).toBe(true);
});

test("a run of some scenes reports their workload, never the whole reel's", () => {
  const whole = createReelReport([]);
  expect(whole.workload).toEqual(MENU_REEL_WORKLOAD);
  expect(whole.fingerprint).toBe(reelFingerprint());
  const last = MENU_REEL_WORKLOAD.scenes.at(-1)!;
  const report = createReelReport([result(last.map, 1000, [10])], {
    maps: [last.map],
    plates: true,
  });
  expect(report.outcome.status).toBe("complete");
  expect(report.workload.scenes).toEqual([last]);
  expect(report.canonical.sceneIds).toEqual([last.map]);
  expect(report.fingerprint).not.toBe(reelFingerprint());
  expect(report.canonical.fingerprint).toBe(report.fingerprint);
  expect(report.tour.phases.map((p) => p.name)).toEqual([last.map]);
  expect(report.durationMs).toBe(last.reel.shots.reduce((ms, shot) => ms + shot.seconds * 1000, 0));
  expect(report.capture.frameTimingComparable).toBe(false);
  // Its capture still names the scene within the whole reel.
  expect(JSON.parse(report.presentationCaptureJson[0]).workload.fingerprint).toBe(
    reelFingerprint(),
  );
  expect(() => createReelReport([], { maps: ["nowhere"] })).toThrow(/nowhere/);
});

test("a preparation failure stays failed in the exported report even before a scene can start", () => {
  const report = createReelReport([], {
    interruption: { status: "failed", reason: "Map download failed" },
  });
  expect(report.outcome).toEqual({ status: "failed", reason: "Map download failed" });
  expect(report.frameRate).toBeNull();
});

test("exports a frozen workload identity and host capture metadata", () => {
  const report = createReelReport([result("first", 1000, [10])]);
  expect(report.canonical).toEqual({
    workloadId: "menu-reel",
    workloadVersion: 1,
    fingerprint: "6427d68b",
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
