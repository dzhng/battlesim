/** Configuration for cinematic capture mode.
 *  Activated by URL params like `?capture=default` or `?capture=tank-follow`.
 *  When active, the game records video using MediaRecorder and writes an mp4. */

import type { MenuReel } from "../menuReel";

/** Capture mode settings from URL parameters. */
export interface CaptureSettings {
  /** Which camera preset to use. */
  preset: string;
  /** Output resolution width. */
  width: number;
  /** Output resolution height. */
  height: number;
  /** Target frame rate. */
  fps: number;
  /** Whether to include audio in the recording. */
  audio: boolean;
  /** Whether to show info cards (with zoom fade). */
  showCards: boolean;
  /** Maximum recording duration in seconds (safety limit). */
  maxDuration: number;
}

/** Default capture settings. */
export const DEFAULT_CAPTURE: CaptureSettings = {
  preset: "default",
  width: 1920,
  height: 1080,
  fps: 60,
  audio: true,
  showCards: true,
  maxDuration: 120,
};

/** Parse capture settings from URL search params.
 *  Returns null if capture mode is not enabled. */
export function parseCaptureParams(search: string): CaptureSettings | null {
  const params = new URLSearchParams(search);
  const capture = params.get("capture");
  if (!capture) return null;

  return {
    preset: capture === "true" ? "default" : capture,
    width: parseInt(params.get("capture-width") ?? String(DEFAULT_CAPTURE.width), 10),
    height: parseInt(params.get("capture-height") ?? String(DEFAULT_CAPTURE.height), 10),
    fps: parseInt(params.get("capture-fps") ?? String(DEFAULT_CAPTURE.fps), 10),
    audio: params.get("capture-audio") !== "false",
    showCards: params.get("capture-cards") !== "false",
    maxDuration: parseInt(
      params.get("capture-max") ?? String(DEFAULT_CAPTURE.maxDuration),
      10,
    ),
  };
}

/** Check if we're in capture mode from URL. */
export function isCaptureMode(search = window.location.search): boolean {
  return new URLSearchParams(search).has("capture");
}

/** Camera presets for cinematic capture.
 *  Each preset is a MenuReel with shots that can follow units or use fixed positions. */
export interface CapturePreset {
  name: string;
  description: string;
  reel: MenuReel;
}

/** The default cinematic preset: wide top-down transitioning to ground-level tank-follow.
 *  ~37 seconds total, ending at the menu's cinematic ground angle.
 *  This demonstrates the game's scale and detail progression.
 *  Shot 1 shows the engagement area from above, then transitions to following unit 0. */
export const DEFAULT_CINEMATIC_REEL: MenuReel = {
  fade_s: 0.8,
  shots: [
    {
      seconds: 9,
      from: { target: [0, 15], distance: 220, yaw: -0.3, pitch: 0.60 },
      to: { target: [0, 12], distance: 140, yaw: -0.2, pitch: 0.45 },
    },
    {
      seconds: 12,
      follow: 0,
      from: { target: [0, 15], distance: 90, yaw: -0.6, pitch: 0.28 },
      to: { target: [0, 8], distance: 55, yaw: -0.3, pitch: 0.18 },
    },
    {
      seconds: 16,
      follow: 0,
      from: { target: [-1, 8], distance: 45, yaw: -0.2, pitch: 0.15 },
      to: { target: [-1, 8], distance: 30, yaw: 0.2, pitch: 0.13 },
    },
  ],
};

/** Built-in capture presets. */
export const CAPTURE_PRESETS: Record<string, CapturePreset> = {
  default: {
    name: "Default Cinematic",
    description: "Wide top-down view transitioning to ground-level tank-follow shot",
    reel: DEFAULT_CINEMATIC_REEL,
  },
  "top-down": {
    name: "Top-Down Overview",
    description: "High-altitude strategic view",
    reel: {
      fade_s: 0.6,
      shots: [
        {
          seconds: 12,
          from: { target: [0, 15], distance: 180, yaw: 0, pitch: 0.55 },
          to: { target: [5, 20], distance: 160, yaw: 0.4, pitch: 0.50 },
        },
      ],
    },
  },
  "ground-level": {
    name: "Ground Level",
    description: "Low-angle dramatic shot following a unit",
    reel: {
      fade_s: 0.6,
      shots: [
        {
          seconds: 15,
          follow: 0,
          from: { target: [-3, 12], distance: 25, yaw: -1.2, pitch: 0.10 },
          to: { target: [-3, 12], distance: 22, yaw: -0.8, pitch: 0.08 },
        },
      ],
    },
  },
  orbit: {
    name: "Orbit",
    description: "Camera orbits around a central point",
    reel: {
      fade_s: 0.6,
      shots: [
        {
          seconds: 20,
          from: { target: [0, 15], distance: 60, yaw: 0, pitch: 0.25 },
          to: { target: [0, 15], distance: 60, yaw: Math.PI * 2, pitch: 0.25 },
        },
      ],
    },
  },
};

/** Get a capture preset by name, falling back to default. */
export function getCapturePreset(name: string): CapturePreset {
  return CAPTURE_PRESETS[name] ?? CAPTURE_PRESETS.default;
}

/** Calculate the total duration of a reel in seconds. */
export function reelDuration(reel: MenuReel): number {
  return reel.shots.reduce((sum, shot) => sum + shot.seconds, 0);
}
