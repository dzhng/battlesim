// Combat effects: the one owner (slice 25). What a side's publications say
// happened becomes short-lived effects, and each frame they become the effect
// pass's instances at the presentation clock:
//
// - a tracer for every visible stretch of flight, styled by its round kind,
//   running along the stretch over the tick that flew it;
// - a muzzle flash where a mount's shot counter rose: at the vehicle's
//   muzzle, or on the soldier a new round names as its shooter;
// - an impact puff where a stretch ends in a hit, by hit class, off the
//   surface along its published normal (sparks off a hull);
// - sparks at every ricochet corner, thrown off the glancing face;
// - a fireball for every published blast.
//
// Every effect has a published cause, and nothing else: no rule, no
// simulation state (enemy stretches arrive already clipped to seen ground).
// A publication is taken once: the same tick again is ignored, an earlier one
// starts over (a new battle). Presentation only; `presentation.effects` holds
// every curve and size.
import { vec3, type Vec3 } from "math";
import { mulberry32 } from "math/random";

type P3 = readonly [number, number, number] | readonly number[];

/** A visible stretch of one round's flight this tick (the observation's). */
export interface EffectSegment {
  /** The flown path, at least two points; it bends at each ricochet. */
  path: readonly P3[];
  /** Where along `path` the round glanced off a hull, with the face's outward normal. */
  ricochets: readonly { point: number; normal: P3 }[];
  /** The round kind: a weapon row name. */
  kind: string;
  /** The soldier who fired it, or null for a vehicle's gun. */
  shooter: number | null;
  /** What the round struck at the path's end: none, ground, hull, prop or soldier. */
  hit: string;
  /** Outward surface normal at the impact, or null. */
  normal: P3 | null;
}

/** A round's burst this tick. */
export interface EffectBlast {
  point: P3;
  radius: number;
  kind: string;
}

/** One weapon mount as published: a rise in `shots` is a shot. */
export interface EffectMount {
  bearing: number;
  elevation: number;
  shots: number;
  /** The round kind it fires (its first weapon row). */
  kind: string;
}

/** A unit whose shots the side sees: an own unit or an identified enemy. */
export interface EffectShooter {
  /** Unique across both sides for the life of the battle. */
  key: number;
  /** A hull: its shots leave the vehicle muzzle; otherwise a soldier's rifle. */
  vehicle: boolean;
  position: P3;
  /** Its soldiers' ids (infantry). */
  members: readonly number[];
  mounts: readonly EffectMount[];
}

/** One publication, as the effects read it. */
export interface EffectPublication {
  tick: number;
  segments: readonly EffectSegment[];
  blasts: readonly EffectBlast[];
  shooters: readonly EffectShooter[];
}

type Rgb = [number, number, number];

export interface TracerStyle {
  color: Rgb;
  intensity: number;
  /** The glowing streak's length and width, metres. */
  length_m: number;
  width_m: number;
}

export interface FlashStyle {
  color: Rgb;
  intensity: number;
  /** The glow's radius, the forward tongue's length, metres. */
  size_m: number;
  tongue_m: number;
  duration_s: number;
  /** A muzzle fireball's size (0 for none) and how long it burns. */
  fireball_m: number;
  fireball_s: number;
}

export interface ImpactStyle {
  /** The puff's lit colour and opacity. */
  color: Rgb;
  opacity: number;
  size_m: number;
  duration_s: number;
  /** Sparks thrown off the surface, and a hot flash's intensity (0 for none). */
  sparks: number;
  flash: number;
}

export interface SparkStyle {
  color: Rgb;
  intensity: number;
  speed_mps: number;
  duration_s: number;
  length_m: number;
  width_m: number;
}

export interface BlastStyle {
  /** The fireball's radius per metre of blast radius, and its least radius. */
  size_per_radius: number;
  min_size_m: number;
  duration_s: number;
  /** The flipbook's multiplier, its fire's extra glow early on, and opacity. */
  tint: Rgb;
  emissive: number;
  opacity: number;
  /** The burst's first flash: colour, intensity, and how long it lasts. */
  flash: Rgb;
  flash_intensity: number;
  /** The burst's light spilling round it while the fire is hot. */
  spill: number;
  /** Its sparks' size over a ricochet's (speed, length, width). */
  spark_scale: number;
  flash_duration_s: number;
  sparks: number;
  /** The dust thrown up around the burst. */
  dust: Rgb;
  dust_opacity: number;
}

