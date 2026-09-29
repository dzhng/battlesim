// `presentation.audio`: every level, curve, budget and per-kind sound table
// the battle's sound reads, keyed like `presentation.effects` (round kinds,
// hit kinds, unit kinds, smoke kinds, cue categories), each table with a
// `default` row so a new kind sounds like something until its row is added.
// Sound names are the synthesised bank's (`synth.ts` `SOUNDS`).
import { requireDefaults } from "@packages/renderer-core/src/kindTable";

/** The mix buses under the master. */
export type Bus = "units" | "effects" | "ambience";
export const BUSES: readonly Bus[] = ["units", "effects", "ambience"];

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

/** A vehicle kind's loops: engine (idle to load), running gear (tracks or
 *  wheels, by speed), turret traverse, and reverse whine. A null sound is none. */
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

export interface AudioPresentation {
  /** Bus levels, and the master's. */
  buses: Record<Bus | "master", number>;
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
  /** A flying round's motor by round kind; a kind without a row has none. */
  motors: Record<string, OneShot>;
  vehicles: Record<string, VehicleSound>;
  footsteps: OneShot & { stride_m: number; max_step_m: number };
  /** Fire loops by smoke-source kind. */
  fires: Record<string, FireSound>;
  /** The countryside bed, never positional. */
  ambience: OneShot;
  /** Unseen enemies, from hearing cues: a sound by `category` or
   *  `category_moving`, its gain by band, the far band muffled, and a same cue
   *  not repeated within `repeat_s`. Played from the cue's direction only. */
  cues: {
    sounds: Record<string, string>;
    near_gain: number;
    far_gain: number;
    far_lowpass_hz: number;
    repeat_s: number;
  };
}

const TABLES = ["shots", "impacts", "impact_scale", "blasts", "vehicles", "fires"] as const;

/** Throws on a table without its `default`, or a budget that leaves no transients. */
export function validateAudio(p: AudioPresentation): AudioPresentation {
  requireDefaults("presentation.audio", p, TABLES);
  if (!p.cues.sounds.default) throw new Error("presentation.audio.cues.sounds needs a default");
  if (!(p.budget.voices > p.budget.loops && p.budget.loops >= 0))
    throw new Error("presentation.audio.budget: voices must exceed loops");
  const { floor, cull } = p.distance;
  if (!(floor >= 0 && floor <= 1 && cull >= 0))
    throw new Error("presentation.audio.distance: floor must be 0 to 1, cull at least 0");
  const { near_hz, far_hz, far_m, wet, reverb_s, attack_s } = p.air;
  if (!(near_hz >= far_hz && far_hz > 0 && far_m > 0))
    throw new Error("presentation.audio.air: near_hz ≥ far_hz > 0 and far_m > 0");
  if (!(wet >= 0 && wet <= 1 && reverb_s > 0 && attack_s >= 0))
    throw new Error("presentation.audio.air: wet 0 to 1, reverb_s > 0, attack_s ≥ 0");
  return p;
}
