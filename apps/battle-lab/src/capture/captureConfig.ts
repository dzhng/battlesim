/** Configuration for cinematic capture mode.
 *  Activated by URL params like `?capture=default` or `?capture=tank-follow`.
 *  When active, the game records video using MediaRecorder and writes an mp4. */

import type { MenuReel } from "../menuReel";

/** Capture mode: "browser" for canvas-based recording, "page" for CDP page-level capture. */
export type CaptureMode = "browser" | "page";

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
  /** Whether to apply the color grade (false for before/after comparisons). */
  grade: boolean;
  /** Maximum recording duration in seconds (safety limit). */
  maxDuration: number;
  /** Capture mode: "browser" for canvas-based, "page" for CDP page-level (includes DOM overlays). */
  mode: CaptureMode;
}

/** Default capture settings. */
export const DEFAULT_CAPTURE: CaptureSettings = {
  preset: "default",
  width: 1920,
  height: 1080,
  fps: 60,
  audio: true,
  showCards: true,
  grade: true,
  maxDuration: 120,
  mode: "page",
};

/** Parse capture settings from URL search params.
 *  Returns null if capture mode is not enabled. */
export function parseCaptureParams(search: string): CaptureSettings | null {
  const params = new URLSearchParams(search);
  const capture = params.get("capture");
  if (!capture) return null;

  const modeParam = params.get("capture-mode");
  const mode: CaptureMode = modeParam === "browser" ? "browser" : "page";

  return {
    preset: capture === "true" ? "default" : capture,
    width: parseInt(params.get("capture-width") ?? String(DEFAULT_CAPTURE.width), 10),
    height: parseInt(params.get("capture-height") ?? String(DEFAULT_CAPTURE.height), 10),
    fps: parseInt(params.get("capture-fps") ?? String(DEFAULT_CAPTURE.fps), 10),
    audio: params.get("capture-audio") !== "false",
    showCards: params.get("capture-cards") !== "false",
    grade: params.get("capture-grade") !== "false",
    maxDuration: parseInt(params.get("capture-max") ?? String(DEFAULT_CAPTURE.maxDuration), 10),
    mode,
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

/** The hero camera: a single continuous zoom from strategic height to ground level.
 *  39s total: 1s preroll (static start pose), 32s eased zoom, 6s hold at end pose.
 *  The 1s preroll ensures the smoothed follow target is initialized before zoom starts.
 *  capture.mjs cuts 0.5s from both video and audio, trimming any stale frames.
 *  Follows unit 1 (the driving tank) with target offset [-1, 8].
 *  Distance is log-linear for smooth zoom feel; yaw/pitch/target are linear.
 *  Info cards fade in as the camera descends (90m → 48m, handled by the game). */
export const DEFAULT_CINEMATIC_REEL: MenuReel = {
  fade_s: 0,
  shots: [
    // 1s preroll: hold start pose while follow target initializes
    {
      seconds: 1.0,
      follow: 1,
      from: { target: [-1, 8], distance: 700, yaw: 0.174, pitch: 1.2 },
      to: { target: [-1, 8], distance: 700, yaw: 0.174, pitch: 1.2 },
    },
    // 38s main shot: 32s zoom + 6s hold
    {
      seconds: 38.0,
      follow: 1,
      from: { target: [-1, 8], distance: 700, yaw: 0.174, pitch: 1.2 },
      to: { target: [-1, 8], distance: 39.6, yaw: -0.426, pitch: 0.147 },
      ease: "cubic",
      logDistance: true,
      zoomDuration: 32,
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
    description: "High-altitude strategic view following the lead tank",
    reel: {
      fade_s: 0.6,
      shots: [
        {
          seconds: 12,
          follow: 1,
          from: { target: [0, 0], distance: 180, yaw: 0, pitch: 0.55 },
          to: { target: [20, 30], distance: 160, yaw: 0.4, pitch: 0.5 },
        },
      ],
    },
  },
  "ground-level": {
    name: "Ground Level",
    description: "Low-angle dramatic shot following the lead tank",
    reel: {
      fade_s: 0.6,
      shots: [
        {
          seconds: 15,
          follow: 1,
          from: { target: [-3, 12], distance: 25, yaw: -1.2, pitch: 0.1 },
          to: { target: [-3, 12], distance: 22, yaw: -0.8, pitch: 0.08 },
        },
      ],
    },
  },
  orbit: {
    name: "Orbit",
    description: "Camera orbits around the lead tank",
    reel: {
      fade_s: 0.6,
      shots: [
        {
          seconds: 20,
          follow: 1,
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

/** Resolve a follow value to an actual unit ID from the observation.
 *  The reel's follow value is interpreted as a "slot" meaning:
 *  - If a unit with that exact ID exists, use it
 *  - Otherwise, find the Nth tracked vehicle (0=first, 1=second, etc.)
 *  Returns the unit ID or null if no match found.
 *  Both camera and tracker should use this for consistency. */
export function resolveFollowId(
  follow: number | null,
  observation: { own: readonly { id: number; kind: string }[] },
  isTrackedVehicle: (kind: string) => boolean,
): number | null {
  if (follow === null) return null;

  // First, check if a unit with this exact ID exists
  const exact = observation.own.find((u) => u.id === follow);
  if (exact) return exact.id;

  // Otherwise, treat follow as an index into tracked vehicles
  const trackedVehicles = observation.own.filter((u) => isTrackedVehicle(u.kind));
  const byIndex = trackedVehicles[follow];
  if (byIndex) return byIndex.id;

  // Fall back to first tracked vehicle
  if (trackedVehicles.length > 0) return trackedVehicles[0].id;

  // Last resort: first unit
  return observation.own[0]?.id ?? null;
}

/** Calculate the total duration of a reel in seconds. */
export function reelDuration(reel: MenuReel): number {
  return reel.shots.reduce((sum, shot) => sum + shot.seconds, 0);
}

/** A logged weapon fire event for correlating audio with visuals. */
export interface WeaponFireEvent {
  /** Capture-relative time in seconds when the shot was detected. */
  time_s: number;
  /** The unit that fired (id). */
  unit_id: number;
  /** The unit's kind (type name). */
  unit_kind: string;
  /** Which mount fired (index into unit's mounts). */
  mount: number;
  /** Weapon kind (round type name like "tank_ap", "rifle", "hmg"). */
  weapon_kind: string;
  /** Position where the shot originated. */
  position: readonly [number, number, number];
  /** Whether this is the followed unit. */
  is_followed: boolean;
}

/** Resolver for weapon kinds from unit type and mount index. */
export interface WeaponKindResolver {
  /** Look up the primary weapon kind for a unit type's mount. */
  getWeaponKind(unitKind: string, mountIndex: number): string | null;
}

/** Tracks weapon fires by watching shot counts on unit mounts. */
export class WeaponFireTracker {
  private shotCounts = new Map<string, number>();
  private events: WeaponFireEvent[] = [];
  private startTime: number | null = null;
  private followedUnit: number | null = null;
  private initialized = false;
  private resolver: WeaponKindResolver | null = null;

  /** Set the weapon kind resolver (call once at construction). */
  setResolver(resolver: WeaponKindResolver) {
    this.resolver = resolver;
  }

  /** Set which unit is being followed (for logging). */
  setFollowedUnit(id: number | null) {
    this.followedUnit = id;
  }

  /** Update with current observation. Call each frame during capture.
   *  Returns any new fire events detected this frame. */
  update(
    observation: {
      own: readonly {
        id: number;
        kind: string;
        position: readonly [number, number, number];
        weaponPoses: readonly { mount: number; shots: number }[];
      }[];
    },
    now: number,
  ): WeaponFireEvent[] {
    if (this.startTime === null) this.startTime = now;

    // On first frame, snapshot initial shot counts without logging events.
    // This prevents false positives when units start with non-zero shot counts.
    if (!this.initialized) {
      this.initialized = true;
      for (const unit of observation.own) {
        for (const pose of unit.weaponPoses) {
          const key = `${unit.id}:${pose.mount}`;
          this.shotCounts.set(key, pose.shots);
        }
      }
      return [];
    }

    const elapsed = (now - this.startTime) / 1000;
    const newEvents: WeaponFireEvent[] = [];

    for (const unit of observation.own) {
      for (const pose of unit.weaponPoses) {
        const key = `${unit.id}:${pose.mount}`;
        const prev = this.shotCounts.get(key) ?? 0;
        const current = pose.shots;

        if (current > prev) {
          const weaponKind = this.resolver?.getWeaponKind(unit.kind, pose.mount) ?? "unknown";
          const event: WeaponFireEvent = {
            time_s: elapsed,
            unit_id: unit.id,
            unit_kind: unit.kind,
            mount: pose.mount,
            weapon_kind: weaponKind,
            position: unit.position,
            is_followed: unit.id === this.followedUnit,
          };
          newEvents.push(event);
          this.events.push(event);
        }

        this.shotCounts.set(key, current);
      }
    }

    return newEvents;
  }

  /** Get all fire events logged so far. */
  getEvents(): WeaponFireEvent[] {
    return [...this.events];
  }

  /** Reset the tracker for a new capture. */
  reset() {
    this.shotCounts.clear();
    this.events = [];
    this.startTime = null;
    this.initialized = false;
  }
}
