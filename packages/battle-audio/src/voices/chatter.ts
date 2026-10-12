// Which voice lines play, and when: the one place line limits are enforced.
// The battle and the sound workbench's chatter preview feed it the same
// requests and it answers both the same way:
//
// - responses beat own barks, which beat others' barks; within a tier the
//   request nearest the camera speaks first;
// - at most `lines` lines sound at once;
// - a request that can't start within `wait_s` of its moment is dropped;
// - a speaker never talks over itself;
// - requests sharing a group key within `group_s` collapse to the first, so
//   one unit answers for a group (the caller orders them: the clicked unit,
//   else the highest value);
// - a line whose take hasn't loaded is dropped at once.
//
// It is pure: time is the sink's clock, audio is the sink's business, and the
// take is chosen by the client's own seeded randomness, never the simulation's.
import { vec3, type Vec3 } from "math";
import { mulberry32, random } from "math/random";
import { hashString } from "@packages/renderer-core/src/math";
import type { Intensity, Trigger, VoiceSpec } from "./voiceCatalog";

/** Who hears a line first: the owner's answers, then own units' shouts, then others'. */
export type VoiceTier = "response" | "own_bark" | "other_bark";
const RANK: Record<VoiceTier, number> = { response: 0, own_bark: 1, other_bark: 2 };

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
  /** The moment it was asked for, on the sink's clock. */
  at: number;
}

/** A line to play: the request, the voice and take chosen, and its start on the sink's clock. */
export interface VoiceLine {
  request: VoiceRequest;
  voice: string;
  take: number;
  buffer: AudioBuffer;
  at: number;
}

export type DropReason = "group" | "late" | "missing";
export interface DroppedLine {
  request: VoiceRequest;
  reason: DropReason;
  /** When it was dropped, on the sink's clock. */
  at: number;
}

/** `presentation.audio.voices` in `fixtures/game.json`. */
export interface ChatterRules {
  /** Lines sounding at once. */
  lines: number;
  /** Seconds a request may wait to start before it is dropped. */
  wait_s: number;
  /** Seconds within which requests sharing a group key collapse to the first. */
  group_s: number;
}

/** Where lines are heard, and the clock they run on. */
export interface LineSink {
  now(): number;
  play(line: VoiceLine): void;
}

/** The loaded takes: a set's voices, and a take's audio or null while it isn't loaded. */
export interface LineSource {
  voices(set: string): readonly Pick<VoiceSpec, "id" | "speaks">[];
  get(
    set: string,
    voice: string,
    trigger: Trigger,
    intensity: Intensity,
    take: number,
  ): AudioBuffer | null;
}

/** What one arbitration did. */
export interface ChatterUpdate {
  started: VoiceLine[];
  dropped: DroppedLine[];
}

interface Pending {
  request: VoiceRequest;
  /** Arrival order: the last tie-break. */
  order: number;
  /** This speaker's requests for this trigger and intensity before it: keys the take. */
  count: number;
}

const TAKES = [0, 1] as const;
const _chatter_listener: Vec3 = [0, 0, 0];
const _chatter_position: Vec3 = [0, 0, 0];

export class Chatter {
  private pending: Pending[] = [];
  private sounding: { speaker: string; end: number }[] = [];
  /** Each group key's first request moment. */
  private groups = new Map<string, number>();
  /** Requests so far per speaker, trigger and intensity. */
  private counts = new Map<string, number>();
  private dropped: DroppedLine[] = [];
  private arrivals = 0;

  constructor(
    private readonly rules: ChatterRules,
    private readonly source: LineSource,
    private readonly sink: LineSink,
    private readonly seed: number,
  ) {}

  /** Queues a request; the next `update` decides whether and when it plays. */
  request(request: VoiceRequest) {
    if (request.groupKey !== undefined) {
      const first = this.groups.get(request.groupKey);
      if (first !== undefined && request.at - first <= this.rules.group_s) {
        this.dropped.push({ request, reason: "group", at: this.sink.now() });
        return;
      }
      this.groups.set(request.groupKey, request.at);
    }
    const key = `${request.speaker}/${request.trigger}/${request.intensity}`;
    const count = this.counts.get(key) ?? 0;
    this.counts.set(key, count + 1);
    this.pending.push({ request, order: this.arrivals++, count });
  }

  /** Starts what may start now, nearest `listener` first within a tier, and drops what is too late. */
  update(listener: readonly number[]): ChatterUpdate {
    const now = this.sink.now();
    const dropped = this.dropped;
    this.dropped = [];
    this.sounding = this.sounding.filter((line) => line.end > now);
    for (const [key, first] of this.groups)
      if (now - first > this.rules.group_s) this.groups.delete(key);
    this.pending = this.pending.filter((item) => {
      if (now - item.request.at <= this.rules.wait_s) return true;
      dropped.push({ request: item.request, reason: "late", at: now });
      return false;
    });

    vec3.set(_chatter_listener, listener[0], listener[1], listener[2]);
    // An owner's answer has no position: it is as near as can be.
    const rangeSq = ({ position: p }: VoiceRequest) =>
      p
        ? vec3.squaredDistance(_chatter_listener, vec3.set(_chatter_position, p[0], p[1], p[2]))
        : 0;
    const ready = this.pending
      .filter((item) => item.request.at <= now)
      .map((item) => ({ item, rangeSq: rangeSq(item.request) }))
      .sort(
        (a, b) =>
          RANK[a.item.request.tier] - RANK[b.item.request.tier] ||
          a.rangeSq - b.rangeSq ||
          a.item.request.at - b.item.request.at ||
          a.item.order - b.item.order,
      );

    const started: VoiceLine[] = [];
    const done = new Set<Pending>();
    for (const { item } of ready) {
      if (this.sounding.length >= this.rules.lines) break;
      const { request } = item;
      if (this.sounding.some((line) => line.speaker === request.speaker)) continue;
      done.add(item);
      const line = this.line(item, now);
      if (!line) {
        dropped.push({ request, reason: "missing", at: now });
        continue;
      }
      this.sounding.push({ speaker: request.speaker, end: now + line.buffer.duration });
      started.push(line);
      this.sink.play(line);
    }
    this.pending = this.pending.filter((item) => !done.has(item));
    return { started, dropped };
  }

  /** Forgets everything waiting and sounding, as for a new battle. */
  reset() {
    this.pending = [];
    this.sounding = [];
    this.groups.clear();
    this.counts.clear();
    this.dropped = [];
  }

  /** The voice and take `item` speaks with, or null when none is loaded. */
  private line({ request, count }: Pending, now: number): VoiceLine | null {
    const voice = this.voice(request);
    if (voice === null) return null;
    const { set, trigger, intensity, speaker } = request;
    const loaded = TAKES.filter((take) => this.source.get(set, voice, trigger, intensity, take));
    if (!loaded.length) return null;
    const rng = mulberry32.create(
      hashString(`${this.seed}/${speaker}/${trigger}/${intensity}/${count}`),
    );
    const take = random.choice(() => mulberry32.sample(rng), loaded);
    const buffer = this.source.get(set, voice, trigger, intensity, take)!;
    return { request, voice, take, buffer, at: now };
  }

  /** Answers come from the set's answering voice; a speaker barks in one voice, by its name. */
  private voice({ tier, set, speaker }: VoiceRequest): string | null {
    const speaks = tier === "response" ? "responses" : "barks";
    const voices = this.source
      .voices(set)
      .filter((voice) => voice.speaks === speaks || voice.speaks === "all");
    if (!voices.length) return null;
    return tier === "response" ? voices[0].id : voices[hashString(speaker) % voices.length].id;
  }
}