/** `presentation.effects`. Styles by round kind or hit kind, each with a `default`. */
export interface EffectPresentation {
  /** Instances the effect pass holds; past it an effect is dropped (and counted). */
  capacity: number;
  /** The narrowest a streak or smallest a sprite draws, pixels; thinner ones dim instead. */
  min_px: number;
  tracers: Record<string, TracerStyle>;
  flashes: Record<string, FlashStyle>;
  impacts: Record<string, ImpactStyle>;
  /** An impact puff's size by round kind. */
  impact_scale: Record<string, number>;
  sparks: SparkStyle;
  ricochet_sparks: number;
  blast: BlastStyle;
}

/** Throws on a style table without its `default`, or a non-positive capacity. */
export function validateEffects(p: EffectPresentation): EffectPresentation {
  if (!(p.capacity > 0)) throw new Error("presentation.effects.capacity must be positive");
  for (const table of ["tracers", "flashes", "impacts", "impact_scale"] as const)
    if (!p[table].default) throw new Error(`presentation.effects.${table} needs a default`);
  return p;
}

// ---- The instance layout the effect pass draws (16 floats each). ----

/** Floats per instance: a, b, colour, misc (vec4 each). */
export const EFFECT_FLOATS = 16;
/** Instance shapes (misc.x): a camera-facing streak from a to b, a glow
 *  sprite at a, a flipbook sprite at a. */
export const SHAPE = { streak: 0, glow: 1, flipbook: 2 } as const;
/** Flipbook layers in the effect atlas (`flipbooks.ts`). */
export const LAYER = { fire: 0, dust: 1 } as const;
/** Frames in each layer, in `LAYER` order. */
export const LAYER_FRAMES = [25, 64] as const;

/** What the effect pass draws this frame: `count` instances in `data`. */
export interface EffectBatch {
  data: Float32Array<ArrayBuffer>;
  count: number;
  /** Instances that did not fit this frame. */
  dropped: number;
}

export function createEffectBatch(capacity: number): EffectBatch {
  return { data: new Float32Array(capacity * EFFECT_FLOATS), count: 0, dropped: 0 };
}

function slot(batch: EffectBatch): number {
  if (batch.count * EFFECT_FLOATS >= batch.data.length) {
    batch.dropped++;
    return -1;
  }
  return batch.count++ * EFFECT_FLOATS;
}

/** A streak from a (tail, `alongA` of the way to the head) to b (`alongB`),
 *  `width` metres wide, additive. */
function streak(
  batch: EffectBatch,
  ax: number,
  ay: number,
  az: number,
  bx: number,
  by: number,
  bz: number,
  width: number,
  minPx: number,
  color: Rgb,
  intensity: number,
  alongA: number,
  alongB: number,
) {
  const o = slot(batch);
  if (o < 0) return;
  const d = batch.data;
  d[o] = ax;
  d[o + 1] = ay;
  d[o + 2] = az;
  d[o + 3] = width;
  d[o + 4] = bx;
  d[o + 5] = by;
  d[o + 6] = bz;
  d[o + 7] = minPx;
  d[o + 8] = color[0] * intensity;
  d[o + 9] = color[1] * intensity;
  d[o + 10] = color[2] * intensity;
  d[o + 11] = 0;
  d[o + 12] = SHAPE.streak;
  d[o + 13] = alongA;
  d[o + 14] = alongB;
  d[o + 15] = 0;
}

/** A glow sprite of `radius` metres at p, additive, with `rays` of star. */
function glow(
  batch: EffectBatch,
  p: Vec3,
  radius: number,
  minPx: number,
  color: Rgb,
  intensity: number,
  rotation: number,
  rays = 1,
) {
  const o = slot(batch);
  if (o < 0) return;
  const d = batch.data;
  d[o] = p[0];
  d[o + 1] = p[1];
  d[o + 2] = p[2];
  d[o + 3] = radius;
  d[o + 4] = rotation;
  d[o + 5] = minPx;
  d[o + 6] = rays;
  d[o + 7] = 0;
  d[o + 8] = color[0] * intensity;
  d[o + 9] = color[1] * intensity;
  d[o + 10] = color[2] * intensity;
  d[o + 11] = 0;
  d[o + 12] = SHAPE.glow;
  d[o + 13] = 0;
  d[o + 14] = 0;
  d[o + 15] = 0;
}

