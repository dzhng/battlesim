// The benchmark's camera tour: keyframed framings
// through named phases, sampled by elapsed time so a slow frame never
// shortens or bends the path. Times are fractions of the run, so the 60 s
// short run flies the same tour as the five-minute run, five times faster.
// The viewport places each sample through `CameraController.place`.
//
// Ported by technique from ~/dev/game/web/src/battle/benchmark/benchmarkCamera.ts
// (see the reuse manifest).
import { clamp } from "math";
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import { smoothstep } from "@packages/renderer-core/src/math";

export const BENCHMARK_PHASES = ["strategic", "pan", "zoom", "ground", "combined", "return"];
export type BenchmarkPhase = (typeof BENCHMARK_PHASES)[number];

/** [run fraction, target x, target y, distance m, yaw rad, pitch rad]. */
export type TourKeyframe = readonly [number, number, number, number, number, number];

export interface BenchmarkTour {
  /** Changing a keyframe or phase changes the workload: bump it. */
  version: string;
  phases: readonly { name: BenchmarkPhase; from: number; to: number }[];
  /** Ascending in time, from 0 to 1. Yaw is unwrapped: it never jumps by 2π. */
  keyframes: readonly TourKeyframe[];
}

// Anchors scouted from the village seed 20260925 with blue on the supported
// script, from tick 6300 (210 s): blue's firing line holds about (400, 812),
// the red garrison (945, 850) in the village, and rounds cross between them.
const OPENING_YAW = -1.57;
const TURN = 2 * Math.PI;
const BLUE: [number, number] = [415, 812];
const VILLAGE: [number, number] = [945, 845];
const BATTLE: [number, number] = [680, 825];

export const BENCHMARK_TOUR: BenchmarkTour = {
  version: "village-contact-6300-v1",
  phases: [
    { name: "strategic", from: 0, to: 0.1 },
    { name: "pan", from: 0.1, to: 0.3 },
    { name: "zoom", from: 0.3, to: 0.5 },
    { name: "ground", from: 0.5, to: 0.7 },
    { name: "combined", from: 0.7, to: 0.9 },
    { name: "return", from: 0.9, to: 1 },
  ],
  keyframes: [
    // Strategic: the whole firing line from high up.
    [0, ...BATTLE, 1300, OPENING_YAW, 0.85],
    [0.08, BATTLE[0] + 20, BATTLE[1], 1200, OPENING_YAW + 0.12, 0.85],
    // Pan: along the line at the default's height, blue to the village and back.
    [0.1, ...BLUE, 260, OPENING_YAW, 0.85],
    [0.17, ...BATTLE, 240, OPENING_YAW - 0.2, 0.85],
    [0.24, ...VILLAGE, 240, OPENING_YAW - 0.4, 0.85],
    [0.3, ...BATTLE, 260, OPENING_YAW - 0.3, 0.85],
    // Zoom: in to each side and out to the whole battle.
    [0.35, ...VILLAGE, 40, OPENING_YAW - 0.6, 0.55],
    [0.42, ...BATTLE, 1200, OPENING_YAW - 0.9, 0.85],
    [0.47, ...BLUE, 40, OPENING_YAW - 1.1, 0.5],
    [0.5, ...BLUE, 250, OPENING_YAW - 1.3, 0.85],
    // Ground: behind blue's line looking east, then in the village looking west.
    [0.53, BLUE[0] - 10, BLUE[1], 40, -Math.PI, 0.25],
    [0.58, BLUE[0] + 30, BLUE[1] + 5, 30, -3.5, 0.2],
    [0.64, ...VILLAGE, 45, -5.9, 0.25],
    [0.7, VILLAGE[0] + 10, VILLAGE[1], 60, -6.3, 0.3],
    // Combined: zoom, pan and turn at once.
    [0.74, ...BATTLE, 900, -6.6, 0.85],
    [0.78, ...BLUE, 70, -7.0, 0.5],
    [0.82, ...BATTLE, 1500, -6.8, 0.85],
    [0.86, ...VILLAGE, 90, -7.3, 0.45],
    [0.9, ...BATTLE, 400, -7.6, 0.85],
    // Return: back to the opening framing, one whole turn later.
    [1, ...BATTLE, 1300, OPENING_YAW - TURN, 0.85],
  ],
};

export interface TourSample {
  phase: BenchmarkPhase;
  pose: CameraPose;
}

/** The tour's framing `elapsedMs` into a run of `durationMs`. */
export function sampleTour(tour: BenchmarkTour, elapsedMs: number, durationMs: number): TourSample {
  const u = clamp(elapsedMs / durationMs, 0, 1);
  const phase = (tour.phases.find((p) => u < p.to) ?? tour.phases.at(-1)!).name;
  const keys = tour.keyframes;
  const next = Math.max(
    1,
    keys.findIndex((k) => k[0] >= u),
  );
  const [a, b] = [keys[next - 1], keys[next]];
  // Eased, no overshoot; a zero-length key span has already arrived.
  const s = b[0] === a[0] ? 1 : smoothstep(a[0], b[0], u);
  const at = (i: number) => a[i] + (b[i] - a[i]) * s;
  return {
    phase,
    pose: { target: [at(1), at(2)], distance: at(3), yaw: at(4), pitch: at(5) },
  };
}
