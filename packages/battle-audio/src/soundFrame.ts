// The battle's sound: the one owner of what is heard, and when.
// It reads exactly the feed the visuals read, and nothing else:
//
// - the effects' `EffectPublication`: gunfire at each launch (`launches.ts`,
//   the muzzle flashes' own derivation), near or far by distance, a recorded
//   burst sounding the gun's next rounds on its cadence; a
//   ricochet whine at each glance; an impact by hit kind; a blast; a hull
//   cooking off, booming with each fireball and clanging where its turret
//   lands, on the effects' own timing (`cookOff.ts`); a motor
//   on a flying guided round; fire crackle and roar on every smoke source
//   the side knows (a wreck), burning then smouldering;
// - the pose driver's motion: each drawn vehicle's engine (idle to load),
//   running gear (tracks or wheels by speed), turret traverse and reverse
//   whine; each drawn soldier's footsteps by the distance he walks;
// - the observation's hearing cues (`audible`): an unseen enemy is heard
//   only as a vague sound from the cue's direction sector, louder in the
//   near band and muffled in the far one, never at a position.
//
// So sound keeps fog of war exactly as the picture does: every positional
// sound has a cause in the side's observation (its own units, identified
// enemies, published segments and blasts, known wrecks).
//
// Timing: every event carries its presentation time (the tick it happened
// in, as the effects place it) and starts when the presentation clock
// reaches it, so what is heard lines up with what is drawn. A clock that
// stands still (pause, or no ticks arriving) is a hold: transients fall
// silent and none start, loops hold. A clock that jumps drops what is too
// late to play. A publication earlier than the last is a new battle.
//
// Voices: a budget (`presentation.audio.budget`) with loops capped apart;
// each voice's priority is its level at the listener, so a louder new sound
// takes the quietest voice's place and a quieter one is dropped (counted).
// The Web Audio graph is behind `VoiceSink` (`webAudioSink.ts`), so all of
// this runs, and is tested, without an audio device.
import {
  sourceLifetime,
  type EffectPublication,
} from "@packages/battle-renderer/src/effects/effectFrame";
import { clamp, lerp, vec3, type Vec3 } from "math";
import { hashString } from "@packages/renderer-core/src/math";
import { impactAfter, type CookOffFeel } from "@packages/battle-renderer/src/effects/cookOff";
import { LaunchTracker } from "@packages/battle-renderer/src/effects/launches";
import { pick } from "@packages/renderer-core/src/kindTable";
import { validateAudio, type AudioPresentation, type Bus } from "./audioPresentation";
import { firingCadence, resolveEffect, resolveShot, type SoundCatalog } from "./catalog";
import { GLANCE } from "./contacts";
import { gameSounds } from "./shippedSounds";

type P3 = readonly [number, number, number] | readonly number[];

/** A hearing cue, as the observation publishes it. */
export interface SoundCue {
  category: string;
  /** 0 = east, counter-clockwise in 45° steps, from the listening unit. */
  sector: number;
  band: string;
  moving: boolean;
}

/** One publication, as sound reads it. */
export interface SoundPublication {
  effects: EffectPublication;
  audible: readonly SoundCue[];
}

/** A drawn vehicle, as posed this frame. */
export interface SoundVehicle {
  /** Unique across both sides (the effects' shooter key). */
  key: number;
  /** Its presentation class (`vehicleClass`): the `vehicles` row it sounds as. */
  vehicleClass: string;
  position: P3;
  /** Metres the left and right running gear have rolled (the pose's articulation). */
  travelL: number;
  travelR: number;
  /** Turret heading relative to the hull, radians. */
  turret: number;
  /** Backing up: a reverse move, or a turn's reversing leg. */
  reverse?: boolean;
}

/** A drawn soldier, as posed this frame. */
export interface SoundSoldier {
  id: number;
  position: P3;
}

/** What the side draws moving this frame (the pose driver's output). */
export interface SoundMotion {
  vehicles: readonly SoundVehicle[];
  soldiers: readonly SoundSoldier[];
}

/** The listener: where it stands and which way the camera looks (horizontal). */
export interface Listener {
  position: P3;
  forward: P3;
}

/** A voice to start. `position` null is not positional: `pan` places it
 *  (a cue's direction), or it plays centred (the ambience bed). */
