// A cook-off: a hull the side watched die brews up. A beat after the killing
// hit its ammunition goes: fireballs roar out of the turret ring, the turret
// is thrown into the air, tumbling, and falls back onto the deck where the
// wreck has it, in a slam of sparks and dust, hops once and settles. The hull
// heaves on its suspension as the ammunition goes and dips as the turret
// strikes it. Both are the wreck's own pieces (the simulation's prop, burnt
// from the tick it died), so what lands is the wreck's, and the whole wreck
// takes over from the pieces without a seam.
//
// The flight is a thrown body's: a parabola `rise_m` high, falling back in
// √(8h/g), with a steady tumble (whole turns over, so it leaves the ring
// upright) and spin; the hop is another, `bounce_m` high, rocking. The
// hull's jolts swing twice and die away within `settle_s`. Presentation
// only, like every effect: the simulation has no turret. The numbers are
// `presentation.effects.cook_off`; the effects take the fireballs, sparks
// and dust (`EffectFrame`), the battle draws the pieces with `turretMotion`
// and `hullMotion`.
//
// What the blast throws clear (armour packs, doors, track runs: the wreck's
// `debris` state) lies round the wreck as it lands, wider than the box the
// simulation keeps as cover. So it does not stay: after `debris.hold_s` it
// sinks into the ground over `debris.fade_s`, as a corpse does, and is gone
// (`debrisSink`). The wreck itself never fades.
import { mat4, quat, type Mat4, type Quat, type Vec3 } from "math";
import { mulberry32 } from "math/random";

/** Gravity, m/s²: the thrown turret falls under it, and every spark. */
export const GRAVITY = 9.81;

/** `presentation.effects.cook_off`. */
export interface CookOffFeel {
  /** Seconds from the killing hit to the ammunition going. */
  delay_s: number;
  /** How high the turret is thrown, metres. */
  rise_m: number;
  /** Whole turns the turret tumbles end over end in flight. */
  flip_turns: number;
  /** Turns it spins about its own upright in flight. */
  spin_turns: number;
  /** How high it hops off the deck after it strikes it, metres, and how
   *  far it rocks as it does, degrees. */
  bounce_m: number;
  bounce_tilt_deg: number;
  /** How high the hull heaves as the ammunition goes, and how far it dips
   *  as the turret strikes it, metres; how far it rocks with each, degrees;
   *  and the seconds each jolt takes to die away. */
  heave_m: number;
  jolt_m: number;
  rock_deg: number;
  settle_s: number;
  /** The fireballs out of the turret ring, each `after_s` after the
   *  ammunition goes, `up_m` above the hull's top, of blast radius
   *  `radius_m`, throwing the share `sparks` of a blast's sparks. */
  fireballs: readonly { after_s: number; up_m: number; radius_m: number; sparks: number }[];
  /** The dust the landing turret throws up, as a blast's plume of this scale. */
  landing_dust: number;
  /** Sparks off the deck as the turret lands. */
  landing_sparks: number;
  /** The thrown debris: seconds it lies still from the blast, then seconds
   *  it takes to sink out of sight. */
  debris: { hold_s: number; fade_s: number };
}

export function validateCookOff(f: CookOffFeel): CookOffFeel {
  const fail = (path: string, why: string): never => {
    throw new Error(`presentation.effects.cook_off.${path}: ${why}`);
  };
  if (!(f.delay_s >= 0)) fail("delay_s", "must be ≥ 0");
  if (!(f.rise_m > 0)) fail("rise_m", "must be positive");
  if (!(Number.isInteger(f.flip_turns) && f.flip_turns >= 0))
    fail("flip_turns", "must be a whole number ≥ 0, so the turret leaves its ring upright");
  if (!Number.isFinite(f.spin_turns)) fail("spin_turns", "must be a number");
  for (const k of ["bounce_m", "bounce_tilt_deg", "heave_m", "jolt_m", "rock_deg"] as const)
    if (!(f[k] >= 0)) fail(k, "must be ≥ 0");
  if (!(f.settle_s > 0)) fail("settle_s", "must be positive");
  if (!f.fireballs?.length) fail("fireballs", "needs at least one");
  f.fireballs.forEach((b, i) => {
    if (!(b.after_s >= 0)) fail(`fireballs[${i}].after_s`, "must be ≥ 0");
    if (!(b.radius_m > 0)) fail(`fireballs[${i}].radius_m`, "must be positive");
    if (!(b.sparks >= 0 && b.sparks <= 1)) fail(`fireballs[${i}].sparks`, "must be in [0, 1]");
  });
  if (!(f.landing_dust >= 0)) fail("landing_dust", "must be ≥ 0");
  if (!(Number.isInteger(f.landing_sparks) && f.landing_sparks >= 0))
    fail("landing_sparks", "must be a whole number ≥ 0");
  if (!(f.debris?.hold_s >= 0)) fail("debris.hold_s", "must be ≥ 0");
  if (!(f.debris.fade_s > 0))
    fail("debris.fade_s", "must be positive, so debris sinks, never pops");
  return f;
}

/** Seconds a body thrown `rise` metres up is in the air. */
function airborne(rise: number): number {
  return Math.sqrt((8 * rise) / GRAVITY);
}

/** Seconds the thrown turret is in the air. */
export function flightSeconds(f: CookOffFeel): number {
  return airborne(f.rise_m);
}

/** Seconds after the killing hit that the turret strikes the deck. */
export function impactAfter(f: CookOffFeel): number {
  return f.delay_s + flightSeconds(f);
}

