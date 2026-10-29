// `audio.json`: every level, curve, budget and per-kind sound table
// the battle's sound reads, keyed like `presentation.effects` (round kinds,
// hit kinds, unit kinds, smoke kinds, cue categories), each table with a
// `default` row so a new kind sounds like something until its row is added.
// Sound names are the synthesised bank's (`synthSounds.gen.ts` `SOUNDS`).
import type { BattleMusicRules } from "./music/battleMusic";
import type { ChatterRules } from "./voices/chatter";

/** The mixer's buses under the master. Music is not one of them: it streams
 *  through `MusicPlayer`, which takes its level from `buses.music`. */
export type Bus = "units" | "effects" | "ambience" | "voices";
export const BUSES: readonly Bus[] = ["units", "effects", "ambience", "voices"];

/** A gunshot or blast: a near and a far sound, switched at `far_m`. */
export interface NearFar {
  near: string;
  far: string;
  gain: number;
  far_m: number;
}

export interface OneShot {
  sound: string;
  gain: number;
}

/** How far a bus dips (a share of its level), and how fast it dips and
 *  recovers (time constants, seconds): the music under a voice line, the
 *  ambience under a loud shot. */
export interface Duck {
  depth: number;
  attack_s: number;
  release_s: number;
}

/** A vehicle kind's loops: engine (idle to load), running gear (tracks or
 *  wheels, by speed), turret traverse, and reverse beeper. A null sound is none. */
export interface VehicleSound {
  engine: string;
  /** Engine playback rate at idle and at full load. */
  idle_rate: number;
  load_rate: number;
  /** Engine gain at idle and at full load. */
  idle_gain: number;
  load_gain: number;
  running: string | null;
  running_gain: number;
  /** The speed at which load and running gear are full, metres a second. */
  full_speed_mps: number;
  turret: string | null;
  turret_gain: number;
  /** The traverse rate at which the turret's whine is full, radians a second. */
  full_traverse_rps: number;
  reverse: string | null;
  reverse_gain: number;
}

/** A burning source's loop: full while it burns, `smoulder_gain` while it smoulders. */
export interface FireSound {
  sound: string;
  gain: number;
  smoulder_gain: number;
}

/** The master's limiter (hard knee): over `threshold_db` (dBFS) the level is
 *  compressed by `ratio`, reacting over `attack_s` and recovering over
 *  `release_s`; what its attack still lets over is clipped hard at `clip_db`. */
export interface Limiter {
  threshold_db: number;
  attack_s: number;
  release_s: number;
  ratio: number;
  clip_db: number;
}

export interface AudioMix {
  /** Bus levels, and the master's. */
  buses: Record<Bus | "master" | "music", number>;
  /** The master's limiter and clip, the last stage before the output. */
  limiter: Limiter;
  /** Where the listener stands: this share of the way from the camera's
   *  target to its eye (0 at the ground point looked at, 1 at the eye). */
  listener_eye_share: number;
  /** Distance attenuation: full within `ref_m`, then inverse (`rolloff`)
   *  down toward `floor`, a share of the sound's gain it never drops below
   *  within `max_m` (so the side always hears its own battle, even from the
   *  farthest zoom), and silent past `max_m`. A voice quieter than `cull` at
   *  the listener is not started. */
  distance: { ref_m: number; rolloff: number; max_m: number; floor: number; cull: number };
  /** The colour of distance, growing from nothing at the listener to full at
   *  `far_m`: the air's low-pass from `near_hz` to `far_hz` (log-spaced), a
   *  reverb send up to `wet` (an outdoor tail `reverb_s` long), and onsets
   *  softened by an attack ramp up to `attack_s`. */
  air: {
    near_hz: number;
    far_hz: number;
    far_m: number;
    wet: number;
    reverb_s: number;
    attack_s: number;
  };
  /** Voices at once, of which `loops` may be loops (engines, fires, motors). */
  budget: { voices: number; loops: number };
  /** The presentation clock standing still this long (wall seconds) is a hold
   *  (pause): transients fall silent, loops hold. */
  hold_s: number;
  /** A transient more than this late (a clock jump) is dropped, not played. */
  late_s: number;
  /** Gunfire by round kind. */
  shots: Record<string, NearFar>;
  /** Impacts by hit kind, and their gain by round kind. */
  impacts: Record<string, OneShot>;
  impact_scale: Record<string, number>;
  ricochet: OneShot;
  /** Blasts by round kind. */
  blasts: Record<string, NearFar>;
  /** A hull cooking off: the boom of each fireball, and its turret's clang
   *  as it strikes the deck, heard as a round of kind `as` striking `hit`
   *  (the catalog's contact sound for it), at `gain`. */
  cook_off: { blast: NearFar; landing: { hit: string; as: string; gain: number } };
  /** A flying round's motor by round kind; a kind without a row has none. */
  motors: Record<string, OneShot>;
  vehicles: Record<string, VehicleSound>;
  footsteps: OneShot & { stride_m: number; max_step_m: number };
  /** Fire loops by smoke-source kind. */
  fires: Record<string, FireSound>;
  /** Battle music: its mood direction and its ducking while voice lines play (`presentation::sound::music`). */
  music: BattleMusicRules;
  /** Unit voice lines: how many sound at once, how long one may wait to start,
   *  and the window in which a group's answers collapse to one (`Chatter`). */
  voices: ChatterRules;
  /** The bed, never positional. Under a sound at least `duck_over` loud at
   *  the listener the ambience bus dips by `duck`, and recovers once the dip
   *  is reached (`presentation::sound::frame`). */
  ambience: OneShot & { duck: Duck; duck_over: number };
  /** One alarm when the first fixed-wing aircraft enters a side's field. */
  air_alarm: OneShot;
  /** A positional warning when a visible missile is steering onto an own unit. */
  missile_warning: OneShot;
  /** A positional burst when an aircraft releases a flare. */
  flare_deploy: OneShot;
  /** Unseen enemies, from hearing cues: a sound by `category` or
   *  `category_moving` (a category with neither is silent), a shout from the
   *  enemy's pool, its gain by band, the far band muffled, and a same cue not
   *  repeated within `repeat_s`. Played from the cue's direction only. */
  cues: {
    sounds: Record<string, string>;
    /** A shout: a stressed bark of the enemy faction's (`ShoutPool`, up to
     *  `takes_per_voice` of each shouting voice), at `gain` times its band's,
     *  through a `lowpass_hz` low-pass and a `wet` reverb send. */
    shout: { gain: number; lowpass_hz: number; wet: number; takes_per_voice: number };
    near_gain: number;
    far_gain: number;
    far_lowpass_hz: number;
    repeat_s: number;
  };
}
