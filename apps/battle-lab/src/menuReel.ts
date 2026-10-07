// The main menu's backdrop reel: a list of slow camera moves over a live
// battle, cut together with dips to black, like a war film's title shots.
// Sampled by elapsed time, so a slow frame never bends a move.
import { clamp } from "math";
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import { categoryMap, type MapEntry } from "@web/maps/catalogue";

/** One move: the camera drifts at a steady pace from `from` to `to`. A
 *  tracking shot follows one unit: its targets are offsets from that unit. */
export interface MenuShot {
  seconds: number;
  follow?: number;
  from: CameraPose;
  to: CameraPose;
}

export interface MenuReel {
  /** How long the picture takes to fade to black before a cut, and back after it. */
  fade_s: number;
  shots: readonly MenuShot[];
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

/** `json` as a reel, or a refusal naming the field at fault: every framing
 *  finite, and every shot long enough to hold both of its dips. */
export function validateReel(json: {
  fade_s: number;
  shots: { seconds: number; follow?: number; from: unknown; to: unknown }[];
}): MenuReel {
  const pose = (p: unknown, at: string): CameraPose => {
    const { target, distance, yaw, pitch } = p as CameraPose;
    const numbers = [...(Array.isArray(target) ? target : []), distance, yaw, pitch];
    if (!Array.isArray(target) || target.length !== 2 || !numbers.every(Number.isFinite))
      throw new Error(`menu reel ${at}: a framing needs a target [x, y], distance, yaw and pitch`);
    return { target: [target[0], target[1]], distance, yaw, pitch };
  };
  if (!(json.fade_s >= 0)) throw new Error("menu reel fade_s: not a duration");
  if (json.shots.length === 0) throw new Error("menu reel shots: none");
  return {
    fade_s: json.fade_s,
    shots: json.shots.map((shot, i) => {
      if (!(shot.seconds > 0 && shot.seconds >= 2 * json.fade_s))
        throw new Error(`menu reel shots[${i}].seconds: shorter than its two dips`);
      if (shot.follow !== undefined && !(Number.isInteger(shot.follow) && shot.follow >= 0))
        throw new Error(`menu reel shots[${i}].follow: not a unit id`);
      return {
        seconds: shot.seconds,
        ...(shot.follow !== undefined && { follow: shot.follow }),
        from: pose(shot.from, `shots[${i}].from`),
        to: pose(shot.to, `shots[${i}].to`),
      };
    }),
  };
}

/** The reel's framing and veil `elapsed` seconds in. */
export function sampleReel(reel: MenuReel, elapsed: number): ReelSample {
  let start = 0;
  for (const shot of reel.shots) {
    const end = start + shot.seconds;
    if (elapsed < end) {
      const edge = Math.min(elapsed - start, end - elapsed);
      return {
        pose: between(shot.from, shot.to, (elapsed - start) / shot.seconds),
        follow: shot.follow ?? null,
        black: 1 - clamp(edge / reel.fade_s, 0, 1),
        done: false,
      };
    }
    start = end;
  }
  const last = reel.shots[reel.shots.length - 1];
  return { pose: last.to, follow: last.follow ?? null, black: 1, done: true };
}

function between(a: CameraPose, b: CameraPose, s: number): CameraPose {
  const at = (u: number, v: number) => u + (v - u) * s;
  return {
    target: [at(a.target[0], b.target[0]), at(a.target[1], b.target[1])],
    distance: at(a.distance, b.distance),
    yaw: at(a.yaw, b.yaw),
    pitch: at(a.pitch, b.pitch),
  };
}

/** The backdrop's camera rig: the player's `config`, reaching in as close as
 *  the closest framing of `reels` asks (a film may stand nearer its subject
 *  than a player may zoom), at the pitch of the player's nearest. */
export function filmCamera<
  C extends { zoom_min: number; pitch_curve: readonly (readonly [number, number])[] },
>(config: C, reels: readonly MenuReel[]): C {
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

/** One scene of the menu's backdrop: a saved battle (`map`'s encounter
 *  `encounter`, from `seed`, stepped to `warm_s` before it is filmed) and
 *  the reel that films it. */
export interface BackdropScene {
  map: string;
  encounter: string;
  seed: number;
  warm_s: number;
  reel: MenuReel;
}

/** The menu's backdrop: its scenes, played in order, then over again. */
export interface Backdrop {
  scenes: readonly BackdropScene[];
}

/** `json` as a backdrop, or a refusal naming the field at fault. */
/** `json` as the menu's backdrop; every scene's map must be one of `maps`'
 *  `menu` maps (the backdrop films no test's ground). */
export function validateBackdrop(
  json: {
    scenes: {
      map: string;
      encounter: string;
      seed: number;
      warm_s: number;
      reel: Parameters<typeof validateReel>[0];
    }[];
  },
  maps: readonly MapEntry[],
): Backdrop {
  if (!json.scenes?.length) throw new Error("menu backdrop scenes: none");
  return {
    scenes: json.scenes.map((s, i) => {
      for (const k of ["map", "encounter"] as const)
        if (typeof s[k] !== "string" || !s[k])
          throw new Error(`menu backdrop scenes[${i}].${k}: a saved map's name`);
      try {
        categoryMap(maps, s.map, "menu");
      } catch (e) {
        throw new Error(`menu backdrop scenes[${i}].map: ${(e as Error).message}`);
      }
      if (!Number.isInteger(s.seed)) throw new Error(`menu backdrop scenes[${i}].seed: an integer`);
      if (!(s.warm_s >= 0)) throw new Error(`menu backdrop scenes[${i}].warm_s: not a duration`);
      return {
        map: s.map,
        encounter: s.encounter,
        seed: s.seed,
        warm_s: s.warm_s,
        reel: validateReel(s.reel),
      };
    }),
  };
}
