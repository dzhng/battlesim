// Which voice lines play, and when, is decided in Rust
// (`presentation::sound::voices`, through the presentation module): the line
// cap, tier then distance, the late drop, self-overlap, group collapse and the
// seeded take. A battle and the sound workbench's chatter preview both ask it.
// This is its binding: it tells Rust which takes are loaded and what units
// ask for, and hands back the start commands a player carries out on the
// voices bus, with the lines started and dropped. The presentation module
// must be loaded first (`loadPresentation`).
import { Chatter as RustChatter, type SoundFrame as RustSoundFrame } from "@wasm/renderer_browser";
import type { VoiceSpec as LineSpec } from "../soundFrame";
import type { Intensity, Trigger, VoiceSpec } from "./voiceCatalog";

/** Who hears a line first: the owner's answers, then own units' shouts, then others'. */
export type VoiceTier = "response" | "own_bark" | "other_bark";

/** Something a unit wants to say. */
export interface VoiceRequest {
  tier: VoiceTier;
  /** A stable name for the unit speaking: it never overlaps itself, and it keeps its voice. */
  speaker: string;
  /** The voice set it speaks with. */
  set: string;
  trigger: Trigger;
  intensity: Intensity;
  /** Where it speaks; without one it is as near as can be (an owner's answer). */
  position?: readonly number[];
  /** Requests sharing this key within `group_s` are one answer. */
  groupKey?: string;
  /** The moment it was asked for, on the player's clock. */
  at: number;
}

/** `voices` in `fixtures/audio.json`. */
export interface ChatterRules {
  /** Lines sounding at once. */
  lines: number;
  /** Seconds a request may wait to start before it is dropped. */
  wait_s: number;
  /** Seconds within which requests sharing a group key collapse to the first. */
  group_s: number;
  /** A line's level on the voices bus, before the calibration every take gets. */
  gain: number;
}

/** A line to start on the voices bus: take `variant` of `sound`
 *  (`set/voice/trigger/intensity`), at `gain` before the take's calibration,
 *  from player time `at`; an answer dry and centred, a bark in a battle
 *  placed where it was shouted (position, low-pass, reverb send). */
export type LineStart = { op: "start"; id: number } & LineSpec;

/** A line started: its start command's id, the voice and take, when and for how long. */
export interface VoiceLine {
  id: number;
  request: VoiceRequest;
  voice: string;
  take: number;
  at: number;
  duration: number;
}

export type DropReason = "group" | "late" | "missing";
export interface DroppedLine {
  request: VoiceRequest;
  reason: DropReason;
  /** When it was dropped, on the player's clock. */
  at: number;
}

/** What one arbitration did; `heard` is the barks a battle's arbitration took
 *  from its sound frame, whether they play or not (none otherwise). */
export interface ChatterUpdate {
  commands: LineStart[];
  started: VoiceLine[];
  dropped: DroppedLine[];
  heard: VoiceRequest[];
}

export class Chatter {
  private chatter: RustChatter | null = null;

  constructor(
    private readonly rules: ChatterRules,
    private readonly seed: number,
  ) {}

  /** The Rust arbiter, made on first use (the module is loaded by then). */
  private get rust(): RustChatter {
    return (this.chatter ??= new RustChatter(JSON.stringify(this.rules), this.seed));
  }

  /** Set `set`'s pack arrived with these voices. */
  voices(set: string, voices: readonly Pick<VoiceSpec, "id" | "speaks">[]) {
    this.rust.voices(set, JSON.stringify(voices.map(({ id, speaks }) => ({ id, speaks }))));
  }

  /** A take is loaded, `duration` seconds long. */
  loaded(
    set: string,
    voice: string,
    trigger: Trigger,
    intensity: Intensity,
    take: number,
    duration: number,
  ) {
    this.rust.loaded(set, voice, trigger, intensity, take, duration);
  }

  /** Queues a request at player time `now`; the next arbitration decides whether and when it plays. */
  request(request: VoiceRequest, now: number) {
    this.rust.request(JSON.stringify(request), now);
  }

  /** Arbitrates at player time `now`, nearest `listener` first within a tier. */
  update(now: number, listener: readonly number[]): ChatterUpdate {
    return JSON.parse(this.rust.update(now, Float64Array.from(listener))) as ChatterUpdate;
  }

  /** A battle's arbitration at player time `now`: takes the barks battle
   *  sound `frame` heard, heard from where it last heard. */
  hear(frame: RustSoundFrame, now: number): ChatterUpdate {
    return JSON.parse(this.rust.hear(frame, now)) as ChatterUpdate;
  }

  /** A line is sounding at player time `now`; lines play out through a reset. */
  speaking(now: number): boolean {
    return this.chatter?.speaking(now) ?? false;
  }

  /** Forgets everything waiting and sounding, as for a new battle. */
  reset() {
    this.chatter?.reset();
  }
}
