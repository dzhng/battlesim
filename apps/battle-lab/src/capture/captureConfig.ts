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
 *  This demonstrates the game's scale and detail progression. */
export const DEFAULT_CINEMATIC_REEL: MenuReel = {
  fade_s: 0.8,
  shots: [
    {
      seconds: 6,
      from: { target: [0, 0], distance: 150, yaw: -0.5, pitch: 0.45 },
      to: { target: [0, 0], distance: 120, yaw: -0.3, pitch: 0.40 },
    },
    {
      seconds: 8,
      follow: 0,
      from: { target: [0, 10], distance: 80, yaw: -0.8, pitch: 0.30 },
      to: { target: [0, 5], distance: 50, yaw: -0.5, pitch: 0.20 },
    },
    {
      seconds: 10,
      follow: 0,
      from: { target: [-2, 8], distance: 35, yaw: -0.3, pitch: 0.14 },
      to: { target: [-2, 8], distance: 28, yaw: 0.1, pitch: 0.12 },
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
          from: { target: [0, 0], distance: 180, yaw: 0, pitch: 0.55 },
          to: { target: [50, 30], distance: 160, yaw: 0.4, pitch: 0.50 },
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
          from: { target: [0, 0], distance: 60, yaw: 0, pitch: 0.25 },
          to: { target: [0, 0], distance: 60, yaw: Math.PI * 2, pitch: 0.25 },
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
