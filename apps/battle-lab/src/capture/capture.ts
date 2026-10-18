// A capture: how saved encounters are filmed, as camera and scenes only (the
// encounter owns the battle). Each scene's reel is a list of slow camera moves
// over a live battle, cut together with dips to black, like a war film's title
// shots. The main menu plays one capture and the capture tool plays any.
// Sampled by elapsed time, so a slow frame never bends a move.
import { clamp } from "math";
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import { categoryMap, type MapEntry } from "@web/maps/catalogue";

/** One move: the camera drifts from `from` to `to`. A tracking shot follows
 *  one of the filming side's units (its index in the encounter's `units`):
 *  its targets are offsets from that unit. */
export interface Shot {
  seconds: number;
  follow?: number;
  from: CameraPose;
  to: CameraPose;
  /** Easing function: "linear" (default) or "cubic" (ease-in-out). */
  ease?: "linear" | "cubic";
  /** Interpolate distance in log space (for smooth zoom feel). */
  logDistance?: boolean;
  /** Duration of the actual zoom; after this, hold the end pose (default: seconds). */
  zoomDuration?: number;
}

export interface Reel {
  /** How long the picture takes to fade to black before a cut, and back after it. */
  fade_s: number;
  shots: readonly Shot[];
}

export interface ReelSample {
  pose: CameraPose;
  /** The unit the target is an offset from; null when the target is the world's. */
  follow: number | null;
  /** 1 is a black picture, 0 the battle unveiled. */
  black: number;
  /** Every shot has played. */
  done: boolean;
}

/** `json` at `at` as an object with only the `known` fields, or a refusal
 *  naming the first field it doesn't know. */
function only(json: unknown, at: string, known: readonly string[]): Record<string, unknown> {
  if (typeof json !== "object" || json === null || Array.isArray(json))
    throw new Error(`capture ${at || "file"}: not an object`);
  for (const field of Object.keys(json))
    if (!known.includes(field))
      throw new Error(`capture ${at ? `${at}.` : ""}${field}: not a capture field`);
  return json as Record<string, unknown>;
}

function pose(json: unknown, at: string): CameraPose {
  const { target, distance, yaw, pitch } = only(json, at, ["target", "distance", "yaw", "pitch"]);
  const numbers = [...(Array.isArray(target) ? target : []), distance, yaw, pitch];
  if (!Array.isArray(target) || target.length !== 2 || !numbers.every(Number.isFinite))
    throw new Error(`capture ${at}: a framing needs a target [x, y], distance, yaw and pitch`);
  if (!((distance as number) > 0))
    throw new Error(`capture ${at}.distance: the camera must stand off its target`);
  return {
    target: [target[0], target[1]],
    distance: distance as number,
    yaw: yaw as number,
    pitch: pitch as number,
  };
}

const SHOT_FIELDS = ["seconds", "follow", "from", "to", "ease", "logDistance", "zoomDuration"];

/** A finite, non-negative number of seconds. */
const isDuration = (value: unknown): value is number =>
  typeof value === "number" && Number.isFinite(value) && value >= 0;

/** `json` as a reel, or a refusal naming the field at fault (under `at`):
 *  every framing finite and standing off its target, every duration finite,
 *  every shot long enough to hold both of its dips, and no field a reel
 *  doesn't have. */
export function validateReel(json: unknown, at = "reel"): Reel {
  const { fade_s, shots } = only(json, at, ["fade_s", "shots"]);
  if (!isDuration(fade_s)) throw new Error(`capture ${at}.fade_s: not a duration`);
  if (!Array.isArray(shots) || shots.length === 0) throw new Error(`capture ${at}.shots: none`);
  return {
    fade_s,
    shots: shots.map((json, i): Shot => {
      const here = `${at}.shots[${i}]`;
      const shot = only(json, here, SHOT_FIELDS);
      const { seconds, follow, ease, logDistance, zoomDuration } = shot;
      if (!(isDuration(seconds) && seconds > 0 && seconds >= 2 * fade_s))
        throw new Error(`capture ${here}.seconds: shorter than its two dips`);
      if (follow !== undefined && !(Number.isInteger(follow) && (follow as number) >= 0))
        throw new Error(`capture ${here}.follow: not a unit id`);
      if (ease !== undefined && ease !== "linear" && ease !== "cubic")
        throw new Error(`capture ${here}.ease: "linear" or "cubic"`);
      if (logDistance !== undefined && typeof logDistance !== "boolean")
        throw new Error(`capture ${here}.logDistance: true or false`);
      if (
        zoomDuration !== undefined &&
        !(isDuration(zoomDuration) && zoomDuration > 0 && zoomDuration <= seconds)
      )
        throw new Error(`capture ${here}.zoomDuration: not a duration within the shot`);
      return {
        seconds,
        ...(follow !== undefined && { follow: follow as number }),
        from: pose(shot.from, `${here}.from`),
        to: pose(shot.to, `${here}.to`),
        ...(ease !== undefined && { ease }),
        ...(logDistance !== undefined && { logDistance }),
        ...(zoomDuration !== undefined && { zoomDuration }),
      };
    }),
  };
}

/** `reel` under reduced motion: its first framing held for the reel's length,
 *  on the unit that framing follows. */
