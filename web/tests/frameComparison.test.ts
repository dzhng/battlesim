// @vitest-environment node
import { expect, test } from "vitest";
import { compareRuns } from "../src/battle/benchmark/frameComparison";

const run = (client: string, scene: string, ms: number, cpu = 1) => ({
  client,
  scene,
  intervalsMs: Array(100).fill(ms),
  cpuMs: Array(100).fill(cpu),
  gpuMs: null,
});

test("each client and scene is one row: the median run's numbers, and the spread of average FPS across runs", () => {
  const rows = compareRuns([
    run("native", "market-town", 20),
    run("browser", "market-town", 10, 2),
    run("browser", "market-town", 8, 3),
    run("browser", "market-town", 12, 4),
  ]);
  expect(rows.map((r) => [r.client, r.scene, r.runs])).toEqual([
    ["browser", "market-town", 3],
    ["native", "market-town", 1],
  ]);
  const browser = rows[0];
  expect(browser.averageFps).toEqual({ median: 100, min: 1000 / 12, max: 125 });
  expect(browser.frameMs.p95).toBe(10);
  expect(browser.cpuMsP50).toBe(2);
  expect(browser.gpuMsMean).toBeNull();
});

test("a run with no valid interval is refused, never summarised as zero", () => {
  expect(() => compareRuns([{ ...run("native", "x", 10), intervalsMs: [] }])).toThrow(/native x/);
});
