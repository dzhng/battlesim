// @vitest-environment node
import { expect, test } from "vitest";
import {
  BENCHMARK_PHASES,
  BENCHMARK_TOUR,
  cityContactTour,
  sampleTour,
  type BenchmarkTour,
} from "../src/battle/benchmark/camera";

test("the city overview includes visual surroundings while its fight stays at the playable centre", () => {
  const bare = cityContactTour([6000, 8000]);
  const framed = cityContactTour([6000, 8000], [-500, -500, 6500, 8500]);
  const overview = (tour: BenchmarkTour) => tour.keyframes.find((k) => k[0] === 0.42)!;
  expect(overview(framed).slice(1, 3)).toEqual([3000, 4000]);
  expect(overview(framed)[3] / overview(bare)[3]).toBe(9000 / 8000);
});

// A small tour of fixed numbers, so behaviour tests don't move when the
// real anchors are rescouted.
const TOUR: BenchmarkTour = {
  version: "test",
  phases: [
    { name: "strategic", from: 0, to: 0.5 },
    { name: "return", from: 0.5, to: 1 },
  ],
  keyframes: [
    [0, 0, 0, 100, 0, 0.8],
    [0.5, 100, 50, 300, -4, 0.4],
    [1, 0, 0, 100, -6.28, 0.8],
  ],
};

test("the tour passes through every keyframe at its time, at any run length", () => {
  for (const durationMs of [60_000, 300_000]) {
    const at = sampleTour(TOUR, 0.5 * durationMs, durationMs);
    expect(at.pose).toEqual({ target: [100, 50], distance: 300, yaw: -4, pitch: 0.4 });
  }
});

test("between keyframes the tour eases from one to the next without overshoot", () => {
  const early = sampleTour(TOUR, 5_000, 100_000).pose;
  const middle = sampleTour(TOUR, 25_000, 100_000).pose;
  expect(middle.target[0]).toBeCloseTo(50, 9); // smoothstep is symmetric
  expect(early.target[0]).toBeGreaterThan(0);
  expect(early.target[0]).toBeLessThan(middle.target[0] / 2); // eased in
  expect(middle.distance).toBeGreaterThan(100);
  expect(middle.distance).toBeLessThan(300);
});

test("yaw stays unwrapped: the tour turns through ±π instead of snapping", () => {
  const yaws = Array.from({ length: 101 }, (_, k) => sampleTour(TOUR, k * 1_000, 100_000).pose.yaw);
  for (let k = 1; k < yaws.length; k++) {
    expect(yaws[k]).toBeLessThanOrEqual(yaws[k - 1]);
    expect(Math.abs(yaws[k] - yaws[k - 1])).toBeLessThan(0.2);
  }
  expect(yaws.at(-1)).toBeCloseTo(-6.28, 9);
});

test("each moment belongs to exactly one phase, and times outside the run clamp", () => {
  expect(sampleTour(TOUR, 0, 10_000).phase).toBe("strategic");
  expect(sampleTour(TOUR, 4_999, 10_000).phase).toBe("strategic");
  expect(sampleTour(TOUR, 5_000, 10_000).phase).toBe("return");
  expect(sampleTour(TOUR, 10_000, 10_000).phase).toBe("return");
  expect(sampleTour(TOUR, 99_000, 10_000).pose).toEqual(sampleTour(TOUR, 10_000, 10_000).pose);
  expect(sampleTour(TOUR, -5, 10_000).pose).toEqual(sampleTour(TOUR, 0, 10_000).pose);
});

test("the benchmark tour visits the named phases in order and covers the whole run", () => {
  expect(BENCHMARK_TOUR.phases.map((p) => p.name)).toEqual(BENCHMARK_PHASES);
  expect(BENCHMARK_TOUR.phases[0].from).toBe(0);
  expect(BENCHMARK_TOUR.phases.at(-1)!.to).toBe(1);
  for (let k = 1; k < BENCHMARK_TOUR.phases.length; k++)
    expect(BENCHMARK_TOUR.phases[k].from).toBe(BENCHMARK_TOUR.phases[k - 1].to);
  const times = BENCHMARK_TOUR.keyframes.map((k) => k[0]);
  expect(times[0]).toBe(0);
  expect(times.at(-1)).toBe(1);
  expect([...times].sort((a, b) => a - b)).toEqual(times);
  // It returns to its opening framing, a whole turn later.
  const [first, last] = [BENCHMARK_TOUR.keyframes[0], BENCHMARK_TOUR.keyframes.at(-1)!];
  expect(last.slice(1, 4)).toEqual(first.slice(1, 4));
  expect(last[5]).toBe(first[5]);
  expect(Math.abs(last[4] - first[4]) % (2 * Math.PI)).toBeCloseTo(0, 2);
});