export interface VoiceSpec {
  sound: string;
  /** Stable event-derived variation, interpreted by the prepared bank. */
  variant: number;
  bus: Bus;
  /** Sink time to start at, seconds. */
  at: number;
  /** Explicit source offset in seconds; omitted loops stagger their phases. */
  offset?: number;
  gain: number;
  rate: number;
  /** Low-pass cutoff, Hz. */
  lowpass: number;
  /** Reverb send, a share of the dry level. */
  wet: number;
  /** Seconds the onset ramps up over (0: as synthesised). */
  attack: number;
  position: P3 | null;
  pan: number | null;
  loop: boolean;
}

/** A running loop's live parameters. */
export interface VoiceParams {
  gain: number;
  rate: number;
  lowpass: number;
  wet: number;
  position: P3 | null;
}

/** The audio graph, as the frame drives it. */
export interface VoiceSink {
  /** The sink's clock, seconds. */
  now(): number;
  /** A sound's length at rate 1, seconds. */
  duration(sound: string, variant?: number): number;
  start(id: number, voice: VoiceSpec): void;
  set(id: number, params: VoiceParams, at: number): void;
  /** Fade out over `fade` seconds from `at`, then free. */
  stop(id: number, at: number, fade: number): void;
  listen(listener: Listener, at: number): void;
}

export interface SoundStats {
  /** Voices sounding now: transients and loops. */
  transients: number;
  loops: number;
  /** Transients waiting for the clock. */
  pending: number;
  /** Started, dropped for the budget, and stolen by louder ones, since the last reset. */
  started: number;
  dropped: number;
  stolen: number;
  /** Positional transients started, and cue sounds started, since the last reset. */
  positional: number;
  cues: number;
  /** Holding: the clock stands still. */
  held: boolean;
  /** The last publication taken. */
  tick: number;
}

export interface SoundFrameOptions {
  tickHz: number;
  presentation: AudioPresentation;
  catalog?: SoundCatalog;
  /** How long each smoke kind burns and smoulders (`presentation.effects.smoke`). */
  smokeTimes: Record<string, { burn_s: number; smoulder_s: number }>;
  /** When a cook-off's fireballs go and its turret lands (`presentation.effects.cook_off`). */
  cookOff: CookOffFeel;
}

/** A transient waiting for the clock. */
interface Pending {
  t: number;
  sound: string;
  variant: number;
  /** The far sound, and the distance it takes over at (gunfire, blasts). */
  far: string | null;
  far_m: number;
  bus: Bus;
  gain: number;
  rate: number;
  position: P3 | null;
  /** A cue's direction sector, placed by the listener at start; its band's
   *  low-pass and far colour (0 near, 1 far). */
  sector: number;
  lowpass: number;
  far_share: number;
}

interface Voice {
  id: number;
  priority: number;
  /** A transient's end on the sink clock; Infinity for a loop. */
  end: number;
}

/** A loop wanted this frame: its identity, sound, placement and level. */
interface LoopWant {
  key: string;
  sound: string;
  bus: Bus;
  gain: number;
  rate: number;
  position: P3 | null;
  priority: number;
}

interface Mover {
  travel: number;
  turret: number;
  speed: number;
  traverse: number;
}

interface Fire {
  x: number;
  y: number;
  z: number;
  kind: string;
  start: number;
}

/** A gun's sounding burst recording: when it started, and the rounds it covers. */
interface BurstVoice {
  start: number;
  /** Its shots `interval` apart; `heard` of them already matched to rounds. */
  shots: number;
  interval: number;
  heard: number;
}

interface Motor {
  kind: string;
  from: P3;
  to: P3;
  t0: number;
}

const endKey = (p: P3) => `${p[0]},${p[1]},${p[2]}`;

/** Stereo position of a direction sector for a listener looking along `forward`. */
export function sectorPan(sector: number, forward: P3): number {
  const a = (sector * Math.PI) / 4;
  const len = Math.hypot(forward[0], forward[1]) || 1;
  // Screen-right: forward × up.
  const rx = forward[1] / len;
  const ry = -forward[0] / len;
  return Math.max(-1, Math.min(1, Math.cos(a) * rx + Math.sin(a) * ry));
}

