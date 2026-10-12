// The browser and native clients compared on the same numbers from the same
// code (`web/fps-compare.mjs`). Each run carries its client's raw frames: the
// browser's from the graphics test's recording (requestAnimationFrame
// timestamps), native's from its bench (`std::time::Instant` at each frame's
// start). Only that clock differs between them.
import { frameTimeSummary, summarize } from "./recording.ts";

/** One run of one client over one scene, after its warm-up. */
export interface FrameRun {
  client: string;
  scene: string;
  intervalsMs: readonly number[];
  cpuMs: readonly number[];
  /** Mean GPU ms per frame, or null where the client measured none. */
  gpuMs: number | null;
}

const median = (values: readonly number[]) => [...values].sort((a, b) => a - b)[values.length >> 1];

/** One row per client and scene: the numbers of the run with the median
 *  average FPS, and the spread of average FPS over all its runs. */
export function compareRuns(runs: readonly FrameRun[]) {
  const groups = new Map<string, FrameRun[]>();
  for (const run of runs) {
    const key = `${run.scene}\u0000${run.client}`;
    groups.set(key, [...(groups.get(key) ?? []), run]);
  }
  return [...groups.keys()].sort().map((key) => {
    const summarised = groups.get(key)!.map((run) => {
      const frames = frameTimeSummary(run.intervalsMs);
      if (!frames) throw new Error(`${run.client} ${run.scene}: no frame interval to summarise`);
      return { run, frames };
    });
    const averages = summarised.map((s) => s.frames.averageFps);
    const middle = median(averages);
    const { run, frames } = summarised.find((s) => s.frames.averageFps === middle)!;
    return {
      client: run.client,
      scene: run.scene,
      runs: summarised.length,
      ...frames,
      averageFps: { median: middle, min: Math.min(...averages), max: Math.max(...averages) },
      cpuMs: summarize(run.cpuMs),
      gpuMsMean: run.gpuMs,
    };
  });
}
