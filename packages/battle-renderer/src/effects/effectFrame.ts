// Combat effects: the one owner. What a side's publications say
// happened becomes short-lived effects, and each frame they become the effect
// pass's instances at the presentation clock:
//
// - a tracer for every visible stretch of flight, styled by its round kind
//   (a glow and a hot core along a tail that fades behind the round, the
//   round itself where it is seen as an object, and a smoke trail where its
//   row has one), running along the stretch over the tick that flew it;
// - a muzzle flash for every launch (`launches.ts`): on the muzzle of the
//   mount or soldier that fired as it is drawn at each frame (a
//   `MuzzleSource`), else at the published launch point;
// - an impact puff where a stretch ends in a hit, by hit class, off the
//   surface along its published normal (sparks off a hull);
// - sparks at every ricochet corner, thrown off the glancing face;
// - a fireball for every published blast, and the dirt and smoke it throws
//   up, rising and drifting downwind;
// - dust behind a vehicle the side sees move, by the distance it covers;
// - flames and a rising smoke column over every smoke source the side knows
//   (a wreck; a future smoke-screen body is another row of the same table),
//   burning, then smouldering, then out.
//
// Every effect has a published cause, and nothing else: no rule, no
// simulation state (enemy stretches arrive already clipped to seen ground).
// Smoke is presentation only: the simulation has none, so it hides nothing.
// Every effect's life is bounded (`EffectLifetime`, `maxEffectLifetime`);
// all of them run on the presentation clock, so pause holds them and a jump
// of the clock (a hidden tab) spawns only what would still be alive.
// A publication is taken once: the same tick again is ignored, an earlier one
// starts over (a new battle). Presentation only; `presentation.effects` holds
// every curve and size.
import { vec3, type Vec3 } from "math";
import { mulberry32 } from "math/random";
import type { MountMuzzle } from "@packages/scene-assets/src/mountMuzzle";
import { LaunchTracker, type Launch } from "./launches";
import { createCastLightList, offerCastLight, type CastLightList } from "../light/castLights";

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
  /** Where a hull's mount fires from (its `mounts` row); null for a
   *  soldier's weapon, whose rounds start at his body. */
  muzzle: MountMuzzle | null;
}

/** A unit whose shots the side sees: an own unit or an identified enemy. */
export interface EffectShooter {
  /** Unique across both sides for the life of the battle. */
  key: number;
  position: P3;
  /** A hull's half extents (length, width, height): its shots leave its
   *  mounts' muzzles and it raises dust. Null for infantry (a soldier's rifle). */
  half: P3 | null;
  /** The hull's heading (world radians), which turns a hull-carried pivot. */
  yaw: number;
  /** Its soldiers' ids (infantry). */
  members: readonly number[];
  mounts: readonly EffectMount[];
}

/** Something the side knows smokes: a wreck today. Presentation only (the
 *  simulation has no smoke). A future smoke-screen body is a source of its
 *  own kind with the same look machinery. */
export interface EffectSmokeSource {
  /** Stable while the source is known. */
  key: string;
  /** Its look: a `presentation.effects.smoke` row. */
  kind: string;
  /** Its footprint: the base centre, yaw, and half extents (length, width, height). */
  center: P3;
  yaw: number;
  half: P3;
}

/** Where a shot's muzzle is drawn at this frame: the model's, not the
 *  simulation's. A flash sits on it, so it stays on the drawn barrel as the
 *  model is interpolated, turns, pitches and recoils, and on the weapon a
 *  soldier holds (whose rounds the simulation starts at his body). */
export interface MuzzleSource {
  /** Write the drawn muzzle of `shooter`'s (`EffectShooter.key`) mount
   *  `mount`, or of its soldier `soldier` when set, into `at`; false when
   *  nothing of it is drawn. */
  muzzle(shooter: number, mount: number, soldier: number | null, at: Vec3): boolean;
}

/** One publication, as the effects read it. */
export interface EffectPublication {
  tick: number;
  segments: readonly EffectSegment[];
  blasts: readonly EffectBlast[];
  shooters: readonly EffectShooter[];
  smokes: readonly EffectSmokeSource[];
}

/** One light of a round in flight: its colour, strength and width, metres. */
export interface TracerLight {
  color: Vec3;
  intensity: number;
  width_m: number;
}

/** The light an effect casts on what is round it (`light/castLights.ts`):
 *  its colour, its irradiance at the centre (the sun's radiance is a few),
 *  and how far it reaches, metres. */
export interface CastStyle {
  color: Vec3;
  intensity: number;
  radius_m: number;
}

/** A cast light that burns out: over `duration_s`, fading as 1 − (age/duration)². */
export interface TransientCast extends CastStyle {
  duration_s: number;
}

/** A round in flight (`presentation.effects.tracers.<kind>`): a soft
 *  coloured glow along a tail that fades out behind the round, brightest at
 *  its front; optionally a hot core, a dark body and a smoke trail (each
 *  absent: none). */
export interface TracerStyle {
  /** The tail is where the round was over the last `tail_s` seconds, so a
   *  fast round draws a long bolt and a slow one a short point; held within
   *  `tail_m` [least, most] metres. */
  tail_s: number;
  tail_m: [number, number];
  /** The soft glow over the whole tail, at least `min_px` wide on screen
   *  (dimmed rather than drawn thinner). */
  glow: TracerLight & { min_px: number };
  /** A hot core over the tail's front `share`. */
  core?: TracerLight & { share: number };
  /** The round seen as a dark object (a grenade, a shell), radius metres. */
  body?: { color: Vec3; size_m: number };
  /** A smoke trail it leaves (a missile's motor, a grenade's fuze). */
  smoke?: TrailStyle;
  /** The light it casts from its head while it flies (a tracer's faint
   *  glow; a missile's motor, a strong moving light). */
  cast?: CastStyle;
}

/** A smoke trail: one unbroken ribbon along the whole flight at `ribbon`
 *  opacity, and a puff every `spacing_m` along it for body. Both rise,
 *  widen and drift downwind alike, fading out over `life_s`. */
export interface TrailStyle extends PuffStyle {
  ribbon: number;
  spacing_m: number;
}