/** Pending transients held at most; past it the oldest go. */
const PENDING_CAP = 1024;
/** How far ahead of the clock a transient is handed to the sink, seconds. */
const LOOKAHEAD_S = 0.05;
/** Seconds a voice fades out over when stopped: a hold, a steal, a lost loop. */
const FADE_S = { hold: 0.03, steal: 0.02, loop: 0.15 };

export class SoundFrame {
  private readonly p: AudioPresentation;
  private readonly catalog: SoundCatalog;
  private readonly dt: number;
  private readonly launches: LaunchTracker;
  private readonly smokeTimes: SoundFrameOptions["smokeTimes"];
  private readonly cookOff: CookOffFeel;
  private pending: Pending[] = [];
  private transients: Voice[] = [];
  private readonly loops = new Map<string, Voice>();
  private readonly movers = new Map<number, Mover>();
  private readonly strides = new Map<number, { x: number; y: number; carry: number }>();
  private readonly fires = new Map<string, Fire>();
  private motors = new Map<string, Motor>();
  private readonly bursts = new Map<string, BurstVoice>();
  private readonly cueHeard = new Map<string, number>();
  private nextId = 1;
  private lastTick = -1;
  private clock: number | null = null;
  private advancedAt = 0;
  private held = false;
  private listener: Listener = { position: [0, 0, 0], forward: [0, 1, 0] };
  private counts = { started: 0, dropped: 0, stolen: 0, positional: 0, cues: 0 };

  constructor(
    options: SoundFrameOptions,
    private readonly sink: VoiceSink,
  ) {
    this.p = validateAudio(options.presentation);
    this.catalog = options.catalog ?? gameSounds;
    this.dt = 1 / options.tickHz;
    this.launches = new LaunchTracker();
    this.smokeTimes = options.smokeTimes;
    this.cookOff = options.cookOff;
  }

  /** Silence and forget everything (a new battle). */
  reset() {
    const now = this.sink.now();
    for (const v of this.transients) this.sink.stop(v.id, now, FADE_S.hold);
    for (const v of this.loops.values()) this.sink.stop(v.id, now, FADE_S.hold);
    this.transients = [];
    this.loops.clear();
    this.pending = [];
    this.launches.reset();
    this.movers.clear();
    this.strides.clear();
    this.fires.clear();
    this.motors.clear();
    this.bursts.clear();
    this.cueHeard.clear();
    this.lastTick = -1;
    this.clock = null;
    this.held = false;
    this.counts = { started: 0, dropped: 0, stolen: 0, positional: 0, cues: 0 };
  }

