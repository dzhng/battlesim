// @vitest-environment node
import { expect, test } from "vitest";
import { BenchmarkRecording } from "@web/battle/benchmark/recording";
import { createReelReport } from "@apps/battle-lab/src/benchmark/reelReport";
import { MENU_REEL_WORKLOAD, menuReelFingerprint } from "@web/battle/benchmark/menuReel";
import type { ReelResult } from "@apps/battle-lab/src/benchmark/reelRun";
import { gameLight } from "@apps/battle-lab/src/gameLight";
import { sunDirection } from "@packages/battle-renderer/src/light/sceneLight";
import type { DrawnUnits } from "@web/battle/benchmark/unitManifest";
import type { EffectsManifest } from "@web/battle/benchmark/effectManifest";
import type { PropsManifest } from "@web/battle/benchmark/propManifest";

const pose = { target: [0, 0] as const, distance: 40, yaw: 0, pitch: 0.2 };
/** A plated scene's units, as the page describes them (`describeUnits`). */
const units: DrawnUnits = {
  pose: {},
  leanHold: 1.5,
  detail: { lod_px: [150, 60, 24], impostor_px: 10 },
  tints: { blue: [1, 1, 1], red: [1.18, 1, 0.78] },
  types: {},
  appearances: {},
  tables: [],
};
/** Its buildings' parts, as the page describes them (`describeBuildings`). */
const props: PropsManifest = { buildings: { parts: [], shells: [] } };
/** Its effects, as the page describes them (`describeEffects`). */
const effects: EffectsManifest = {
  presentation: { smoke: {} },
  types: {},
  smokes: {},
  wrecks: {},
  wreckLooks: {},
};
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
    light: gameLight,
    plates: [],
    units,
    props,
    effects,
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
      light: { ...gameLight, sunDirection: Array.from(sunDirection(gameLight)) },
      layers: ["sky", "terrain"],
      omissions: [],
      units,
      props,
      effects,
    },
  ]);
  expect(report.capture.frameTimingComparable).toBe(true);
});

test("a run of some scenes reports their workload, never the whole reel's", () => {
  const whole = createReelReport([]);
  expect(whole.workload).toEqual(MENU_REEL_WORKLOAD);
  expect(whole.fingerprint).toBe(menuReelFingerprint());
  const last = MENU_REEL_WORKLOAD.scenes.at(-1)!;
  const report = createReelReport([result(last.map, 1000, [10])], {
    maps: [last.map],
    plates: true,
  });
  expect(report.outcome.status).toBe("complete");
  expect(report.workload.scenes).toEqual([last]);
  expect(report.canonical.sceneIds).toEqual([last.map]);
  expect(report.fingerprint).not.toBe(menuReelFingerprint());
  expect(report.canonical.fingerprint).toBe(report.fingerprint);
  expect(report.tour.phases.map((p) => p.name)).toEqual([last.map]);
  expect(report.durationMs).toBe(last.reel.shots.reduce((ms, shot) => ms + shot.seconds * 1000, 0));
  expect(report.capture.frameTimingComparable).toBe(false);
  // Its capture still names the scene within the whole reel.
  expect(JSON.parse(report.presentationCaptureJson[0]).workload.fingerprint).toBe(
    menuReelFingerprint(),
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