/** A flipbook sprite of `radius` metres at p: `layer`'s frame `frame`
 *  (fractional: the two frames blend), multiplied by `tint`, its bright
 *  parts lifted by `emissive`, at `opacity`, fading into what it meets
 *  over `softM` metres. */
function flipbook(
  batch: EffectBatch,
  p: Vec3,
  radius: number,
  rotation: number,
  layer: number,
  frame: number,
  tint: Rgb,
  opacity: number,
  emissive: number,
  softM: number,
) {
  const o = slot(batch);
  if (o < 0) return;
  const d = batch.data;
  d[o] = p[0];
  d[o + 1] = p[1];
  d[o + 2] = p[2];
  d[o + 3] = radius;
  d[o + 4] = rotation;
  d[o + 5] = frame;
  d[o + 6] = layer;
  d[o + 7] = softM;
  d[o + 8] = tint[0];
  d[o + 9] = tint[1];
  d[o + 10] = tint[2];
  d[o + 11] = opacity;
  d[o + 12] = SHAPE.flipbook;
  d[o + 13] = emissive;
  d[o + 14] = 0;
  d[o + 15] = 0;
}

// ---- Effects in flight. ----

const TRACER = 0;
const FLASH = 1;
const IMPACT = 2;
const SPARKS = 3;
const BLAST = 4;

/** How far off a face sparks start, metres. */
const SURFACE_LIFT_M = 0.08;
/** Sparks fall under gravity, m/s². */
const GRAVITY = 9.81;

/** One effect: every field present from creation (one shape for all kinds). */
class Effect {
  type = TRACER;
  start = 0;
  end = 0;
  /** Seconds the effect runs its course over (a tracer: one tick). */
  span = 1;
  /** A point, a direction and a size (meaning per type). */
  p = vec3.create();
  n = vec3.create();
  size = 0;
  rotation = 0;
  /** A tracer's path, flat (x, y, z), its cumulative lengths and total. */
  path: number[] = [];
  cum: number[] = [];
  length = 0;
  /** Spark velocities, flat. */
  sparks: number[] = [];
  tracer: TracerStyle | null = null;
  flash: FlashStyle | null = null;
  impact: ImpactStyle | null = null;
}

export interface EffectFrameOptions {
  tickHz: number;
  presentation: EffectPresentation;
  /** A hull's muzzle in its turret's frame (the rules' `tank_muzzle_local_m`):
   *  forward along the bearing, left, up. */
  vehicleMuzzle: P3;
}

export interface EffectStats {
  /** Effects still running, instances drawn last build, and dropped for capacity. */
  live: number;
  instances: number;
  dropped: number;
  /** The last publication taken. */
  tick: number;
}

const _build_a = vec3.create();
const _build_b = vec3.create();
const _note_dir = vec3.create();
const _note_rand = vec3.create();

const pick = <T>(table: Record<string, T>, key: string): T => table[key] ?? table.default;
const endKey = (p: P3) => `${p[0]},${p[1]},${p[2]}`;

export class EffectFrame {
  private readonly effects: Effect[] = [];
  private readonly dt: number;
  private readonly p: EffectPresentation;
  private readonly muzzle: P3;
  private lastTick = -1;
  /** Shot counters by shooter key, as last published. */
  private counters = new Map<number, number[]>();
  /** Where the last publication's still-flying stretches ended. */
  private flying = new Set<string>();
  private instances = 0;
  private dropped = 0;

  constructor(options: EffectFrameOptions) {
    this.dt = 1 / options.tickHz;
    this.p = validateEffects(options.presentation);
    this.muzzle = options.vehicleMuzzle;
  }

  /** Forget everything (a new battle). */
  reset() {
    this.effects.length = 0;
    this.counters.clear();
    this.flying.clear();
    this.lastTick = -1;
  }