export interface FlashStyle {
  color: Vec3;
  intensity: number;
  /** The glow's radius, the forward tongue's length, metres. */
  size_m: number;
  tongue_m: number;
  duration_s: number;
  /** A muzzle fireball's size (0 for none) and how long it burns. */
  fireball_m: number;
  fireball_s: number;
  /** The shot's light on what is round the muzzle, centred `forward_m`
   *  along the shot (behind it when negative: a launcher's back-blast). */
  cast?: TransientCast & { forward_m?: number };
}

export interface ImpactStyle {
  /** The puff's lit colour and opacity. */
  color: Vec3;
  opacity: number;
  size_m: number;
  duration_s: number;
  /** Sparks thrown off the surface, and a hot flash's intensity (0 for none). */
  sparks: number;
  flash: number;
  /** Its flash's light off the face, within the impact's life, reaching
   *  farther for a bigger round (by the square root of its `impact_scale`). */
  cast?: TransientCast;
}

export interface SparkStyle {
  color: Vec3;
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
  tint: Vec3;
  emissive: number;
  opacity: number;
  /** The burst's first flash: colour, intensity, and how long it lasts. */
  flash: Vec3;
  flash_intensity: number;
  /** The glow in the air round the burst while the fire is hot (a sprite: its light on the ground is `cast`). */
  spill: number;
  /** Its sparks' size over a ricochet's (speed, length, width). */
  spark_scale: number;
  flash_duration_s: number;
  sparks: number;
  /** The dirt thrown up in a column, and the smoke that follows it; their
   *  sizes and speeds scale with the square root of the fireball's size
   *  over `min_size_m`. */
  plume: PuffBurst;
  smoke: PuffBurst;
  /** The burst's light on what is round it, within the blast's life; its
   *  reach grows with the square root of the fireball's size over `min_size_m`. */
  cast: TransientCast;
}

/** A soft sun-lit puff (smoke, dust) that rises, spreads and drifts downwind. */
export interface PuffStyle {
  /** Its albedo, lit by the world's light: the sun on its sunward side,
   *  the sky all over (`effectPass.ts`). */
  albedo: Vec3;
  opacity: number;
  /** Radius at birth and at death, metres. */
  size_m: [number, number];
  life_s: number;
  /** Upward speed at birth (it slows as it spreads) and random sideways speed, m/s. */
  rise_mps: number;
  spread_mps: number;
}

/** `count` puffs born over the first `over_s` seconds. */
export interface PuffBurst extends PuffStyle {
  count: number;
  over_s: number;
}

/** Flames licking over a burning source: short fire flipbook sprites. */
export interface FlameStyle {
  tint: Vec3;
  emissive: number;
  size_m: number;
  life_s: number;
  rate_hz: number;
  /** The fire's glow in the air round it: a sprite's colour, intensity and radius (its light on the ground is `cast`). */
  light: Vec3;
  light_intensity: number;
  light_m: number;
  /** The fire's light cast on the ground and hull round it, flickering. */
  cast?: CastStyle;
}

/** A smoke source's look (`presentation.effects.smoke.<kind>`): it burns
 *  for `burn_s` (flames and thick smoke), smoulders for `smoulder_s` (thin
 *  smoke), and is then out. */
export interface SmokeSourceStyle {
  burn_s: number;
  smoulder_s: number;
  flame: FlameStyle;
  smoke: PuffStyle;
  smoke_hz: number;
  smoulder: PuffStyle;
  smoulder_hz: number;
}

/** Dust behind a moving hull: a puff off each track every `spacing_m`,
 *  thicker with speed up to `full_speed_mps`. */
export interface DustStyle extends PuffStyle {
  spacing_m: number;
  min_speed_mps: number;
  full_speed_mps: number;
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
  dust: DustStyle;
  /** Smoke sources' looks by kind; a kind without one draws nothing. */
  smoke: Record<string, SmokeSourceStyle>;
  /** Instances every smoke source together may hold at once. Past it each
   *  source thins alike (every puff and flame kept by its own draw), so
   *  every known wreck still smokes, fainter, and nothing else is starved. */
  smoke_budget: number;
  /** The wind that carries smoke and dust, metres a second (east, north). */
  wind_mps: [number, number];
  /** How far round a surface a cast light reaches past facing it, 0..1:
   *  0 is plain Lambert; more lights a pool's rim on flat ground. */
  cast_wrap: number;
}

/** Throws on a style table without its `default`, a non-positive capacity,
 *  or an unbounded life. */
export function validateEffects(p: EffectPresentation): EffectPresentation {
  if (!(p.capacity > 0)) throw new Error("presentation.effects.capacity must be positive");
  for (const table of ["tracers", "flashes", "impacts", "impact_scale"] as const)
    if (!p[table].default) throw new Error(`presentation.effects.${table} needs a default`);
  for (const [kind, t] of Object.entries(p.tracers)) validateTracer(kind, t);
  if (!(p.cast_wrap >= 0 && p.cast_wrap <= 1))
    throw new Error("presentation.effects.cast_wrap must be in [0, 1]");
  const cast = (at: string, c: CastStyle | undefined, fades: boolean) =>
    c && validateCast(`presentation.effects.${at}.cast`, c, fades);
  for (const [k, t] of Object.entries(p.tracers)) cast(`tracers.${k}`, t.cast, false);
  for (const [k, f] of Object.entries(p.flashes)) cast(`flashes.${k}`, f.cast, true);
  for (const [k, i] of Object.entries(p.impacts)) cast(`impacts.${k}`, i.cast, true);
  for (const [k, s] of Object.entries(p.smoke)) cast(`smoke.${k}.flame`, s.flame.cast, false);
  cast("blast", p.blast.cast, true);
  // A burst's or an impact's light burns within its own life.
  for (const [k, i] of Object.entries(p.impacts))
    if (i.cast && i.cast.duration_s > i.duration_s)
      throw new Error(`presentation.effects.impacts.${k}.cast outlives its impact`);
  if (p.blast.cast.duration_s > p.blast.duration_s)
    throw new Error("presentation.effects.blast.cast outlives its blast");
  if (!Number.isFinite(maxEffectLifetime(p)))
    throw new Error("presentation.effects: every life must be finite");
  return p;
}

/** Throws on a tracer row that could not draw: a tail with no time (or
 *  more than a tracer lives past its tick) or an empty range, a core share
 *  outside (0, 1], or a smoke trail with no spacing. */