  /** Take one publication: what it shows happened becomes transients at
   *  their presentation times, and its smoke sources and flying rounds the
   *  fire and motor loops. The same tick again is ignored. */
  note(pub: SoundPublication) {
    const fx = pub.effects;
    if (fx.tick === this.lastTick) return;
    if (fx.tick < this.lastTick) this.reset();
    const gap = this.lastTick >= 0 && fx.tick !== this.lastTick + 1;
    this.lastTick = fx.tick;
    const t0 = (fx.tick - 1) * this.dt;
    const t1 = fx.tick * this.dt;
    const p = this.p;

    for (const [gun, b] of this.bursts)
      if (t0 > b.start + b.shots * b.interval) this.bursts.delete(gun);
    for (const l of this.launches.note(fx, gap)) {
      const base = pick(p.shots, l.kind);
      const s = resolveShot(this.catalog, base, l.kind);
      // A burst recording sounds the gun's next rounds too, while they land
      // on its shots; a round off its cadence, or past its last, starts another.
      const gun = `${l.shooter}:${l.mount}:${l.soldier}`;
      const b = this.bursts.get(gun);
      if (
        b &&
        b.heard < b.shots &&
        Math.abs(t0 - b.start - b.heard * b.interval) <= b.interval / 2
      ) {
        b.heard++;
        continue;
      }
      const burst = firingCadence(this.catalog, s.near);
      if (burst.shots > 1)
        this.bursts.set(gun, {
          start: t0,
          shots: burst.shots,
          interval: burst.interval_s,
          heard: 1,
        });
      else this.bursts.delete(gun);
      this.queue(
        t0,
        s.near,
        s.far,
        s.far_m,
        "effects",
        s.gain,
        [l.x, l.y, l.z],
        l.shooter + l.kind,
        `${fx.tick}:${l.shooter}:${l.mount}:${l.soldier}:${l.kind}`,
      );
    }
    const motors = new Map<string, Motor>();
    for (const s of fx.segments) {
      if (s.path.length < 2) continue;
      // Ricochets at their place along the tick's stretch.
      if (s.ricochets.length) {
        let total = 0;
        const cum = [0];
        for (let i = 1; i < s.path.length; i++) {
          const a = s.path[i - 1];
          const b = s.path[i];
          total += Math.hypot(b[0] - a[0], b[1] - a[1], b[2] - a[2]);
          cum.push(total);
        }
        for (const r of s.ricochets) {
          const at = t0 + (this.dt * cum[r.point]) / Math.max(total, 1e-6);
          const pt = s.path[r.point];
          this.queue(
            at,
            this.contact(GLANCE, s.kind, p.ricochet.sound),
            null,
            0,
            "effects",
            p.ricochet.gain * pick(p.impact_scale, s.kind),
            pt,
            endKey(pt),
          );
        }
      }
      const end = s.path[s.path.length - 1];
      if (s.hit !== "none") {
        const hit = pick(p.impacts, s.hit);
        const scale = pick(p.impact_scale, s.kind);
        this.queue(
          t1,
          this.contact(s.hit, s.kind, hit.sound),
          null,
          0,
          "effects",
          hit.gain * scale,
          end,
          endKey(end),
        );
      } else if (p.motors[s.kind]) {
        // A flying round with a motor: one loop along its chain of stretches.
        // Chains are keyed by where their latest stretch ends; a stretch
        // starting where one ended continues it, and its loop follows.
        motors.set(endKey(end), { kind: s.kind, from: s.path[0], to: end, t0 });
        const loop = this.loops.get(`motor:${endKey(s.path[0])}`);
        if (loop) {
          this.loops.delete(`motor:${endKey(s.path[0])}`);
          this.loops.set(`motor:${endKey(end)}`, loop);
        }
      }
    }
    this.motors = motors;
    for (const b of fx.blasts) {
      const s = pick(p.blasts, b.kind);
      this.queue(
        t1,
        this.effect(s.near),
        this.effect(s.far),
        s.far_m,
        "effects",
        s.gain,
        b.point,
        endKey(b.point),
      );
    }
    // A cook-off: a boom with each fireball, the biggest loudest, and the
    // clang of its turret striking the deck, as the effects time them.
    const cook = this.cookOff;
    const biggest = Math.max(...cook.fireballs.map((f) => f.radius_m));
    for (const c of fx.cookOffs ?? []) {
      const ring: P3 = [c.center[0], c.center[1], c.center[2] + c.height];
      cook.fireballs.forEach((f, i) =>
        this.queue(
          t0 + cook.delay_s + f.after_s,
          this.effect(p.cook_off.blast.near),
          this.effect(p.cook_off.blast.far),
          p.cook_off.blast.far_m,
          "effects",
          (p.cook_off.blast.gain * f.radius_m) / biggest,
          ring,
          `cook_off:${i}:${endKey(ring)}`,
        ),
      );
      if (c.landing)
        this.queue(
          t0 + impactAfter(cook),
          this.contact(
            p.cook_off.landing.hit,
            p.cook_off.landing.as,
            pick(p.impacts, p.cook_off.landing.hit).sound,
          ),
          null,
          0,
          "effects",
          p.cook_off.landing.gain,
          c.landing,
          `cook_off:landing:${endKey(c.landing)}`,
        );
    }
    // Fires: every smoke source the side knows, from when it was first known.
    const known = new Set<string>();
    for (const k of fx.smokes) {
      known.add(k.key);
      if (!this.fires.has(k.key))
        this.fires.set(k.key, {
          x: k.center[0],
          y: k.center[1],
          z: k.center[2] + k.half[2],
          kind: k.kind,
          start: t0,
        });
    }
    for (const key of this.fires.keys()) if (!known.has(key)) this.fires.delete(key);

    // Unseen enemies: one vague sound per distinct cue, not repeated within
    // `repeat_s`, placed by direction at start and never at a position.
    const cues = p.cues;
    for (const c of pub.audible) {
      const name = `${c.category}${c.moving ? "_moving" : ""}`;
      const key = `${name}/${c.sector}/${c.band}`;
      const last = this.cueHeard.get(key);
      if (last !== undefined && t0 - last < cues.repeat_s && t0 >= last) continue;
      this.cueHeard.set(key, t0);
      const sound = cues.sounds[name] ?? cues.sounds[c.category] ?? cues.sounds.default;
      const near = c.band === "near";
      this.pending.push({
        t: t0,
        sound: this.effect(sound),
        variant: 0,
        far: null,
        far_m: 0,
        bus: "effects",
        gain: near ? cues.near_gain : cues.far_gain,
        rate: 1,
        position: null,
        sector: c.sector,
        lowpass: near ? p.air.near_hz : cues.far_lowpass_hz,
        far_share: near ? 0 : 1,
      });
    }
    if (this.pending.length > PENDING_CAP)
      this.pending.splice(0, this.pending.length - PENDING_CAP);
  }

