import { BenchmarkRecording } from "@web/battle/benchmark/recording";
import type { BenchmarkOutcome } from "@web/battle/benchmark/report";
import { sampleReel, type BackdropScene } from "../menuReel";
import type { ScriptedSim } from "../useSimSession";
import type { ViewportPilot } from "../LabViewport";
import type { PresentationSample } from "@web/battle/benchmark/presentationCapture";

export interface ReelResult {
  scene: BackdropScene;
  outcome: BenchmarkOutcome;
  recording: BenchmarkRecording;
  startTick: number;
  endTick: number;
  tickHz: number;
  layout: string;
  adapter: string | null;
  capture: readonly PresentationSample[];
}

/** One complete scene; preparation and inter-scene loading have no frame samples. */
export function createReelRun(scene: BackdropScene, tickHz: number, onDone: (result: ReelResult) => void) {
  const durationMs = scene.reel.shots.reduce((sum, shot) => sum + shot.seconds * 1000, 0);
  const recording = new BenchmarkRecording(durationMs);
  const startTick = Math.round(scene.warm_s * tickHz);
  let endTick = startTick;
  let layout = "";
  let warm = false;
  let finished = false;
  let adapter: string | null = null;
  let stats: Parameters<NonNullable<ViewportPilot["attach"]>>[0]["stats"] | null = null;
  let lastStats = 0;
  const subject = { unitAt: (_id: number): ArrayLike<number> | null => null };
  let lastSubject: readonly number[] = [0, 0];
  let tracked: { follow: number; at: [number, number]; now: number } | null = null;
  const capture: PresentationSample[] = [];
  const track = (follow: number, now: number): readonly number[] => {
    const live = subject.unitAt(follow);
    if (tracked?.follow !== follow) tracked = live && { follow, at: [live[0], live[1]], now };
    if (!tracked) return lastSubject;
    if (live) {
      const k = 1 - Math.exp(-(now - tracked.now) / 1000 / 0.5);
      tracked.at[0] += (live[0] - tracked.at[0]) * k;
      tracked.at[1] += (live[1] - tracked.at[1]) * k;
    }
    tracked.now = now;
    return tracked.at;
  };
  const finish = (outcome: BenchmarkOutcome) => {
    if (finished) return;
    finished = true;
    onDone({ scene, outcome, recording, startTick, endTick, tickHz, layout, adapter, capture });
  };
  const sample = (now: number) => sampleReel(scene.reel, recording.elapsed(now) / 1000);
  const pilot: ViewportPilot = {
    attach(frame) { stats = frame.stats; adapter = frame.adapter; },
    pose(now) {
      const s = sample(now);
      if (s.follow === null) return s.pose;
      lastSubject = Array.from(track(s.follow, now));
      return { ...s.pose, target: [lastSubject[0] + s.pose.target[0], lastSubject[1] + s.pose.target[1]] };
    },
    frame({ now, cpuMs, camera }) {
      if (!warm || finished) return;
      if (!recording.started) { recording.start(now); lastStats = now; return; }
      const pose = { target: [camera.target[0], camera.target[1]] as const, distance: camera.distance, yaw: camera.yaw, pitch: camera.pitch };
      recording.frame({ now, cpuMs, camera: pose, intended: pilot.pose(now)!, phase: scene.map });
      if (stats && now - lastStats >= 2000) {
        const s = stats();
        recording.sample({ elapsedMs: recording.elapsed(now), phase: scene.map, gpu: s.gpu, memory: s.memory, heapBytes: null });
        lastStats = now;
      }
      if (recording.elapsed(now) >= durationMs) finish({ status: "complete", reason: "Full scene complete" });
    },
  };
  const scripted: ScriptedSim = {
    warmTo: startTick,
    onReady(info) { layout = info.layout; },
    onWarm() { warm = true; },
    onTick(t) { endTick = t.tick; recording.tick(t); },
    onPublication(p) {
      if (finished) return;
      const pose = pilot.pose(performance.now());
      if (pose) capture.push({ tick: p.tick, digest: p.digest, publication: p.packed, camera: pose });
    },
  };
  return {
    pilot, scripted, subject, recording, durationMs, captureSamples: () => capture,
    opacity: (now: number) => sample(now).black,
    cancel: () => finish({ status: "cancelled", reason: "Cancelled before the full reel finished" }),
    fail: (reason: string) => finish({ status: "failed", reason }),
  };
}