  /** Take one publication. The tick it covers runs from the previous tick's
   *  presentation time to its own: every effect it causes starts in there. */
  note(pub: EffectPublication) {
    if (pub.tick === this.lastTick) return;
    if (pub.tick < this.lastTick) this.reset();
    const gap = this.lastTick >= 0 && pub.tick !== this.lastTick + 1;
    this.lastTick = pub.tick;
    const t0 = (pub.tick - 1) * this.dt;
    const t1 = pub.tick * this.dt;
    const rng = mulberry32.create((pub.tick * 2654435761) >>> 0);

    // Launches: a mount's counter rose. A hull fires from its muzzle; a
    // squad's rise goes to the soldiers who start new rounds this tick.
    const starts = new Map<number, EffectSegment[]>();
    for (const s of pub.segments) {
      if (s.shooter === null || s.path.length < 2) continue;
      if (!gap && this.flying.has(endKey(s.path[0]))) continue; // a round still flying
      const list = starts.get(s.shooter);
      if (list) list.push(s);
      else starts.set(s.shooter, [s]);
    }
    const seen = new Set<number>();
    for (const u of pub.shooters) {
      seen.add(u.key);
      const before = this.counters.get(u.key);
      const now = u.mounts.map((m) => m.shots);
      this.counters.set(u.key, now);
      if (!before) continue; // first seen: no shot to show
      for (let m = 0; m < u.mounts.length; m++) {
        const mount = u.mounts[m];
        let rose = mount.shots - (before[m] ?? mount.shots);
        if (rose <= 0) continue;
        if (u.vehicle) {
          const [f, l, h] = this.muzzle;
          const c = Math.cos(mount.bearing);
          const s = Math.sin(mount.bearing);
          vec3.set(
            _note_dir,
            Math.cos(mount.elevation) * c,
            Math.cos(mount.elevation) * s,
            Math.sin(mount.elevation),
          );
          this.addFlash(
            t0,
            u.position[0] + f * c - l * s,
            u.position[1] + f * s + l * c,
            u.position[2] + h,
            _note_dir,
            mount.kind,
            rng,
          );
          continue;
        }
        for (const id of u.members) {
          for (const s of starts.get(id) ?? []) {
            if (rose <= 0 || s.kind !== mount.kind) continue;
            rose--;
            const [a, b] = [s.path[0], s.path[1]];
            vec3.set(_note_dir, b[0] - a[0], b[1] - a[1], b[2] - a[2]);
            vec3.normalize(_note_dir, _note_dir);
            this.addFlash(t0, a[0], a[1], a[2], _note_dir, s.kind, rng);
          }
        }
      }
    }
    // Units no longer seen start over when they are seen again.
    for (const key of this.counters.keys()) if (!seen.has(key)) this.counters.delete(key);

    this.flying = new Set();
    for (const s of pub.segments) {
      if (s.path.length < 2) continue;
      const e = this.addTracer(t0, s);
      if (s.hit === "none") this.flying.add(endKey(s.path[s.path.length - 1]));
      for (const r of s.ricochets) {
        const at = t0 + (this.dt * e.cum[r.point]) / Math.max(e.length, 1e-6);
        const next = s.path[Math.min(r.point + 1, s.path.length - 1)];
        const p = s.path[r.point];
        vec3.set(_note_dir, next[0] - p[0], next[1] - p[1], next[2] - p[2]);
        vec3.normalize(_note_dir, _note_dir);
        this.addSparks(at, p, r.normal, _note_dir, this.p.ricochet_sparks, rng);
      }
      if (s.hit !== "none") {
        const end = s.path[s.path.length - 1];
        this.addImpact(t1, end, s.normal, s.hit, s.kind, rng);
      }
    }
    for (const b of pub.blasts) this.addBlast(t1, b, rng);
  }