  /** A round's sound on a contact (a hit material, or a glance): the round's
   *  choice, the contact's default, or the baseline slot's replacement. */
  private contact(contact: string, kind: string, baseline: string): string {
    const row = this.catalog.impacts[contact];
    return row?.[kind] ?? row?.default ?? this.effect(baseline);
  }

  private effect(sound: string): string {
    return resolveEffect(this.catalog, sound);
  }

  private queue(
    t: number,
    sound: string,
    far: string | null,
    far_m: number,
    bus: Bus,
    gain: number,
    position: P3,
    variety: string,
    variantKey = variety,
  ) {
    this.pending.push({
      t,
      sound,
      variant: hashString(`${t}:${variantKey}:${endKey(position)}`) >>> 0,
      far,
      far_m,
      bus,
      gain,
      rate: 0.94 + 0.12 * hash01(variety),
      position,
      sector: -1,
      lowpass: 0,
      far_share: 0,
    });
  }

  /** Level at the listener for a sound of `gain` at `position`. */
  private level(gain: number, position: P3 | null): number {
    return position ? gain * distanceGain(this.p, this.distance(position)) : gain;
  }

  private distance(position: P3): number {
    const l = this.listener.position;
    return Math.hypot(position[0] - l[0], position[1] - l[1], position[2] - l[2]);
  }

  /** How far `position` sounds: 0 at the listener, 1 at `air.far_m` and beyond. */
  private farShare(position: P3): number {
    return clamp(this.distance(position) / this.p.air.far_m, 0, 1);
  }

  /** Advance to presentation time `clock` at wall time `wall` (seconds):
   *  start what is due, footsteps and loops from `motion`, heard at `listener`. */
  update(clock: number, wall: number, motion: SoundMotion, listener: Listener) {
    this.listener = listener;
    const now = this.sink.now();
    this.sink.listen(listener, now);
    // The clock running backwards is a new battle.
    if (this.clock !== null && clock < this.clock - 1e-9) this.reset();
    if (this.clock === null || clock !== this.clock) this.advancedAt = wall;
    const step = this.clock === null ? 0 : Math.max(0, clock - this.clock);
    this.clock = clock;
    const held = wall - this.advancedAt > this.p.hold_s;
    if (held && !this.held) {
      for (const v of this.transients) this.sink.stop(v.id, now, FADE_S.hold);
      this.transients = [];
    }
    this.held = held;
    this.transients = this.transients.filter((v) => v.end > now);
    if (held) return; // loops hold as they are
    this.footsteps(motion, clock, step);
    this.flush(clock, now);
    this.updateLoops(clock, now, motion, step);
  }

  /** A footstep each `stride_m` a soldier walks; a jump past `max_step_m`
   *  (seen again elsewhere) starts his count afresh. */
  private footsteps(motion: SoundMotion, clock: number, step: number) {
    const f = this.p.footsteps;
    const seen = new Set<number>();
    for (const s of motion.soldiers) {
      seen.add(s.id);
      const [x, y] = s.position;
      const st = this.strides.get(s.id);
      if (!st) {
        this.strides.set(s.id, { x, y, carry: hash01(`step${s.id}`) * f.stride_m });
        continue;
      }
      const d = Math.hypot(x - st.x, y - st.y);
      st.x = x;
      st.y = y;
      if (step <= 0 || d > f.max_step_m) continue;
      st.carry += d;
      if (st.carry < f.stride_m) continue;
      st.carry %= f.stride_m;
      if (this.level(f.gain, s.position) < this.p.distance.cull) continue;
      this.queue(
        clock,
        this.effect(f.sound),
        null,
        0,
        "units",
        f.gain,
        s.position,
        `${s.id}:${clock}`,
      );
    }
    for (const id of this.strides.keys()) if (!seen.has(id)) this.strides.delete(id);
  }

