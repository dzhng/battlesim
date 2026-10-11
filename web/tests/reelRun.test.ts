// @vitest-environment node
import { expect, test } from "vitest";
import { createReelRun } from "@apps/battle-lab/src/benchmark/reelRun";
import type { BackdropScene } from "@apps/battle-lab/src/menuReel";

const pose = { target: [0, 0] as [number, number], distance: 40, yaw: 0, pitch: 0.2 };
const scene: BackdropScene = {
  map: "fake-menu",
  encounter: "e",
  seed: 3,
  warm_s: 2,
  reel: { fade_s: 0.5, shots: [{ seconds: 2, from: pose, to: { ...pose, distance: 20 } }] },
};
const camera = {
  ...pose,
  target: [0, 0, 0] as [number, number, number],
  fovY: 1,
  aspect: 1,
  near: 0.1,
  far: 1000,
};

test("a reel records only drawn intervals after warm-up, finishes once, and uses wall time for the camera", () => {
  const results: unknown[] = [];
  const run = createReelRun(scene, 30, (r) => results.push(r));
  run.pilot.frame!({ now: 100, cpuMs: 1, camera });
  expect(run.recording.frames).toEqual([]);
  run.scripted.onWarm();
  run.pilot.frame!({ now: 1000, cpuMs: 1, camera });
  expect(run.pilot.pose(2000)?.distance).toBe(30);
  run.scripted.onTick({ tick: 90, stepMs: 2, bytes: 100 });
  run.pilot.frame!({ now: 2000, cpuMs: 1, camera });
  run.pilot.frame!({ now: 3000, cpuMs: 1, camera });
  run.pilot.frame!({ now: 3100, cpuMs: 1, camera });
  expect(run.recording.frames.map((f) => f.intervalMs)).toEqual([1000, 1000]);
  expect(results).toHaveLength(1);
  expect(results[0]).toMatchObject({ outcome: { status: "complete" }, startTick: 60, endTick: 90 });
});

test("a reel capture copies packed publications before their credits are released", () => {
  const results: unknown[] = [];
  const run = createReelRun(scene, 30, (r) => results.push(r));
  run.scripted.onWarm();
  run.pilot.frame!({ now: 1000, cpuMs: 1, camera });
  run.scripted.onPublication!({
    tick: 61,
    digest: "0123456789abcdef",
    packed: [1, 2, 3],
    warm: true,
  });
  expect(run.captureSamples()).toEqual([
    expect.objectContaining({ tick: 61, digest: "0123456789abcdef", publication: [1, 2, 3] }),
  ]);
});

test("cancelling during preparation cannot later publish a complete result", () => {
  const results: unknown[] = [];
  const run = createReelRun(scene, 30, (r) => results.push(r));
  run.cancel();
  run.scripted.onWarm();
  run.pilot.frame!({ now: 1000, cpuMs: 1, camera });
  run.pilot.frame!({ now: 4000, cpuMs: 1, camera });
  run.fail("late failure");
  expect(results).toHaveLength(1);
  expect(results[0]).toMatchObject({ outcome: { status: "cancelled" } });
  expect(run.recording.frames).toEqual([]);
});
