// @vitest-environment node
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { expect, test, vi } from "vitest";
import { createBenchmarkRun } from "@apps/battle-lab/src/benchmark/run";
import { cityContactTour } from "@web/battle/benchmark/camera";
import { CITY_CONTACT } from "@web/battle/benchmark/presets";
import type { BenchmarkReport } from "@web/battle/benchmark/report";

test("benchmark samples reset GPU windows and attribute the outgoing window before changing phase", () => {
  vi.stubGlobal("navigator", { userAgent: "test" });
  vi.stubGlobal("window", { innerWidth: 1280, innerHeight: 800, devicePixelRatio: 1 });
  try {
    const reports: BenchmarkReport[] = [];
    const run = createBenchmarkRun(
      { ...CITY_CONTACT, tour: cityContactTour([10000, 10000]) },
      "short",
      30,
      (r) => reports.push(r),
    );
    let gpu = { frames: 50, meanMs: 99, p95Ms: 99 };
    const resetGpuTiming = vi.fn(() => {
      gpu = { frames: 0, meanMs: 0, p95Ms: 0 };
    });
    run.scripted.pilot.attach!({
      adapter: "test",
      stats: () => ({ gpu, memory: {} }) as never,
      resetGpuTiming,
    });
    const camera: Camera3DParams = {
      target: [0, 0, 0],
      distance: 65,
      yaw: 0,
      pitch: 0.85,
      fovY: 0.8,
      aspect: 1.6,
      near: 1,
    };
    const draw = (now: number) => {
      run.scripted.pilot.pose(now);
      run.scripted.pilot.frame!({
        now,
        cpuMs: 1,
        receiverFit: null,
        camera,
        simTime: 3,
        canvas: [960, 600],
        counts: () => ({}) as never,
      });
    };
    run.scripted.onWarm();
    draw(1000);
    expect(resetGpuTiming).toHaveBeenCalledTimes(1);
    gpu = { frames: 40, meanMs: 5, p95Ms: 6 };
    draw(3000);
    expect(resetGpuTiming).toHaveBeenCalledTimes(2);
    gpu = { frames: 20, meanMs: 7, p95Ms: 8 };
    draw(7100);
    run.cancel();
    expect(reports[0].samples.stats.map((s) => [s.phase, s.gpu?.meanMs])).toEqual([
      ["strategic", 5],
      ["strategic", 7],
    ]);
  } finally {
    vi.unstubAllGlobals();
  }
});