function validateTracer(kind: string, t: TracerStyle) {
  const at = `presentation.effects.tracers.${kind}`;
  if (!(t.tail_s > 0 && t.tail_s <= TRACER_TAIL_MAX_S))
    throw new Error(`${at}.tail_s must be in (0, ${TRACER_TAIL_MAX_S}]`);
  if (!(t.tail_m[0] > 0 && t.tail_m[1] >= t.tail_m[0]))
    throw new Error(`${at}.tail_m must be [least, most], least above 0`);
  if (t.core && !(t.core.share > 0 && t.core.share <= 1))
    throw new Error(`${at}.core.share must be in (0, 1]`);
  if (t.smoke && !(t.smoke.spacing_m > 0))
    throw new Error(`${at}.smoke.spacing_m must be positive`);
}

/** Throws on a cast light that could not light: no reach, a negative
 *  intensity, or (one that burns out) no duration. */
function validateCast(at: string, c: CastStyle, fades: boolean) {
  if (!(c.radius_m > 0)) throw new Error(`${at}.radius_m must be positive`);
  if (!(c.intensity >= 0)) throw new Error(`${at}.intensity must not be negative`);
  if (fades && !((c as TransientCast).duration_s > 0))
    throw new Error(`${at}.duration_s must be positive`);
}

/** When an effect starts and ends on the presentation clock, seconds. */
export interface EffectLifetime {
  start: number;
  end: number;
}

/** The longest any one effect lives, seconds: after the last publication,
 *  everything is gone this long after its tick. (A smoke source emits for
 *  its burn and smoulder, `sourceLifetime`; each puff lives this at most.) */
export function maxEffectLifetime(p: EffectPresentation): number {
  const b = p.blast;
  let longest = Math.max(
    b.duration_s,
    b.plume.over_s + b.plume.life_s,
    b.smoke.over_s + b.smoke.life_s,
    p.dust.life_s,
    p.sparks.duration_s * Math.sqrt(Math.max(1, b.spark_scale)),
  );
  for (const f of Object.values(p.flashes))
    longest = Math.max(longest, f.duration_s, f.fireball_s, f.cast?.duration_s ?? 0);
  for (const i of Object.values(p.impacts)) longest = Math.max(longest, i.duration_s);
  for (const s of Object.values(p.smoke))
    longest = Math.max(longest, s.flame.life_s, s.smoke.life_s, s.smoulder.life_s);
  // A round's smoke trail is born by the end of its tick.
  for (const t of Object.values(p.tracers))
    if (t.smoke) longest = Math.max(longest, t.smoke.life_s);
  // A tracer lives its tick and the time its tail takes to leave it.
  return longest + 1;
}

/** How long a smoke source of this look emits after it is first known, seconds. */
export const sourceLifetime = (s: SmokeSourceStyle) => s.burn_s + s.smoulder_s;

// ---- The instance layout the effect pass draws (16 floats each). ----

/** Floats per instance: a, b, colour, misc (vec4 each). */
export const EFFECT_FLOATS = 16;
/** Instance shapes (misc.x): a camera-facing streak from a to b (a lit
 *  smoke ribbon when its colour carries an opacity), a glow sprite at a (a
 *  solid disc when its colour carries an opacity), a flipbook sprite at a. */
export const SHAPE = { streak: 0, glow: 1, flipbook: 2 } as const;
/** Flipbook layers in the effect atlas (`flipbooks.ts`). */
export const LAYER = { fire: 0, dust: 1 } as const;
/** Frames in each layer, in `LAYER` order. */
export const LAYER_FRAMES = [25, 64] as const;

/** What the effect pass draws this frame: `count` instances in `data`, and
 *  the light they cast on the world (`light/castLights.ts`). */
export interface EffectBatch {
  data: Float32Array<ArrayBuffer>;
  count: number;
  /** Instances that did not fit this frame. */
  dropped: number;
  lights: CastLightList;
}

export function createEffectBatch(capacity: number): EffectBatch {
  return {
    data: new Float32Array(capacity * EFFECT_FLOATS),
    count: 0,
    dropped: 0,
    lights: createCastLightList(),
  };
}

function slot(batch: EffectBatch): number {
  if (batch.count * EFFECT_FLOATS >= batch.data.length) {
    batch.dropped++;
    return -1;
  }
  return batch.count++ * EFFECT_FLOATS;
}

/** A streak from a (tail, `alongA` of the way to the head) to b (`alongB`),
 *  `width` metres wide, additive: a sharp line, or a soft glow when `soft`. */
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
  color: Vec3,
  intensity: number,
  alongA: number,
  alongB: number,
  soft = false,
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
  d[o + 15] = soft ? 1 : 0;
}