  /** Every running effect at presentation time `clock` (seconds), into
   *  `batch`; effects that have run their course are dropped. */
  build(clock: number, batch: EffectBatch): EffectBatch {
    batch.count = 0;
    batch.dropped = 0;
    let kept = 0;
    const list = this.effects;
    for (let i = 0; i < list.length; i++) {
      const e = list[i];
      if (e.end <= clock) continue;
      list[kept++] = e;
      if (clock < e.start) continue;
      const age = clock - e.start;
      switch (e.type) {
        case TRACER:
          this.drawTracer(e, age, batch);
          break;
        case FLASH:
          this.drawFlash(e, age, batch);
          break;
        case IMPACT:
          this.drawImpact(e, age, batch);
          break;
        case SPARKS:
          this.drawSparks(e, age, batch, this.p.sparks.color, this.p.sparks.intensity);
          break;
        case BLAST:
          this.drawBlast(e, age, batch);
          break;
      }
    }
    list.length = kept;
    this.instances = batch.count;
    this.dropped = batch.dropped;
    return batch;
  }

  stats(): EffectStats {
    return {
      live: this.effects.length,
      instances: this.instances,
      dropped: this.dropped,
      tick: this.lastTick,
    };
  }

  // ---- Creation (per publication). ----

  private push(type: number, start: number, end: number): Effect {
    const e = new Effect();
    e.type = type;
    e.start = start;
    e.end = end;
    e.span = end - start;
    this.effects.push(e);
    return e;
  }

  private addTracer(t0: number, s: EffectSegment): Effect {
    const style = pick(this.p.tracers, s.kind);
    const e = this.push(TRACER, t0, t0);
    e.tracer = style;
    let total = 0;
    for (let i = 0; i < s.path.length; i++) {
      const q = s.path[i];
      if (i > 0) {
        const r = s.path[i - 1];
        total += Math.hypot(q[0] - r[0], q[1] - r[1], q[2] - r[2]);
      }
      e.path.push(q[0], q[1], q[2]);
      e.cum.push(total);
    }
    e.length = total;
    e.span = this.dt;
    // Until the streak's tail has left the stretch's end.
    e.end = total > 1e-6 ? t0 + this.dt * (1 + style.length_m / total) : t0;
    return e;
  }

  private addFlash(
    t0: number,
    x: number,
    y: number,
    z: number,
    dir: Vec3,
    kind: string,
    rng: ReturnType<typeof mulberry32.create>,
  ) {
    const style = pick(this.p.flashes, kind);
    const last = Math.max(style.duration_s, style.fireball_m > 0 ? style.fireball_s : 0);
    const e = this.push(FLASH, t0, t0 + last);
    e.flash = style;
    vec3.set(e.p, x, y, z);
    vec3.copy(e.n, dir);
    e.rotation = mulberry32.sample(rng) * Math.PI * 2;
  }

  private addImpact(
    at: number,
    point: P3,
    normal: P3 | null,
    hit: string,
    kind: string,
    rng: ReturnType<typeof mulberry32.create>,
  ) {
    const style = pick(this.p.impacts, hit);
    const e = this.push(IMPACT, at, at + style.duration_s);
    e.impact = style;
    vec3.set(e.p, point[0], point[1], point[2]);
    if (normal) vec3.set(e.n, normal[0], normal[1], normal[2]);
    else vec3.set(e.n, 0, 0, 1);
    e.size = style.size_m * pick(this.p.impact_scale, kind);
    e.rotation = mulberry32.sample(rng) * Math.PI * 2;
    e.path.push(mulberry32.sample(rng)); // the puff's first frame
    if (style.sparks > 0) this.addSparks(at, point, e.n, e.n, style.sparks, rng);
  }

  /** Sparks from `point`, thrown along `dir` and off the face `normal`. */
  private addSparks(
    at: number,
    point: P3,
    normal: P3,
    dir: Vec3,
    count: number,
    rng: ReturnType<typeof mulberry32.create>,
    scale = 1,
  ): Effect {
    const style = this.p.sparks;
    // A blast's sparks are bigger: faster, longer, thicker and longer-lived.
    const e = this.push(SPARKS, at, at + style.duration_s * Math.sqrt(scale));
    e.size = scale;
    // A hand's breadth off the face, so the face does not swallow them.
    vec3.set(
      e.p,
      point[0] + normal[0] * SURFACE_LIFT_M,
      point[1] + normal[1] * SURFACE_LIFT_M,
      point[2] + normal[2] * SURFACE_LIFT_M,
    );
    for (let k = 0; k < count; k++) {
      // A random direction, pulled toward the glance and off the face.
      const r = vec3.set(
        _note_rand,
        mulberry32.sample(rng) * 2 - 1,
        mulberry32.sample(rng) * 2 - 1,
        mulberry32.sample(rng) * 2 - 1,
      );
      const vx = dir[0] * 0.8 + normal[0] * 0.6 + r[0] * 0.6;
      const vy = dir[1] * 0.8 + normal[1] * 0.6 + r[1] * 0.6;
      const vz = dir[2] * 0.8 + normal[2] * 0.6 + r[2] * 0.6;
      const len = Math.hypot(vx, vy, vz) || 1;
      const speed = style.speed_mps * scale * (0.4 + 0.6 * mulberry32.sample(rng));
      e.sparks.push((vx / len) * speed, (vy / len) * speed, (vz / len) * speed);
    }
    return e;
  }

