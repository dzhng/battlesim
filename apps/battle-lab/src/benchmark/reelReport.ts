import { BenchmarkRecording } from "@web/battle/benchmark/recording";
import {
  MENU_REEL_WORKLOAD,
  reelFingerprint,
  reelWorkload,
  type ReelWorkload,
} from "@web/battle/benchmark/reels";
import { APP_COMMIT } from "../buildIdentity";
import type { BenchmarkOutcome } from "@web/battle/benchmark/report";
import type { ReelResult } from "./reelRun";
import {
  encodePresentationCapture,
  type PresentationCapture,
} from "@web/battle/benchmark/presentationCapture";
import type {
  DrawnPresentation,
  PresentationLayer,
} from "@web/battle/benchmark/visualPresentation";

const reelMs = (scene: { reel: { shots: readonly { seconds: number }[] } }) =>
  scene.reel.shots.reduce((sum, shot) => sum + shot.seconds * 1000, 0);

/** The report of a run of `reel` (the menu's by default) that filmed
 *  `maps`' scenes (the whole reel by default) and drew `layers`; `plates`
 *  read control plates back, which costs those frames their timing. The report's workload is what ran, so a
 *  run of some scenes never stands for the whole reel; each capture names
 *  its scene within the whole reel. */
export function createReelReport(
  results: readonly ReelResult[],
  {
    reel = MENU_REEL_WORKLOAD,
    maps,
    plates = false,
    layers = [],
    interruption,
  }: {
    reel?: ReelWorkload;
    maps?: readonly string[];
    plates?: boolean;
    layers?: readonly PresentationLayer[];
    interruption?: BenchmarkOutcome;
  } = {},
) {
  const workload = reelWorkload(reel, maps);
  const fingerprint = reelFingerprint(workload);
  const { scenes } = workload;
  const durationMs = scenes.reduce((sum, s) => sum + reelMs(s), 0);
  const merged = new BenchmarkRecording(durationMs);
  let offset = 0;
  for (const result of results) {
    merged.ticks.push(...result.recording.ticks);
    merged.frames.push(
      ...result.recording.frames.map((f) => ({ ...f, elapsedMs: f.elapsedMs + offset })),
    );
    merged.stats.push(
      ...result.recording.stats.map((s) => ({ ...s, elapsedMs: s.elapsedMs + offset })),
    );
    offset += result.recording.frames.at(-1)?.elapsedMs ?? 0;
  }
  let phaseStart = 0;
  const phases = scenes.map((s) => {
    const from = phaseStart / durationMs;
    phaseStart += reelMs(s);
    return { name: s.map, from, to: phaseStart / durationMs };
  });
  const complete =
    results.length === scenes.length && results.every((r) => r.outcome.status === "complete");
  const presentationCaptures: PresentationCapture[] = results.map((r) => ({
    schema: "battle-presentation-capture",
    workload: {
      id: reel.id,
      fingerprint: reelFingerprint(reel),
      scene: r.scene.map,
      map: r.scene.map,
      encounter: r.scene.encounter,
      seed: r.scene.seed,
    },
    tickHz: r.tickHz,
    warmTick: r.startTick,
    side: "blue",
    layout: r.layout,
    samples: r.capture,
    frames: r.frames,
  }));
  // What each scene drew its frames with; null for a scene that drew none.
  const drawnPresentations: (DrawnPresentation | null)[] = results.map(
    (r) => r.lens && { lens: r.lens, layers },
  );
  const host = typeof navigator === "undefined" ? null : navigator;
  const viewport =
    typeof window === "undefined"
      ? null
      : { width: window.innerWidth, height: window.innerHeight, dpr: window.devicePixelRatio };
  return {
    kind: "graphics-test" as const,
    canonical: {
      workloadId: workload.id,
      workloadVersion: workload.version,
      fingerprint,
      subjectTrackingLagSeconds: 0.5,
      plateComposition: "standalone_center" as const,
      sceneIds: scenes.map((scene) => scene.map),
    },
    identity: {
      userAgent: host?.userAgent ?? "unknown",
      viewport,
      dpr: viewport?.dpr ?? 1,
      adapters: [...new Set(results.map((result) => result.adapter))],
    },
    capture: {
      client: "browser" as const,
      build: APP_COMMIT,
      resolution:
        typeof window === "undefined"
          ? null
          : { width: window.innerWidth, height: window.innerHeight, dpr: window.devicePixelRatio },
      quality: "current-client-settings" as const,
      presentation: {
        clock: "wall_elapsed" as const,
        subjectTrackingLagSeconds: 0.5,
        plateComposition: "standalone_center" as const,
      },
      /** Reading a plate back stalls its frame, so a run with plates measures nothing. */
      frameTimingComparable: !plates,
    },
    workload,
    fingerprint,
    durationMs,
    recordedMs: offset,
    outcome:
      interruption ??
      (complete
        ? { status: "complete" as const, reason: "Every scene complete" }
        : results.at(-1)?.outcome.status === "failed"
          ? results.at(-1)!.outcome
          : { status: "cancelled" as const, reason: "Partial reel" }),
    measurement:
      "Frame intervals measure rendered requestAnimationFrame cadence, not physical display scan-out. Scene preparation is excluded. Browser refresh pacing may cap FPS; GPU timings are reported separately when available.",
    scenes: results.map((r) => ({
      scene: r.scene,
      startTick: r.startTick,
      endTick: r.endTick,
      outcome: r.outcome,
      adapter: r.adapter,
      ...r.recording.report([r.scene.map]),
    })),
    presentationCaptureJson: presentationCaptures.map(encodePresentationCapture),
    drawnPresentations,
    ...merged.report(phases.map((p) => p.name)),
    tour: { phases },
    samples: { frames: merged.frames, stats: merged.stats },
  };
}
export type ReelReport = ReturnType<typeof createReelReport>;