/** Seconds after the killing hit that every piece lies still as the wreck has it. */
export function landedAfter(f: CookOffFeel): number {
  return impactAfter(f) + Math.max(airborne(f.bounce_m), f.settle_s);
}

/** Seconds after the killing hit that the thrown debris is gone. */
export function debrisGoneAfter(f: CookOffFeel): number {
  return f.delay_s + f.debris.hold_s + f.debris.fade_s;
}

/**
 * How far the thrown debris has sunk into the ground `age` seconds after the
 * killing hit, metres, or null when it is not drawn: before the ammunition
 * throws it, and once it is gone. It lies still (0) through its hold, then
 * sinks, easing in, until its top (`top`, metres above the ground) is under
 * the ground. The one owner of the debris's fade; the wreck never fades.
 */
export function debrisSink(f: CookOffFeel, age: number, top: number): number | null {
  if (!(age >= f.delay_s && age < debrisGoneAfter(f))) return null;
  const u = Math.max(0, age - f.delay_s - f.debris.hold_s) / f.debris.fade_s;
  return top * u * u;
}

/** Height on a hop's parabola `h` high, at `u` of its way (0 outside it). */
function hop(h: number, u: number): number {
  return u > 0 && u < 1 ? 4 * h * u * (1 - u) : 0;
}

/** A jolt `t` seconds in: two swings, the first toward +1, dying away to
 *  nothing at `settle` seconds (0 outside it). */
function jolt(t: number, settle: number): number {
  if (!(t > 0 && t < settle)) return 0;
  const u = t / settle;
  return Math.sin(4 * Math.PI * u) * (1 - u) * (1 - u);
}

const DEG = Math.PI / 180;
const _motion_q = quat.create();
const _motion_turn = quat.create();
const _motion_axis: Vec3 = [0, 0, 0];
const _motion_offset: Vec3 = [0, 0, 0];
const _motion_origin: Vec3 = [0, 0, 0];
const UP: Vec3 = [0, 0, 1];
const ALONG: Vec3 = [1, 0, 0];
const ACROSS: Vec3 = [0, 1, 0];

/** A level axis of the wreck's own, picked by `seed` (the wreck's). */
function levelAxis(seed: number): Vec3 {
  const across = mulberry32.sample(mulberry32.create(seed >>> 0)) * Math.PI * 2;
  _motion_axis[0] = Math.cos(across);
  _motion_axis[1] = Math.sin(across);
  _motion_axis[2] = 0;
  return _motion_axis;
}

/**
 * The thrown turret's rigid motion, `age` seconds after the killing hit, in
 * the wreck's own frame, into `out`: what moves the turret piece from where
 * the wreck has it (its centre `lies`) to where it is. Before the ammunition
 * goes it sits centred over the ring (the wreck's origin); once it has
 * struck the deck, hopped and settled, it is the identity. `seed` (the
 * wreck's) picks the way it tumbles.
 */
export function turretMotion(
  out: Mat4,
  f: CookOffFeel,
  age: number,
  lies: Vec3,
  seed: number,
): Mat4 {
  const s = Math.min(1, Math.max(0, (age - f.delay_s) / flightSeconds(f)));
  const left = 1 - s;
  const u = (age - impactAfter(f)) / airborne(f.bounce_m);
  // Along the ground at a steady pace from over the ring; up and back down
  // on the parabola, then the hop off the deck.
  _motion_offset[0] = -lies[0] * left;
  _motion_offset[1] = -lies[1] * left;
  _motion_offset[2] = hop(f.rise_m, s) + hop(f.bounce_m, u);
  // The tumble still to come about a level axis, with the hop's rock about
  // it, and the spin still to come about the upright.
  const tumble = 2 * Math.PI * f.flip_turns * left + f.bounce_tilt_deg * DEG * hop(1, u);
  quat.setAxisAngle(_motion_turn, levelAxis(seed), tumble);
  quat.setAxisAngle(_motion_q, UP, 2 * Math.PI * f.spin_turns * left);
  quat.multiply(_motion_q, _motion_q, _motion_turn as Quat);
  return mat4.fromRotationTranslationScaleOrigin(out, _motion_q, _motion_offset, [1, 1, 1], lies);
}

/**
 * The hull's rigid motion, `age` seconds after the killing hit, in the
 * wreck's own frame, into `out`: a heave up off its suspension as the
 * ammunition goes, rocking fore and aft, and a dip as the turret strikes it,
 * rolling; each dies away within `settle_s`, and the hull lies as the wreck
 * has it. It turns about its base centre; `seed` rolls it one way or the other.
 */
export function hullMotion(out: Mat4, f: CookOffFeel, age: number, seed: number): Mat4 {
  const blast = jolt(age - f.delay_s, f.settle_s);
  const strike = jolt(age - impactAfter(f), f.settle_s);
  _motion_offset[0] = 0;
  _motion_offset[1] = 0;
  _motion_offset[2] = f.heave_m * blast - f.jolt_m * strike;
  const side = seed & 1 ? 1 : -1;
  quat.setAxisAngle(_motion_q, ACROSS, f.rock_deg * DEG * blast);
  quat.setAxisAngle(_motion_turn, ALONG, side * f.rock_deg * DEG * strike);
  quat.multiply(_motion_q, _motion_q, _motion_turn as Quat);
  return mat4.fromRotationTranslationScaleOrigin(
    out,
    _motion_q,
    _motion_offset,
    [1, 1, 1],
    _motion_origin,
  );
}