/** A glow sprite of `radius` metres at p, additive, with `rays` of star. */
function glow(
  batch: EffectBatch,
  p: Vec3,
  radius: number,
  minPx: number,
  color: Vec3,
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

/** A solid disc of `radius` metres at p, at least `minPx` across (its
 *  coverage thinned below that), in `color` at `opacity`, blended over:
 *  a round seen as an object. */
function disc(
  batch: EffectBatch,
  p: Vec3,
  radius: number,
  minPx: number,
  color: Vec3,
  opacity: number,
) {
  const o = slot(batch);
  if (o < 0) return;
  const d = batch.data;
  d[o] = p[0];
  d[o + 1] = p[1];
  d[o + 2] = p[2];
  d[o + 3] = radius;
  d[o + 4] = 0;
  d[o + 5] = minPx;
  d[o + 6] = 0;
  d[o + 7] = 0;
  d[o + 8] = color[0] * opacity;
  d[o + 9] = color[1] * opacity;
  d[o + 10] = color[2] * opacity;
  d[o + 11] = opacity;
  d[o + 12] = SHAPE.glow;
  d[o + 13] = 0;
  d[o + 14] = 0;
  d[o + 15] = 0;
}

/** A lit smoke ribbon from a to b, `width` metres wide (at least `minPx`,
 *  thinned below that), `albedo` at `opacity`, blended over: a streak whose
 *  colour carries an opacity. Its ends are cut square, so pieces laid end
 *  to end neither gap nor double up. */
function ribbon(
  batch: EffectBatch,
  a: Vec3,
  b: Vec3,
  width: number,
  minPx: number,
  albedo: Vec3,
  opacity: number,
) {
  const o = slot(batch);
  if (o < 0) return;
  const d = batch.data;
  d[o] = a[0];
  d[o + 1] = a[1];
  d[o + 2] = a[2];
  d[o + 3] = width;
  d[o + 4] = b[0];
  d[o + 5] = b[1];
  d[o + 6] = b[2];
  d[o + 7] = minPx;
  d[o + 8] = albedo[0];
  d[o + 9] = albedo[1];
  d[o + 10] = albedo[2];
  d[o + 11] = opacity;
  d[o + 12] = SHAPE.streak;
  d[o + 13] = 1;
  d[o + 14] = 1;
  d[o + 15] = 0;
}

/** A flipbook sprite of `radius` metres at p: `layer`'s frame `frame`
 *  (fractional: the two frames blend), multiplied by `tint`, its bright
 *  parts lifted by `emissive`, at `opacity`, fading into what it meets
 *  over `softM` metres. `lit`: `tint` is an albedo the frame's sun lights
 *  (smoke, dust); otherwise a colour of its own (fire). */
function flipbook(
  batch: EffectBatch,
  p: Vec3,
  radius: number,
  rotation: number,
  layer: number,
  frame: number,
  tint: Vec3,
  opacity: number,
  emissive: number,
  softM: number,
  lit = false,
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
  d[o + 14] = lit ? 1 : 0;
  d[o + 15] = 0;
}

// ---- Effects in flight. ----

const TRACER = 0;
const FLASH = 1;
const IMPACT = 2;
const SPARKS = 3;
const BLAST = 4;
const PUFF = 5;
const FLAME = 6;
const TRAIL = 7;

/** How far off a face sparks start, metres. */
const SURFACE_LIFT_M = 0.08;
/** Sparks fall under gravity, m/s². */
const GRAVITY = 9.81;
/** A tracer on a very short stretch lives at most this past its tick, s. */
const TRACER_TAIL_MAX_S = 1;
/** Frames a puff's flipbook turns through over its life. */
const PUFF_TURN_FRAMES = 24;
/** A puff fades in over its first quarter second, per second. */
const PUFF_FADE_IN = 4;
/** A trail's ribbon's width over its puffs' diameter: inside them, so the
 *  puffs give its edge body. */
const RIBBON_WIDTH = 0.7;

/** One effect: every field present from creation (one shape for all kinds). */
class Effect implements EffectLifetime {
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
  /** A flash's launch: the shooter key, mount and soldier (`Launch`). */
  shooter = 0;
  mount = 0;
  soldier: number | null = null;
  impact: ImpactStyle | null = null;
  puff: PuffStyle | null = null;
  trail: TrailStyle | null = null;
  flame: FlameStyle | null = null;
  /** What it is, for a light it casts (`CastLightList.causes`). */
  cause = "";
  /** A puff's size and opacity over its style's, and its flipbook's first frame. */
  scale = 1;
  alpha = 1;
  frame = 0;
}

/** A smoke source the side knows, and how far its emission has got. */
interface Source {
  key: string;
  /** What its light is (`CastLightList.causes`): `fire:<kind>`. */
  cause: string;
  style: SmokeSourceStyle;
  seed: number;
  /** When it was first known, on the presentation clock. */
  start: number;
  /** The top of its footprint's centre, its yaw and half extents. */
  x: number;
  y: number;
  top: number;
  yaw: number;
  hx: number;
  hy: number;
  /** Its own size and thickness, so no two wrecks' columns match. */
  scale: number;
  alpha: number;
  /** Puffs and flames emitted so far, per stream (smoke, smoulder, flame). */
  emitted: [number, number, number];
  /** Seen in the latest publication. */
  known: boolean;
}

/** Where a hull the side sees was last published, and how far it has gone
 *  since its last dust. */
interface Mover {
  x: number;
  y: number;
  z: number;
  carry: number;
  seen: boolean;
}

/** The instances a source of this look holds `age` seconds after it is
 *  first known, once its streams are running steadily. */
function steadyInstances(s: SmokeSourceStyle, age: number): number {
  if (age < s.burn_s) return s.smoke_hz * s.smoke.life_s + s.flame.rate_hz * s.flame.life_s + 1;
  if (age < s.burn_s + s.smoulder_s) return s.smoulder_hz * s.smoulder.life_s;
  return 0;
}

/** A 32-bit hash of a string (FNV-1a), a source's seed. */
function hashKey(key: string): number {
  let h = 2166136261;
  for (let i = 0; i < key.length; i++) h = Math.imul(h ^ key.charCodeAt(i), 16777619);
  return h >>> 0;
}

export interface EffectFrameOptions {
  tickHz: number;
  presentation: EffectPresentation;
}

export interface EffectStats {
  /** Effects still running, instances drawn last build, and dropped for capacity. */
  live: number;
  instances: number;
  dropped: number;
  /** Smoke sources still burning or smouldering. */
  sources: number;
  /** The last publication taken. */
  tick: number;
}

const _build_a = vec3.create();
const _build_b = vec3.create();
const _note_dir = vec3.create();
const _note_rand = vec3.create();
const _note_at = vec3.create();
const _flash_at = vec3.create();
const _trail_p = vec3.create();
const _trail_rise = vec3.create();

const pick = <T>(table: Record<string, T>, key: string): T => table[key] ?? table.default;

/** A puff's radius `x` of the way through its life. */
const puffRadius = (s: PuffStyle, x: number) =>
  s.size_m[0] + (s.size_m[1] - s.size_m[0]) * Math.sqrt(x);

/** The point `at` metres along a tracer's path, into `out`. */
function pointAlong(e: Effect, at: number, out: Vec3): Vec3 {
  const { path, cum } = e;
  let i = 0;
  while (i + 2 < cum.length && cum[i + 1] < at) i++;
  const u = (at - cum[i]) / Math.max(cum[i + 1] - cum[i], 1e-6);
  const o = i * 3;
  for (let k = 0; k < 3; k++) out[k] = path[o + k] + (path[o + 3 + k] - path[o + k]) * u;
  return out;
}

export class EffectFrame {
  private readonly effects: Effect[] = [];
  /** Effects that have run their course, for reuse. */
  private readonly spent: Effect[] = [];
  private readonly sources: Source[] = [];
  private readonly sourceIndex = new Map<string, Source>();
  /** The presentation time the last publication covers up to. */
  private noted = 0;
  /** The share of smoke sources' births kept, under `smoke_budget`. */
  private keep = 1;
  private readonly movers = new Map<number, Mover>();
  private readonly dt: number;
  private readonly p: EffectPresentation;
  private lastTick = -1;
  private readonly launches: LaunchTracker;
  private instances = 0;
  private dropped = 0;

  constructor(options: EffectFrameOptions) {
    this.dt = 1 / options.tickHz;
    this.p = validateEffects(options.presentation);
    this.launches = new LaunchTracker();
  }

  /** Forget everything (a new battle). */
  reset() {
    for (const e of this.effects) this.spent.push(e);
    this.effects.length = 0;
    this.sources.length = 0;
    this.sourceIndex.clear();
    this.movers.clear();
    this.launches.reset();
    this.lastTick = -1;
    this.noted = 0;
    this.keep = 1;
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

    // Launches (`launches.ts`): a flash at each, at its tick's start.
    for (const l of this.launches.note(pub, gap)) this.addFlash(t0, l, rng);

    for (const s of pub.segments) {
      if (s.path.length < 2) continue;
      const e = this.addTracer(t0, s, rng);
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
    this.noteMovers(pub, gap, t0);
    this.noteSources(pub, t0, t1);
  }

  /** Dust behind every hull the side sees move: a puff off each track for
   *  every `spacing_m` it covers, at the time and place it passed. A hull
   *  first seen, or seen again after a gap, starts without dust. */
  private noteMovers(pub: EffectPublication, gap: boolean, t0: number) {
    const style = this.p.dust;
    for (const m of this.movers.values()) m.seen = false;
    for (const u of pub.shooters) {
      if (!u.half) continue;
      const [x, y, z] = u.position;
      const m = this.movers.get(u.key);
      if (!m) {
        this.movers.set(u.key, { x, y, z, carry: 0, seen: true });
        continue;
      }
      m.seen = true;
      const dx = x - m.x;
      const dy = y - m.y;
      const dist = Math.hypot(dx, dy);
      const speed = dist / this.dt;
      if (!gap && speed >= style.min_speed_mps && dist > 1e-6) {
        const fx = dx / dist;
        const fy = dy / dist;
        const [hl, hw] = u.half;
        const thick = Math.min(1, speed / style.full_speed_mps);
        let along = style.spacing_m - m.carry;
        while (along <= dist) {
          const u01 = along / dist;
          const px = m.x + dx * u01;
          const py = m.y + dy * u01;
          const pz = m.z + (z - m.z) * u01;
          const at = t0 + this.dt * u01;
          const rng = mulberry32.create(
            hashKey(`${u.key}:${Math.round(px * 100)},${Math.round(py * 100)}`),
          );
          for (let side = -1; side <= 1; side += 2) {
            const tx = px - fx * hl * 1.1 - fy * hw * 0.75 * side;
            const ty = py - fy * hl * 1.1 + fx * hw * 0.75 * side;
            this.addPuff(at, tx, ty, pz + 0.3, style, 1, thick, rng);
          }
          along += style.spacing_m;
        }
        m.carry = dist - (along - style.spacing_m);
      } else m.carry = 0;
      m.x = x;
      m.y = y;
      m.z = z;
    }
    for (const [key, m] of this.movers) if (!m.seen) this.movers.delete(key);
  }

  /** Smoke sources: each known one burns from the tick it is first known,
   *  then smoulders, then is out. Every puff and flame has its own time,
   *  `start + k / rate`, and its own seed, so what is drawn does not depend
   *  on how publications arrive; after a gap only those still alive are
   *  made. A source no longer published stops; its puffs live out. */
  private noteSources(pub: EffectPublication, t0: number, t1: number) {
    for (const src of this.sources) src.known = false;
    for (const k of pub.smokes) {
      let src = this.sourceIndex.get(k.key);
      const style = this.p.smoke[k.kind];
      if (!style) continue; // a kind with no look smokes nothing
      if (!src) {
        const seed = hashKey(k.key);
        src = {
          key: k.key,
          cause: `fire:${k.kind}`,
          style,
          seed,
          start: t0,
          x: k.center[0],
          y: k.center[1],
          top: k.center[2] + k.half[2] * 2,
          yaw: k.yaw,
          hx: k.half[0],
          hy: k.half[1],
          scale: 0.75 + 0.5 * ((seed & 0xffff) / 0xffff),
          alpha: 0.75 + 0.25 * ((seed >>> 16) / 0xffff),
          emitted: [0, 0, 0],
          known: true,
        };
        this.sources.push(src);
        this.sourceIndex.set(k.key, src);
      }
      src.known = true;
    }
    let kept = 0;
    this.noted = t1;
    let demand = 0;
    for (const src of this.sources) {
      if (!src.known) {
        this.sourceIndex.delete(src.key);
        continue;
      }
      this.sources[kept++] = src;
      demand += steadyInstances(src.style, t1 - src.start);
    }
    this.sources.length = kept;
    this.keep = Math.min(1, this.p.smoke_budget / Math.max(demand, 1));
    for (const src of this.sources) {
      const st = src.style;
      const burnEnd = src.start + st.burn_s;
      this.emit(src, 0, src.start, burnEnd, st.smoke_hz, st.smoke.life_s, t1);
      this.emit(src, 1, burnEnd, burnEnd + st.smoulder_s, st.smoulder_hz, st.smoulder.life_s, t1);
      this.emit(src, 2, src.start, burnEnd, st.flame.rate_hz, st.flame.life_s, t1);
    }
  }

  /** Stream `stream` of `src`: one birth every 1/`hz` s from `from` to
   *  `to`, up to `t1`, skipping any that would already be dead. */
  private emit(
    src: Source,
    stream: 0 | 1 | 2,
    from: number,
    to: number,
    hz: number,
    life: number,
    t1: number,
  ) {
    if (hz <= 0) return;
    const last = Math.ceil((Math.min(to, t1) - from) * hz) - 1;
    const first = Math.max(src.emitted[stream], Math.ceil((t1 - life - from) * hz));
    const st = src.style;
    const c = Math.cos(src.yaw);
    const s = Math.sin(src.yaw);
    for (let k = first; k <= last; k++) {
      const at = from + k / hz;
      const rng = mulberry32.create(
        (src.seed ^ Math.imul(k + 1, 2654435761) ^ (stream << 29)) >>> 0,
      );
      if (mulberry32.sample(rng) >= this.keep) continue; // over the budget
      // A point on the footprint's top, toward its middle.
      const u = (mulberry32.sample(rng) * 2 - 1) * 0.6;
      const v = (mulberry32.sample(rng) * 2 - 1) * 0.6;
      const x = src.x + u * src.hx * c - v * src.hy * s;
      const y = src.y + u * src.hx * s + v * src.hy * c;
      if (stream === 2) {
        // Flames die down over the burn's last third.
        const left = (to - at) / (to - from);
        this.addFlame(at, x, y, src.top - 0.3, st.flame, Math.min(1, left * 3), rng);
      } else {
        const style = stream === 0 ? st.smoke : st.smoulder;
        this.addPuff(at, x, y, src.top, style, src.scale, src.alpha, rng);
      }
    }
    src.emitted[stream] = Math.max(src.emitted[stream], last + 1);
  }

  /** Every running effect at presentation time `clock` (seconds), into
   *  `batch`; effects that have run their course are dropped. `muzzles` says
   *  where the models are drawn at `clock`, for the flashes to sit on. */
  build(clock: number, batch: EffectBatch, muzzles: MuzzleSource | null = null): EffectBatch {
    batch.count = 0;
    batch.dropped = 0;
    batch.lights.count = 0;
    batch.lights.dropped = 0;
    batch.lights.wrap = this.p.cast_wrap;
    let kept = 0;
    const list = this.effects;
    // Trails' ribbons first, under every puff: drawn in their turn, a later
    // stretch's ribbon would cover half of an earlier stretch's puffs.
    for (let i = 0; i < list.length; i++) {
      const e = list[i];
      if (e.type === TRAIL && e.start <= clock && clock < e.end)
        this.drawTrail(e, clock - e.start, batch);
    }
    for (let i = 0; i < list.length; i++) {
      const e = list[i];
      if (e.end <= clock) {
        this.spent.push(e);
        continue;
      }
      list[kept++] = e;
      if (clock < e.start) continue;
      const age = clock - e.start;
      switch (e.type) {
        case TRACER:
          this.drawTracer(e, age, batch);
          break;
        case FLASH:
          this.drawFlash(e, age, batch, muzzles);
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
        case PUFF:
          this.drawPuff(e, age, batch);
          break;
        case FLAME:
          this.drawFlame(e, age, batch);
          break;
      }
    }
    list.length = kept;
    for (let i = 0; i < this.sources.length; i++) this.drawFireLight(this.sources[i], clock, batch);
    this.instances = batch.count;
    this.dropped = batch.dropped;
    return batch;
  }

  stats(): EffectStats {
    return {
      live: this.effects.length,
      instances: this.instances,
      dropped: this.dropped,
      sources: this.sources.reduce(
        (n, s) => n + (this.noted < s.start + sourceLifetime(s.style) ? 1 : 0),
        0,
      ),
      tick: this.lastTick,
    };
  }

  // ---- Creation (per publication). ----

  private push(type: number, start: number, end: number): Effect {
    const e = this.spent.pop() ?? new Effect();
    e.path.length = 0;
    e.cum.length = 0;
    e.sparks.length = 0;
    e.length = 0;
    e.size = 0;
    e.rotation = 0;
    e.scale = 1;
    e.alpha = 1;
    e.frame = 0;
    e.tracer = null;
    e.flash = null;
    e.impact = null;
    e.puff = null;
    e.trail = null;
    e.flame = null;
    e.type = type;
    e.start = start;
    e.end = end;
    e.span = end - start;
    this.effects.push(e);
    return e;
  }

  private addTracer(
    t0: number,
    s: EffectSegment,
    rng: ReturnType<typeof mulberry32.create>,
  ): Effect {
    const style = pick(this.p.tracers, s.kind);
    const e = this.push(TRACER, t0, t0);
    e.tracer = style;
    e.cause = `round:${s.kind}`;
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
    // The tail: where the round was `tail_s` ago, at this stretch's speed.
    const [least, most] = style.tail_m;
    e.size = Math.min(most, Math.max(least, (total / this.dt) * style.tail_s));
    // Until the streak's tail has left the stretch's end.
    e.end =
      total > 1e-6
        ? t0 + Math.min(this.dt * (1 + e.size / total), this.dt + TRACER_TAIL_MAX_S)
        : t0;
    if (style.smoke && total > 1e-6) this.addTrail(t0, e, style.smoke, rng);
    return e;
  }

  /** A smoke trail along the stretch `e` flies: its ribbon, born as the
   *  round passes along it, and a puff every `spacing_m`. The puffs sit on a
   *  lattice along the stretch's heading (where the distance travelled that
   *  way is a whole number of spacings), so one tick's puffs run on evenly
   *  from the last one's without knowing which round it is. */
  private addTrail(
    t0: number,
    e: Effect,
    style: TrailStyle,
    rng: ReturnType<typeof mulberry32.create>,
  ) {
    const ribbon = this.push(TRAIL, t0, t0 + this.dt + style.life_s);
    ribbon.trail = style;
    ribbon.path.push(...e.path);
    ribbon.cum.push(...e.cum);
    ribbon.length = e.length;
    const path = e.path;
    const spacing = style.spacing_m;
    const first = Math.max(e.cum[1], 1e-6);
    const along =
      (path[0] * (path[3] - path[0]) +
        path[1] * (path[4] - path[1]) +
        path[2] * (path[5] - path[2])) /
      first;
    const past = ((along % spacing) + spacing) % spacing;
    for (let at = (spacing - past) % spacing; at < e.length; at += spacing) {
      const p = pointAlong(e, at, _note_at);
      this.addPuff(t0 + (this.dt * at) / e.length, p[0], p[1], p[2], style, 1, 1, rng);
    }
  }

  private addFlash(t0: number, l: Launch, rng: ReturnType<typeof mulberry32.create>) {
    const style = pick(this.p.flashes, l.kind);
    const last = Math.max(
      style.duration_s,
      style.fireball_m > 0 ? style.fireball_s : 0,
      style.cast?.duration_s ?? 0,
    );
    const e = this.push(FLASH, t0, t0 + last);
    e.flash = style;
    e.cause = `flash:${l.kind}`;
    e.shooter = l.shooter;
    e.mount = l.mount;
    e.soldier = l.soldier;
    vec3.set(e.p, l.x, l.y, l.z);
    vec3.set(e.n, l.dx, l.dy, l.dz);
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
    e.cause = `impact:${hit}`;
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
    // Dirt thrown up in a column, then smoke rising out of it; a bigger
    // burst's by the square root of its fireball's size over the least.
    const scale = Math.sqrt(e.size / style.min_size_m);
    for (const burst of [style.plume, style.smoke])
      for (let k = 0; k < burst.count; k++) {
        const r = e.size * 0.3 * Math.sqrt(mulberry32.sample(rng));
        const a = mulberry32.sample(rng) * Math.PI * 2;
        this.addPuff(
          at + burst.over_s * (k / Math.max(1, burst.count - 1)),
          b.point[0] + Math.cos(a) * r,
          b.point[1] + Math.sin(a) * r,
          b.point[2] + e.size * 0.2,
          burst,
          scale,
          1,
          rng,
        );
      }
  }

  /** A puff born at `at` at (x, y, z): its own drift, spin and first frame. */
  private addPuff(
    at: number,
    x: number,
    y: number,
    z: number,
    style: PuffStyle,
    scale: number,
    alpha: number,
    rng: ReturnType<typeof mulberry32.create>,
  ) {
    const e = this.push(PUFF, at, at + style.life_s);
    e.puff = style;
    e.scale = scale;
    e.alpha = alpha;
    vec3.set(e.p, x, y, z);
    const a = mulberry32.sample(rng) * Math.PI * 2;
    const spread = style.spread_mps * scale * mulberry32.sample(rng);
    const rise = style.rise_mps * scale * (0.7 + 0.6 * mulberry32.sample(rng));
    vec3.set(e.n, Math.cos(a) * spread, Math.sin(a) * spread, rise);
    e.rotation = mulberry32.sample(rng) * Math.PI * 2;
    e.size = mulberry32.sample(rng) - 0.5; // its spin
    e.frame = mulberry32.sample(rng) * (LAYER_FRAMES[LAYER.dust] - PUFF_TURN_FRAMES - 1);
  }

  /** A flame born at `at`, at `strength` of its style (a fire dying down). */
  private addFlame(
    at: number,
    x: number,
    y: number,
    z: number,
    style: FlameStyle,
    strength: number,
    rng: ReturnType<typeof mulberry32.create>,
  ) {
    const e = this.push(FLAME, at, at + style.life_s);
    e.flame = style;
    e.scale = strength * (0.7 + 0.6 * mulberry32.sample(rng));
    vec3.set(e.p, x, y, z);
    e.rotation = mulberry32.sample(rng) * Math.PI * 2;
    e.frame = 3 + mulberry32.sample(rng) * 3;
  }

  // ---- Drawing (per frame, allocation-free). ----

  /** A round in flight: its glow along the tail, its hot core along the
   *  tail's front, and its body (while this stretch holds the round). */
  private drawTracer(e: Effect, age: number, batch: EffectBatch) {
    const style = e.tracer!;
    const L = e.length;
    if (L <= 1e-6) return;
    const head = (L * age) / e.span;
    this.drawTail(e, head, e.size, style.glow, style.glow.min_px, true, batch);
    if (style.core)
      this.drawTail(e, head, e.size * style.core.share, style.core, this.p.min_px, false, batch);
    // The round is on this stretch from its tick's start to its end; the
    // next tick's stretch takes it on from there.
    // (A clock at the tick's end, as when paused, lands a rounding off it.)
    if ((!style.body && !style.cast) || age <= 0 || head > L * (1 + 1e-6)) return;
    pointAlong(e, Math.min(head, L), _build_a);
    if (style.body)
      disc(batch, _build_a, style.body.size_m, this.p.min_px * 2, style.body.color, 1);
    const c = style.cast;
    if (c) {
      const a = _build_a;
      offerCastLight(batch.lights, a[0], a[1], a[2], c.radius_m, c.color, c.intensity, e.cause);
    }
  }

  /** The part of `e`'s stretch within `length` behind `head`, in `light`,
   *  as streaks fading out toward the tail's end. */
  private drawTail(
    e: Effect,
    head: number,
    length: number,
    light: TracerLight,
    minPx: number,
    soft: boolean,
    batch: EffectBatch,
  ) {
    const tail = head - length;
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
        light.width_m,
        minPx,
        light.color,
        light.intensity,
        (lo - tail) / length,
        (hi - tail) / length,
        soft,
      );
    }
  }

  private drawFlash(e: Effect, age: number, batch: EffectBatch, muzzles: MuzzleSource | null) {
    const s = e.flash!;
    // On the drawn muzzle, pointing where the round went; the published
    // launch point where nothing of the shooter is drawn.
    const at = muzzles?.muzzle(e.shooter, e.mount, e.soldier, _flash_at) ? _flash_at : e.p;
    const c = s.cast;
    if (c && age < c.duration_s) {
      const f = age / c.duration_s;
      const a = vec3.scaleAndAdd(_build_a, at, e.n, c.forward_m ?? 0);
      const k = c.intensity * (1 - f * f);
      offerCastLight(batch.lights, a[0], a[1], a[2], c.radius_m, c.color, k, e.cause);
    }
    const x = age / s.duration_s;
    if (x < 1) {
      const k = (1 - x) * (1 - x);
      glow(
        batch,
        at,
        s.size_m * (0.7 + 0.5 * x),
        this.p.min_px,
        s.color,
        s.intensity * k,
        e.rotation,
        0,
      );
      const reach = s.tongue_m * (0.6 + 0.4 * x);
      vec3.scaleAndAdd(_build_b, at, e.n, reach);
      streak(
        batch,
        at[0],
        at[1],
        at[2],
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
      vec3.scaleAndAdd(_build_a, at, e.n, s.fireball_m * (0.2 + 0.6 * f));
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
    const c = s.cast;
    if (c && age < c.duration_s) {
      // Off the face, reaching farther for a bigger round.
      const f = age / c.duration_s;
      const reach = Math.sqrt(e.size / s.size_m);
      const a = vec3.scaleAndAdd(_build_a, e.p, e.n, 0.5);
      const k = c.intensity * (1 - f * f);
      offerCastLight(batch.lights, a[0], a[1], a[2], c.radius_m * reach, c.color, k, e.cause);
    }
    if (s.flash > 0 && x < 0.12) {
      const k = 1 - x / 0.12;
      // Off the face by its own radius: a sprite in the face would fade into it.
      const r = e.size * 0.35;
      vec3.scaleAndAdd(_build_a, e.p, e.n, r);
      glow(batch, _build_a, r, this.p.min_px, this.p.sparks.color, s.flash * k * k, e.rotation);
    }
  }

  private drawSparks(e: Effect, age: number, batch: EffectBatch, color: Vec3, intensity: number) {
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

  /** Where smoke born at `p`, moving at `n` (its own rise and spread, m/s),
   *  is `age` seconds into a life of `life`: it rises, slowing as it spreads,
   *  and drifts on its own and more downwind as it climbs. */
  private smokeAt(p: Vec3, n: Vec3, life: number, age: number, out: Vec3): Vec3 {
    const x = age / life;
    const [w0, w1] = this.p.wind_mps;
    const climb = life * 0.5 * (1 - (1 - x) * (1 - x));
    const drift = age * (0.3 + 0.7 * x);
    out[0] = p[0] + n[0] * climb + w0 * drift;
    out[1] = p[1] + n[1] * climb + w1 * drift;
    out[2] = p[2] + n[2] * climb;
    return out;
  }

  /** A trail's ribbon along the part of its stretch the round has flown:
   *  each end where its smoke has drifted since the round passed it, as wide
   *  and faded as its middle's age. Stretches meet end to end, each end born
   *  when the next stretch's start was, so the ribbon runs unbroken. */
  private drawTrail(e: Effect, age: number, batch: EffectBatch) {
    const s = e.trail!;
    const L = e.length;
    if (L <= 1e-6) return;
    const flown = Math.min(L, (L * age) / this.dt);
    vec3.set(_trail_rise, 0, 0, s.rise_mps);
    for (let i = 0; i + 1 < e.cum.length; i++) {
      const lo = e.cum[i];
      const hi = Math.min(e.cum[i + 1], flown);
      if (hi <= lo) continue;
      // Each point's age: since the round passed it.
      const perM = this.dt / L;
      const ageLo = age - lo * perM;
      const ageHi = Math.max(0, age - hi * perM);
      this.smokeAt(pointAlong(e, lo, _trail_p), _trail_rise, s.life_s, ageLo, _build_a);
      this.smokeAt(pointAlong(e, hi, _trail_p), _trail_rise, s.life_s, ageHi, _build_b);
      const x = Math.min(1, (ageLo + ageHi) / 2 / s.life_s);
      ribbon(
        batch,
        _build_a,
        _build_b,
        2 * RIBBON_WIDTH * puffRadius(s, x),
        this.p.min_px,
        s.albedo,
        // Thinning before its puffs fade, so an old trail loosens into them.
        s.ribbon * (1 - x) ** 2,
      );
    }
  }

  /** A puff: it rises, slowing as it spreads; drifts on its own and more
   *  downwind as it climbs; fades in quickly and out slowly. */
  private drawPuff(e: Effect, age: number, batch: EffectBatch) {
    const s = e.puff!;
    const x = age / e.span;
    this.smokeAt(e.p, e.n, s.life_s, age, _build_a);
    const radius = e.scale * puffRadius(s, x);
    const fade = Math.min(1, age * PUFF_FADE_IN) * (1 - x) ** 1.2;
    flipbook(
      batch,
      _build_a,
      radius,
      e.rotation + e.size * x * 1.5,
      LAYER.dust,
      e.frame + x * PUFF_TURN_FRAMES,
      s.albedo,
      s.opacity * e.alpha * fade,
      0,
      radius * 0.8,
      true,
    );
  }

  /** A flame: the fire sheet's hot frames, licking up and shrinking away. */
  private drawFlame(e: Effect, age: number, batch: EffectBatch) {
    const s = e.flame!;
    const x = age / e.span;
    vec3.copy(_build_a, e.p);
    _build_a[2] += s.size_m * e.scale * (0.3 + 1.2 * x);
    flipbook(
      batch,
      _build_a,
      s.size_m * e.scale * (0.8 + 0.4 * Math.sin(Math.PI * x)),
      e.rotation,
      LAYER.fire,
      e.frame + x * 4,
      s.tint,
      Math.min(1, x * 6) * (1 - x),
      s.emissive,
      0.3,
    );
  }

  /** A burning source's fire light (its glow in the air and its cast
   *  light), flickering on the clock, dying down
   *  over the burn's last third; it lasts no longer than the flames made
   *  from the publications taken so far. */
  private drawFireLight(src: Source, clock: number, batch: EffectBatch) {
    const f = src.style.flame;
    const age = clock - src.start;
    const burn = src.style.burn_s;
    if (age < 0 || age >= burn || clock > this.noted + f.life_s || f.light_intensity <= 0) return;
    if (src.seed / 2 ** 32 >= this.keep) return; // over the budget
    const phase = (src.seed % 1000) * 0.01;
    const flicker =
      0.7 + 0.3 * Math.sin(clock * 11.3 + phase) * Math.sin(clock * 5.7 + phase * 1.7);
    const strength = Math.min(1, ((burn - age) / burn) * 3, age * 4);
    vec3.set(_build_b, src.x, src.y, src.top + 0.5);
    glow(batch, _build_b, f.light_m, 0, f.light, f.light_intensity * flicker * strength, phase, 0);
    const c = f.cast;
    if (c) {
      const k = c.intensity * flicker * strength;
      offerCastLight(batch.lights, src.x, src.y, src.top + 0.5, c.radius_m, c.color, k, src.cause);
    }
  }

  private drawBlast(e: Effect, age: number, batch: EffectBatch) {
    const s = this.p.blast;
    const x = age / e.span;
    // The fireball: the flipbook's whole life over the blast's, rising.
    vec3.copy(_build_a, e.p);
    _build_a[2] += e.size * (0.55 + 0.5 * x);
    const heat = Math.max(0, 1 - x * 2.2);
    const c = s.cast;
    if (age < c.duration_s) {
      const f = age / c.duration_s;
      const reach = Math.sqrt(e.size / s.min_size_m);
      const k = c.intensity * (1 - f * f);
      const z = e.p[2] + e.size * 0.5;
      offerCastLight(batch.lights, e.p[0], e.p[1], z, c.radius_m * reach, c.color, k, "blast");
    }
    // The glow in the air round the burst, fading as the fire cools.
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
