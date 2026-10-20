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