  /** Start every pending transient due by `clock`, loudest first, within the budget. */
  private flush(clock: number, now: number) {
    const due: Pending[] = [];
    const later: Pending[] = [];
    for (const e of this.pending) {
      if (e.t > clock + LOOKAHEAD_S) later.push(e);
      else if (e.t >= clock - this.p.late_s) due.push(e);
    }
    this.pending = later;
    if (!due.length) return;
    const cap = this.p.budget.voices - this.p.budget.loops;
    const cull = this.p.distance.cull;
    const ranked = due
      .map((e) => ({ e, priority: this.level(e.gain, e.position) }))
      .filter((r) => r.priority >= cull)
      .sort((a, b) => b.priority - a.priority);
    for (const { e, priority } of ranked) {
      if (this.transients.length >= cap) {
        let low = 0;
        for (let i = 1; i < this.transients.length; i++)
          if (this.transients[i].priority < this.transients[low].priority) low = i;
        if (this.transients[low].priority >= priority) {
          this.counts.dropped++;
          continue;
        }
        this.sink.stop(this.transients[low].id, now, FADE_S.steal);
        this.transients.splice(low, 1);
        this.counts.stolen++;
      }
      const far = e.far !== null && e.position && this.distance(e.position) > e.far_m;
      const sound = far ? e.far! : e.sound;
      const at = now + Math.max(0, e.t - clock);
      const id = this.nextId++;
      const u = e.position ? this.farShare(e.position) : e.far_share;
      this.sink.start(id, {
        sound,
        variant: e.variant,
        bus: e.bus,
        at,
        gain: e.position ? priority : e.gain,
        rate: e.rate,
        lowpass: e.position ? airLowpass(this.p, u) : e.lowpass,
        wet: this.p.air.wet * u,
        attack: this.p.air.attack_s * u,
        position: e.position,
        pan: e.position ? null : sectorPan(e.sector, this.listener.forward),
        loop: false,
      });
      this.transients.push({
        id,
        priority,
        end: at + this.sink.duration(sound, e.variant) / e.rate,
      });
      this.counts.started++;
      if (e.position) this.counts.positional++;
      else this.counts.cues++;
    }
  }

