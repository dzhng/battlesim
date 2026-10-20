// @vitest-environment node
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { expect, test, vi } from "vitest";
import { createViewMeasurement } from "@apps/battle-lab/src/benchmark/viewMeasurement";

test("each hold records only its own drawn intervals and resolved camera", () => {
  const probe = createViewMeasurement();
  const resetGpuTiming = vi.fn();
  const stats = () => ({ gpu: { frames: 2, meanMs: 7, p95Ms: 8 }, memory: {} });
  probe.pilot.attach!({ adapter: "test", stats, resetGpuTiming } as never);
  const intended = { target: [10, 20] as const, distance: 65, pitch: 0.85, yaw: -1.57 };
  const camera: Camera3DParams = {
    ...intended,
    target: [10, 20, 3],
    distance: 80,
    fovY: 0.8,
    aspect: 1.6,
    near: 1,
  };
  const draw = (now: number, cpuMs: number) =>
    probe.pilot.frame!({
      now,
      cpuMs,
      receiverFit:
        cpuMs === 4
          ? { bounds_visited: 13, triangle_tests: 2, clip_tests: 2, reused: false }
          : { bounds_visited: 0, triangle_tests: 0, clip_tests: 0, reused: true },
      camera,
      simTime: 3,
      canvas: [960, 600],
      counts: () => ({}),
    } as never);
  probe.place(intended);
  draw(100, 99);
  probe.begin();
  draw(1000, 90);
  draw(1020, 4);
  draw(1050, 6);
  const first = probe.read();
  expect(first?.frames.map((f) => f.receiverFit)).toEqual([
    { bounds_visited: 13, triangle_tests: 2, clip_tests: 2, reused: false },
    { bounds_visited: 0, triangle_tests: 0, clip_tests: 0, reused: true },
  ]);
  expect(first?.measurement.frameMs?.mean).toBe(25);
  expect(first?.measurement.cpuMs?.mean).toBe(5);
  expect(first?.camera).toEqual(camera);
  expect(first?.canvas).toEqual([960, 600]);
  expect(first?.requested).toEqual(intended);
  probe.begin();
  draw(2000, 70);
  draw(2010, 2);
  expect(probe.read()?.measurement.frameMs?.mean).toBe(10);
  expect(probe.read()?.measurement.cpuMs?.mean).toBe(2);
  expect(resetGpuTiming).toHaveBeenCalledTimes(2);
});

test("an unattached or undrawn measurement cannot look complete", () => {
  const probe = createViewMeasurement();
  expect(() => probe.begin()).toThrow(/ready/);
  expect(probe.read()).toBeNull();
});

test("a transition freezes moving samples before endpoint holds and starts a fresh epoch", () => {
  const probe = createViewMeasurement();
  let gpuFrames = 999;
  probe.pilot.attach!({
    adapter: "test",
    stats: () => ({ gpu: { frames: gpuFrames }, memory: {} }),
    resetGpuTiming: () => {
      gpuFrames = 0;
    },
  } as never);
  const start = { target: [0, 0] as const, distance: 50, yaw: 0, pitch: 0.5 };
  const end = { target: [100, 200] as const, distance: 500, yaw: 1, pitch: 1 };
  const draw = (now: number) => {
    const pose = probe.pilot.pose(now)!;
    probe.pilot.frame!({
      now,
      cpuMs: 2,
      camera: { ...pose, target: [...pose.target, 3] },
      simTime: 3,
      canvas: [960, 600],
      counts: () => ({}),
    } as never);
    return pose;
  };
  probe.place(start);
  probe.transition(end, 1000);
  expect(draw(100)).toEqual(start);
  expect(gpuFrames).toBe(0);
  gpuFrames = 12;
  const middle = draw(600);
  expect(middle.target).toEqual([50, 100]);
  expect(middle.distance).toBe(275);
  expect(middle.yaw).toBe(0.5);
  expect(draw(1100)).toEqual(end);
  const result = probe.read();
  expect(result?.complete).toBe(true);
  expect(result?.measurement.phases[0].name).toBe("transition");
  expect(result?.stats.gpu?.frames).toBe(12);
  gpuFrames = 100;
  draw(2000);
  expect(probe.read()).toEqual(result);
  probe.place(end);
  probe.begin();
  draw(3000);
  draw(3020);
  expect(probe.read()?.measurement.phases[0].name).toBe("hold");
  expect(probe.read()?.measurement.frameMs?.mean).toBe(20);
});

test("a terminal stall remains in frame timing and GPU windows cover the earlier path", () => {
  const probe = createViewMeasurement();
  let frames = 500;
  probe.pilot.attach!({
    adapter: "test",
    stats: () => ({ gpu: { frames, meanMs: 4, p95Ms: 6 }, memory: {} }),
    resetGpuTiming: () => {
      frames = 0;
    },
  } as never);
  const pose = { target: [0, 0] as const, distance: 65, yaw: 0, pitch: 1 };
  const draw = (now: number) => {
    const camera = probe.pilot.pose(now)!;
    probe.pilot.frame!({
      now,
      cpuMs: 2,
      camera: { ...camera, target: [...camera.target, 0] },
      simTime: 3,
      canvas: [960, 600],
      counts: () => ({}),
    } as never);
  };
  probe.place(pose);
  probe.transition({ ...pose, distance: 260 }, 3000);
  draw(100);
  frames = 10;
  draw(1100);
  frames = 20;
  draw(2100);
  frames = 30;
  draw(5100);
  const result = probe.read()!;
  expect(result.measurement.frameMs).toMatchObject({ count: 3, max: 3000 });
  expect(result.validGpuSamples).toBe(60);
  expect(result.gpuWindows.map((s) => s.gpu?.frames)).toEqual([10, 20, 30]);
  expect(result.measurement.frameRate?.average).toBe(result.measurement.averageFps);
  expect(probe.current()?.camera.distance).toBe(260);
  expect(result.camera.distance).toBeLessThan(260);
});
