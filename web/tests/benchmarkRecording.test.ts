// @vitest-environment node
import { expect, test } from "vitest";
import { BenchmarkRecording, summarize } from "../src/battle/benchmark/recording";

const pose = { target: [0, 0] as const, distance: 100, yaw: 0, pitch: 0.8 };
const frame = (now: number, phase = "strategic", cpuMs = 1) => ({
  now,
  cpuMs,
  phase,
  camera: pose,
  intended: pose,
});

test("percentiles are nearest-rank over the samples", () => {
  const values = Array.from({ length: 100 }, (_, k) => 100 - k); // 100..1, unsorted
  const s = summarize(values)!;
  expect([s.p50, s.p95, s.p99, s.max]).toEqual([50, 95, 99, 100]);
  expect(s.mean).toBe(50.5);
  expect(s.count).toBe(100);
  expect(summarize([7])).toMatchObject({ p50: 7, p95: 7, p99: 7 });
  expect(summarize([])).toBeNull();
});

test("the first frame interval begins at the explicit recording start", () => {
  const rec = new BenchmarkRecording(60_000);
  rec.start(1000);
  rec.frame(frame(1010));
  rec.frame(frame(1018));
  rec.frame(frame(1034));
  expect(rec.frames.map((f) => f.intervalMs)).toEqual([10, 8, 16]);
  expect(rec.frames.map((f) => f.elapsedMs)).toEqual([10, 18, 34]);
});

test("the report splits frame percentiles by phase and counts slow frames", () => {
  const rec = new BenchmarkRecording(60_000);
  rec.start(0);
  let now = 0;
  for (let k = 0; k < 10; k++) rec.frame(frame((now += 10), "strategic"));
  for (let k = 0; k < 10; k++) rec.frame(frame((now += 40), "ground", 3));
  const report = rec.report(["strategic", "ground", "return"]);
  const byName = Object.fromEntries(report.phases.map((p) => [p.name, p]));
  expect(byName.strategic.frameMs!.p50).toBe(10);
  expect(byName.ground.frameMs!.p50).toBe(40);
  expect(byName.ground.cpuMs!.p50).toBe(3);
  expect(byName.return.frameMs).toBeNull(); // no frames: no number, not zero
  expect(report.frameMs!.p95).toBe(40);
  expect(report.framesOver33ms).toBe(10);
  expect(report.averageFps).toBeCloseTo((1000 * 20) / 500, 9);
});

test("GPU time per phase comes from the frame's stats samples taken in that phase", () => {
  const memory = { buffers: 1, textures: 1, bufferBytes: 10, textureBytes: 20 };
  const stat = (
    elapsedMs: number,
    phase: string,
    gpu: { meanMs: number; p95Ms: number } | null,
  ) => ({
    elapsedMs,
    phase,
    gpu: gpu && { frames: 240, ...gpu },
    memory,
    heapBytes: null,
  });
  const rec = new BenchmarkRecording(60_000);
  rec.sample(stat(0, "strategic", { meanMs: 99, p95Ms: 99 })); // before timing: dropped
  rec.start(0);
  rec.sample(stat(2000, "strategic", { meanMs: 1, p95Ms: 2 }));
  rec.sample(stat(4000, "strategic", { meanMs: 3, p95Ms: 5 }));
  rec.sample(stat(6000, "ground", { meanMs: 4, p95Ms: 6 }));
  rec.sample(stat(8000, "ground", null)); // no reading: not a zero
  const report = rec.report(["strategic", "ground", "return"]);
  const byName = Object.fromEntries(report.phases.map((p) => [p.name, p.gpu]));
  expect(byName.strategic).toEqual({ samples: 2, meanMs: 2, p95Ms: 5 });
  expect(byName.ground).toEqual({ samples: 1, meanMs: 4, p95Ms: 6 });
  expect(byName.return).toBeNull();
  expect(report.gpu).toEqual({ samples: 3, meanMs: 8 / 3, p95Ms: 6 });
  expect(report.memory.peakTextureBytes).toBe(20);
});

test("ticks and memory are recorded only once timing starts", () => {
  const rec = new BenchmarkRecording(60_000);
  rec.tick({ tick: 5, stepMs: 9, bytes: 100 });
  rec.start(0);
  rec.tick({ tick: 6, stepMs: 1, bytes: 200 });
  rec.tick({ tick: 7, stepMs: 3, bytes: 400 });
  const report = rec.report([]);
  expect(report.ticks).toMatchObject({ first: 6, last: 7, count: 2 });
  expect(report.ticks.stepMs).toMatchObject({ p50: 1, max: 3 });
  expect(report.ticks.publicationBytes).toMatchObject({ max: 400 });
});