export function stillReel(reel: Reel): Reel {
  const seconds = reel.shots.reduce((sum, shot) => sum + shot.seconds, 0);
  const [{ from, follow }] = reel.shots;
  return {
    fade_s: reel.fade_s,
    shots: [{ seconds, ...(follow !== undefined && { follow }), from, to: from }],
  };
}

/** Cubic ease-in-out: t<0.5 ? 4t³ : 1 - (-2t+2)³/2 */
function cubicEaseInOut(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

/** The reel's framing and veil `elapsed` seconds in. */
export function sampleReel(reel: Reel, elapsed: number): ReelSample {
  let start = 0;
  for (const shot of reel.shots) {
    const end = start + shot.seconds;
    if (elapsed < end) {
      const shotElapsed = elapsed - start;
      const zoomDur = shot.zoomDuration ?? shot.seconds;
      // t goes 0→1 over zoomDuration, then clamps at 1 for any hold period
      const rawT = Math.min(1, shotElapsed / zoomDur);
      // Apply easing
      const e = shot.ease === "cubic" ? cubicEaseInOut(rawT) : rawT;
      const edge = Math.min(shotElapsed, end - elapsed);
      return {
        pose: betweenEased(shot.from, shot.to, e, shot.logDistance ?? false),
        follow: shot.follow ?? null,
        // A reel without dips cuts straight from shot to shot.
        black: reel.fade_s === 0 ? 0 : 1 - clamp(edge / reel.fade_s, 0, 1),
        done: false,
      };
    }
    start = end;
  }
  const last = reel.shots[reel.shots.length - 1];
  return { pose: last.to, follow: last.follow ?? null, black: 1, done: true };
}

/** The shot whose middle a frame `now` seconds into the reel is the first to
 *  reach, after a frame at `previous`; null if it reaches none. A shot's
 *  middle is always unveiled (`validateReel` fits both dips inside it), so
 *  that frame is a clean picture of the battle: a control plate. */
export function plateShot(reel: Reel, previous: number, now: number): number | null {
  let start = 0;
  for (const [i, shot] of reel.shots.entries()) {
    const middle = start + shot.seconds / 2;
    if (previous < middle && middle <= now) return i;
    start += shot.seconds;
  }
  return null;
}

function betweenEased(a: CameraPose, b: CameraPose, s: number, logDistance: boolean): CameraPose {
  const at = (u: number, v: number) => u + (v - u) * s;
  // Log-linear distance: exp(lerp(ln(a), ln(b), s)) for smooth zoom feel
  const dist = logDistance
    ? Math.exp(at(Math.log(a.distance), Math.log(b.distance)))
    : at(a.distance, b.distance);
  return {
    target: [at(a.target[0], b.target[0]), at(a.target[1], b.target[1])],
    distance: dist,
    yaw: at(a.yaw, b.yaw),
    pitch: at(a.pitch, b.pitch),
  };
}

/** A capture's camera rig: the player's `config`, reaching in as close as
 *  the closest framing of `reels` asks (a film may stand nearer its subject
 *  than a player may zoom), at the pitch of the player's nearest. */
export function filmCamera<
  C extends { zoom_min: number; pitch_curve: readonly (readonly [number, number])[] },
>(config: C, reels: readonly Reel[]): C {
  const closest = Math.min(
    ...reels.flatMap((r) => r.shots.flatMap((s) => [s.from.distance, s.to.distance])),
  );
  if (!(closest < config.zoom_min)) return config;
  const [first] = config.pitch_curve;
  return {
    ...config,
    zoom_min: closest,
    pitch_curve: [[closest, first[1]], ...config.pitch_curve],
  };
}

/** One scene of a capture: a saved battle (`map`'s encounter `encounter`,
 *  from `seed`, stepped to `warm_s` before it is filmed) and the reel that
 *  films it. */
export interface CaptureScene {
  map: string;
  encounter: string;
  seed: number;
  warm_s: number;
  reel: Reel;
}

/** A capture: its scenes, played in order. */
export interface Capture {
  scenes: readonly CaptureScene[];
}

/** `json` as a capture, or a refusal naming the field at fault; every
 *  scene's map must be one of `maps`' `capture` maps (a capture films no
 *  test's ground). A `follow` beyond the encounter's units is the sim's
 *  fixture test to refuse: the encounter isn't loaded here. */
export function validateCapture(json: unknown, maps: readonly MapEntry[]): Capture {
  const { scenes } = only(json, "", ["scenes"]);
  if (!Array.isArray(scenes) || scenes.length === 0) throw new Error("capture scenes: none");
  return {
    scenes: scenes.map((json, i) => {
      const at = `scenes[${i}]`;
      const s = only(json, at, ["map", "encounter", "seed", "warm_s", "reel"]);
      const { map, encounter, seed, warm_s } = s;
      for (const [k, v] of [
        ["map", map],
        ["encounter", encounter],
      ] as const)
        if (typeof v !== "string" || !v) throw new Error(`capture ${at}.${k}: a saved map's name`);
      try {
        categoryMap(maps, map as string, "capture");
      } catch (e) {
        throw new Error(`capture ${at}.map: ${(e as Error).message}`);
      }
      if (!Number.isInteger(seed)) throw new Error(`capture ${at}.seed: an integer`);
      if (!isDuration(warm_s)) throw new Error(`capture ${at}.warm_s: not a duration`);
      return {
        map: map as string,
        encounter: encounter as string,
        seed: seed as number,
        warm_s,
        reel: validateReel(s.reel, `${at}.reel`),
      };
    }),
  };
}