  private addBlast(at: number, b: EffectBlast, rng: ReturnType<typeof mulberry32.create>) {
    const style = this.p.blast;
    const e = this.push(BLAST, at, at + style.duration_s);
    vec3.set(e.p, b.point[0], b.point[1], b.point[2]);
    e.size = Math.max(style.min_size_m, b.radius * style.size_per_radius);
    e.rotation = mulberry32.sample(rng) * Math.PI * 2;
    vec3.set(_note_dir, 0, 0, 1);
    this.addSparks(at, b.point, _note_dir, _note_dir, style.sparks, rng, style.spark_scale);
  }

  // ---- Drawing (per frame, allocation-free). ----

  private drawTracer(e: Effect, age: number, batch: EffectBatch) {
    const style = e.tracer!;
    const L = e.length;
    if (L <= 1e-6) return;
    const head = (L * age) / e.span;
    const tail = head - style.length_m;
    const path = e.path;
    const cum = e.cum;
    for (let i = 0; i + 1 < cum.length; i++) {
      const c0 = cum[i];
      const c1 = cum[i + 1];
      const lo = Math.max(c0, tail);
      const hi = Math.min(c1, head);
      if (hi <= lo || c1 <= c0) continue;
      const u0 = (lo - c0) / (c1 - c0);
      const u1 = (hi - c0) / (c1 - c0);
      const o = i * 3;
      for (let k = 0; k < 3; k++) {
        _build_a[k] = path[o + k] + (path[o + 3 + k] - path[o + k]) * u0;
        _build_b[k] = path[o + k] + (path[o + 3 + k] - path[o + k]) * u1;
      }
      streak(
        batch,
        _build_a[0],
        _build_a[1],
        _build_a[2],
        _build_b[0],
        _build_b[1],
        _build_b[2],
        style.width_m,
        this.p.min_px,
        style.color,
        style.intensity,
        (lo - tail) / style.length_m,
        (hi - tail) / style.length_m,
      );
    }
  }

  private drawFlash(e: Effect, age: number, batch: EffectBatch) {
    const s = e.flash!;
    const x = age / s.duration_s;
    if (x < 1) {
      const k = (1 - x) * (1 - x);
      glow(
        batch,
        e.p,
        s.size_m * (0.7 + 0.5 * x),
        this.p.min_px,
        s.color,
        s.intensity * k,
        e.rotation,
        0,
      );
      const reach = s.tongue_m * (0.6 + 0.4 * x);
      vec3.scaleAndAdd(_build_b, e.p, e.n, reach);
      streak(
        batch,
        e.p[0],
        e.p[1],
        e.p[2],
        _build_b[0],
        _build_b[1],
        _build_b[2],
        s.size_m * 0.9,
        this.p.min_px,
        s.color,
        s.intensity * k,
        1,
        0.15,
      );
    }
    if (s.fireball_m > 0 && age < s.fireball_s) {
      const f = age / s.fireball_s;
      vec3.scaleAndAdd(_build_a, e.p, e.n, s.fireball_m * (0.2 + 0.6 * f));
      flipbook(
        batch,
        _build_a,
        s.fireball_m * (0.6 + 0.6 * Math.sqrt(f)),
        e.rotation,
        LAYER.fire,
        f * 10,
        s.color,
        (1 - f) * (1 - f),
        s.intensity * 0.25 * (1 - f),
        0.5,
      );
    }
  }