  /** Every loop wanted now: the bed, engines and running gear, turrets,
   *  reverse, fires and motors; the loudest within the loop budget sound. */
  private updateLoops(clock: number, now: number, motion: SoundMotion, step: number) {
    const p = this.p;
    const wants: LoopWant[] = [];
    const seen = new Set<number>();
    for (const v of motion.vehicles) {
      seen.add(v.key);
      const row = pick(p.vehicles, v.vehicleClass);
      const travel = (Math.abs(v.travelL) + Math.abs(v.travelR)) / 2;
      let m = this.movers.get(v.key);
      if (!m) {
        m = { travel, turret: v.turret, speed: 0, traverse: 0 };
        this.movers.set(v.key, m);
      }
      if (step > 0) {
        // Rolled distance and traverse over the frame, eased (about 0.25 s).
        const speed = Math.abs(travel - m.travel) / step;
        const turn = Math.abs(v.turret - m.turret) / step;
        const k = Math.min(1, step / 0.25);
        m.speed += (Math.min(speed, 60) - m.speed) * k;
        m.traverse += (Math.min(turn, 10) - m.traverse) * k;
      }
      m.travel = travel;
      m.turret = v.turret;
      const load = clamp(m.speed / row.full_speed_mps, 0, 1);
      const key = `v${v.key}`;
      wants.push({
        key: `${key}:engine`,
        sound: row.engine,
        bus: "units",
        gain: lerp(row.idle_gain, row.load_gain, load),
        rate: lerp(row.idle_rate, row.load_rate, load),
        position: v.position,
        priority: 0,
      });
      if (row.running && load > 0.02)
        wants.push({
          key: `${key}:running`,
          sound: row.running,
          bus: "units",
          gain: row.running_gain * Math.sqrt(load),
          rate: 0.5 + load,
          position: v.position,
          priority: 0,
        });
      const traverse = clamp(m.traverse / row.full_traverse_rps, 0, 1);
      if (row.turret && traverse > 0.05)
        wants.push({
          key: `${key}:turret`,
          sound: row.turret,
          bus: "units",
          gain: row.turret_gain * traverse,
          rate: 0.9 + 0.2 * traverse,
          position: v.position,
          priority: 0,
        });
      if (row.reverse && v.reverse && load > 0.02)
        wants.push({
          key: `${key}:reverse`,
          sound: row.reverse,
          bus: "units",
          gain: row.reverse_gain * load,
          rate: 0.8 + 0.4 * load,
          position: v.position,
          priority: 0,
        });
    }
    for (const key of this.movers.keys()) if (!seen.has(key)) this.movers.delete(key);
    for (const [key, f] of this.fires) {
      const times = pick(this.smokeTimes, f.kind);
      if (!times) continue;
      const row = pick(p.fires, f.kind);
      const age = clock - f.start;
      if (age > sourceLifetime(times)) continue;
      // Full while it burns, easing to the smoulder over its last tenth.
      const burn = clamp((times.burn_s - age) / (times.burn_s * 0.1), 0, 1);
      wants.push({
        key: `fire:${key}`,
        sound: row.sound,
        bus: "ambience",
        gain: lerp(row.smoulder_gain, row.gain, burn),
        rate: 1,
        position: [f.x, f.y, f.z],
        priority: 0,
      });
    }
    for (const [key, m] of this.motors) {
      const u = clamp((clock - m.t0) / this.dt, 0, 1);
      const at = vec3.lerp(vec3.create(), m.from as Vec3, m.to as Vec3, u);
      const row = p.motors[m.kind];
      wants.push({
        key: `motor:${key}`,
        sound: row.sound,
        bus: "effects",
        gain: row.gain,
        rate: 1,
        position: at,
        priority: 0,
      });
    }
    for (const w of wants) w.priority = this.level(w.gain, w.position);
    const kept = wants
      .filter((w) => w.priority >= p.distance.cull)
      .sort((a, b) => b.priority - a.priority)
      .slice(0, p.budget.loops);
    // The bed is always on and outside the budget.
    kept.push({
      key: "ambience",
      sound: p.ambience.sound,
      bus: "ambience",
      gain: p.ambience.gain,
      rate: 1,
      position: null,
      priority: Infinity,
    });
    const keep = new Set(kept.map((w) => w.key));
    for (const [key, v] of this.loops)
      if (!keep.has(key)) {
        this.sink.stop(v.id, now, FADE_S.loop);
        this.loops.delete(key);
      }
    for (const w of kept) {
      const u = w.position ? this.farShare(w.position) : 0;
      const params: VoiceParams = {
        gain: w.position ? w.priority : w.gain,
        rate: w.rate,
        lowpass: airLowpass(p, u),
        wet: p.air.wet * u,
        position: w.position,
      };
      const v = this.loops.get(w.key);
      if (v) {
        v.priority = w.priority;
        this.sink.set(v.id, params, now);
        continue;
      }
      const id = this.nextId++;
      this.sink.start(id, {
        sound: this.effect(w.sound),
        variant: 0,
        bus: w.bus,
        at: now,
        ...params,
        attack: 0,
        pan: null,
        loop: true,
      });
      this.loops.set(w.key, { id, priority: w.priority, end: Infinity });
    }
  }

  stats(): SoundStats {
    return {
      transients: this.transients.length,
      loops: this.loops.size,
      pending: this.pending.length,
      ...this.counts,
      held: this.held,
      tick: this.lastTick,
    };
  }
}

/** The share of a sound's gain heard `d` metres off: full within `ref_m`,
 *  then inverse distance easing down to `floor` (never below it), and
 *  nothing past `max_m`. */
export function distanceGain(p: AudioPresentation, d: number): number {
  const { ref_m, rolloff, max_m, floor } = p.distance;
  if (d > max_m) return 0;
  const inverse = ref_m / (ref_m + rolloff * Math.max(0, d - ref_m));
  return floor + (1 - floor) * inverse;
}

/** The air's low-pass cutoff at far share `u`, log-spaced: each octave of
 *  cutoff lost over an equal stretch. */
function airLowpass(p: AudioPresentation, u: number): number {
  const { near_hz, far_hz } = p.air;
  return near_hz * (far_hz / near_hz) ** u;
}

const hash01 = (key: string) => hashString(key) / 0xffffffff;
