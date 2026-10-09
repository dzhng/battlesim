import { BenchmarkRecording } from "@web/battle/benchmark/recording";
import { MENU_REEL_WORKLOAD, menuReelFingerprint } from "@web/battle/benchmark/menuReel";
import { APP_COMMIT } from "../buildIdentity";
import type { BenchmarkOutcome } from "@web/battle/benchmark/report";
import type { ReelResult } from "./reelRun";

export function createReelReport(results: readonly ReelResult[], interruption?: BenchmarkOutcome) {
  const durationMs = MENU_REEL_WORKLOAD.scenes.reduce((sum, s) => sum + s.reel.shots.reduce((n, shot) => n + shot.seconds * 1000, 0), 0);
  const merged = new BenchmarkRecording(durationMs);
  let offset = 0;
  for (const result of results) {
    merged.ticks.push(...result.recording.ticks);
    merged.frames.push(...result.recording.frames.map((f) => ({ ...f, elapsedMs: f.elapsedMs + offset })));
    merged.stats.push(...result.recording.stats.map((s) => ({ ...s, elapsedMs: s.elapsedMs + offset })));
    offset += result.recording.frames.at(-1)?.elapsedMs ?? 0;
  }
  let phaseStart = 0;
  const phases = MENU_REEL_WORKLOAD.scenes.map((s) => {
    const from = phaseStart / durationMs;
    phaseStart += s.reel.shots.reduce((sum, shot) => sum + shot.seconds * 1000, 0);
    return { name: s.map, from, to: phaseStart / durationMs };
  });
  const complete = results.length === MENU_REEL_WORKLOAD.scenes.length && results.every((r) => r.outcome.status === "complete");
  return {
    kind: "graphics-test" as const,
    capture: {
      client: "browser" as const,
      build: APP_COMMIT,
      resolution: typeof window === "undefined" ? null : { width: window.innerWidth, height: window.innerHeight, dpr: window.devicePixelRatio },
      quality: "current-client-settings" as const,
      presentation: { clock: "wall_elapsed" as const, subjectTrackingLagSeconds: 0.5, plateComposition: "standalone_center" as const },
    },
    workload: MENU_REEL_WORKLOAD,
    fingerprint: menuReelFingerprint(),
    durationMs,
    recordedMs: offset,
    outcome: interruption ?? (complete ? { status: "complete" as const, reason: "Full reel complete" } : (results.at(-1)?.outcome.status === "failed" ? results.at(-1)!.outcome : { status: "cancelled" as const, reason: "Partial reel" })),
    measurement: "Frame intervals measure rendered requestAnimationFrame cadence, not physical display scan-out. Scene preparation is excluded. Browser refresh pacing may cap FPS; GPU timings are reported separately when available.",
    scenes: results.map((r) => ({ scene: r.scene, startTick: r.startTick, endTick: r.endTick, outcome: r.outcome, adapter: r.adapter, ...r.recording.report([r.scene.map]) })),
    ...merged.report(phases.map((p) => p.name)),
    tour: { phases },
    samples: { frames: merged.frames, stats: merged.stats },
  };
}
export type ReelReport = ReturnType<typeof createReelReport>;