  private drawImpact(e: Effect, age: number, batch: EffectBatch) {
    const s = e.impact!;
    const x = age / e.span;
    const size = e.size * (0.7 + 0.6 * Math.sqrt(x));
    // Off the face along its normal, and rising a little as it spreads.
    vec3.scaleAndAdd(_build_a, e.p, e.n, size * 0.45);
    _build_a[2] += e.size * 0.3 * x;
    const opacity = s.opacity * Math.min(1, x * 12) * (1 - x) * (1 - x);
    flipbook(
      batch,
      _build_a,
      size,
      e.rotation,
      LAYER.dust,
      (e.path[0] + x * 0.4) * (LAYER_FRAMES[LAYER.dust] - 1),
      s.color,
      opacity,
      0,
      size * 0.6,
    );
    if (s.flash > 0 && x < 0.12) {
      const k = 1 - x / 0.12;
      // Off the face by its own radius: a sprite in the face would fade into it.
      const r = e.size * 0.35;
      vec3.scaleAndAdd(_build_a, e.p, e.n, r);
      glow(batch, _build_a, r, this.p.min_px, this.p.sparks.color, s.flash * k * k, e.rotation);
    }
  }

  private drawSparks(e: Effect, age: number, batch: EffectBatch, color: Rgb, intensity: number) {
    const s = this.p.sparks;
    const fade = 1 - age / e.span;
    const v = e.sparks;
    for (let k = 0; k + 2 < v.length; k += 3) {
      const speed = Math.hypot(v[k], v[k + 1], v[k + 2]) || 1;
      const lag = Math.min(age, (s.length_m * e.size) / speed);
      const t1 = age;
      const t0 = age - lag;
      for (let j = 0; j < 3; j++) {
        const g = j === 2 ? -0.5 * GRAVITY : 0;
        _build_a[j] = e.p[j] + v[k + j] * t0 + g * t0 * t0;
        _build_b[j] = e.p[j] + v[k + j] * t1 + g * t1 * t1;
      }
      streak(
        batch,
        _build_a[0],
        _build_a[1],
        _build_a[2],
        _build_b[0],
        _build_b[1],
        _build_b[2],
        s.width_m * e.size,
        this.p.min_px * 0.5,
        color,
        intensity * fade,
        0,
        1,
      );
    }
  }

  private drawBlast(e: Effect, age: number, batch: EffectBatch) {
    const s = this.p.blast;
    const x = age / e.span;
    // Dust thrown up around the burst, under the fire.
    const dustSize = e.size * (0.6 + 0.6 * Math.sqrt(x));
    vec3.copy(_build_a, e.p);
    _build_a[2] += dustSize * 0.35;
    flipbook(
      batch,
      _build_a,
      dustSize,
      e.rotation + 1.3,
      LAYER.dust,
      x * 20,
      s.dust,
      s.dust_opacity * Math.min(1, x * 8) * (1 - x),
      0,
      dustSize * 0.5,
    );
    // The fireball: the flipbook's whole life over the blast's, rising.
    vec3.copy(_build_a, e.p);
    _build_a[2] += e.size * (0.55 + 0.5 * x);
    const heat = Math.max(0, 1 - x * 2.2);
    // The burst's light on what is around it, fading as the fire cools.
    if (heat > 0) {
      vec3.copy(_build_a, e.p);
      _build_a[2] += e.size * 0.5;
      glow(batch, _build_a, e.size * 1.2, 0, s.flash, s.spill * heat * heat, e.rotation, 0);
    }
    flipbook(
      batch,
      _build_a,
      e.size * (0.75 + 0.5 * Math.sqrt(x)),
      e.rotation,
      LAYER.fire,
      Math.min(x * 1.15, 1) * (LAYER_FRAMES[LAYER.fire] - 1),
      s.tint,
      s.opacity * Math.min(1, (1 - x) * 3),
      s.emissive * heat * heat,
      e.size * 0.5,
    );
    if (age < s.flash_duration_s) {
      const k = 1 - age / s.flash_duration_s;
      vec3.copy(_build_a, e.p);
      _build_a[2] += e.size * 0.4;
      glow(
        batch,
        _build_a,
        e.size * 0.3,
        this.p.min_px,
        s.flash,
        s.flash_intensity * k * k,
        e.rotation,
        0.5,
      );
    }
  }
}
