// @vitest-environment node
import { expect, test } from "vitest";
import { mat4, vec3, type Vec3 } from "math";
import {
  flightSeconds,
  hullMotion,
  impactAfter,
  landedAfter,
  turretMotion,
  validateCookOff,
  type CookOffFeel,
} from "@packages/battle-renderer/src/effects/cookOff";

const FEEL: CookOffFeel = validateCookOff({
  delay_s: 0.3,
  rise_m: 6,
  flip_turns: 1,
  spin_turns: 0.6,
  fireballs: [{ after_s: 0, up_m: 0.5, radius_m: 5, sparks: 1 }],
  bounce_m: 0.4,
  bounce_tilt_deg: 6,
  heave_m: 0.25,
  jolt_m: 0.08,
  rock_deg: 2,
  settle_s: 0.8,
  landing_dust: 1.5,
  landing_sparks: 8,
});
/** Where the thrown turret lies on the wreck: aft of and beside its ring. */
const LIES: Vec3 = [-0.3, 0.45, 2.1];

function moved(age: number, p: Vec3 = LIES): Vec3 {
  const m = turretMotion(mat4.create(), FEEL, age, LIES, 17);
  return vec3.transformMat4(vec3.create(), p, m);
}

test("the turret leaves from its ring, rises as high as thrown, and lands where the wreck has it", () => {
  const flight = flightSeconds(FEEL);
  // A body thrown `rise_m` up falls back in √(8h/g).
  expect(flight).toBeCloseTo(Math.sqrt((8 * 6) / 9.81), 6);
  // Until the ammunition goes, and as it does, the turret sits over the ring.
  for (const age of [0, FEEL.delay_s]) {
    const at = moved(age);
    expect([at[0], at[1]]).toEqual([expect.closeTo(0, 6), expect.closeTo(0, 6)]);
    expect(at[2]).toBeCloseTo(LIES[2], 6);
  }
  // At the top of its arc it is the throw's height above where it lands.
  expect(moved(FEEL.delay_s + flight / 2)[2]).toBeCloseTo(LIES[2] + FEEL.rise_m, 6);
  // It strikes the deck where the wreck has it, kicks back up off it as high
  // as it bounces, and then lies exactly as the wreck's: the whole takes over.
  expect(impactAfter(FEEL)).toBeCloseTo(FEEL.delay_s + flight, 6);
  expect(moved(impactAfter(FEEL))[2]).toBeCloseTo(LIES[2], 6);
  // A hop as high as it bounces, then lying still from the hull's settling on.
  const hop = Math.sqrt((8 * FEEL.bounce_m) / 9.81);
  expect(landedAfter(FEEL)).toBeGreaterThanOrEqual(impactAfter(FEEL) + hop);
  expect(moved(impactAfter(FEEL) + hop + 0.01)[2]).toBeCloseTo(LIES[2], 6);
  expect(moved(impactAfter(FEEL) + hop / 2)[2]).toBeCloseTo(LIES[2] + FEEL.bounce_m, 6);
  for (const age of [landedAfter(FEEL), 60]) {
    const m = turretMotion(mat4.create(), FEEL, age, LIES, 17);
    expect(Array.from(m)).toEqual(Array.from(mat4.create()).map((v) => expect.closeTo(v, 6)));
  }
});

test("in flight the turret tumbles as one rigid body", () => {
  const flight = flightSeconds(FEEL);
  const a: Vec3 = [LIES[0] + 2, LIES[1], LIES[2]];
  const b: Vec3 = [LIES[0], LIES[1] - 1, LIES[2] + 0.5];
  for (const u of [0.2, 0.5, 0.8]) {
    const age = FEEL.delay_s + flight * u;
    expect(vec3.distance(moved(age, a), moved(age, b))).toBeCloseTo(vec3.distance(a, b), 5);
    // and turns: a point off its centre is not where a mere lift would put it
    const lifted = vec3.sub(vec3.create(), moved(age, a), moved(age));
    expect(vec3.distance(lifted, [2, 0, 0])).toBeGreaterThan(0.1);
  }
});

test("a throw that ends below its start, or a fireball with no size, is refused", () => {
  expect(() => validateCookOff({ ...FEEL, rise_m: 0 })).toThrow(/rise_m/);
  expect(() =>
    validateCookOff({ ...FEEL, fireballs: [{ after_s: 0, up_m: 0, radius_m: 0, sparks: 0 }] }),
  ).toThrow(/radius_m/);
});

test("the hull heaves as the ammunition goes, dips as the turret strikes it, and settles as the wreck", () => {
  const at = (age: number) => {
    const m = hullMotion(mat4.create(), FEEL, age, 17);
    return vec3.transformMat4(vec3.create(), [0, 0, 1], m)[2] - 1;
  };
  // At rest before the ammunition goes.
  expect(at(FEEL.delay_s * 0.5)).toBeCloseTo(0, 6);
  // Thrown up off its suspension by the blast, a quarter swing in.
  expect(at(FEEL.delay_s + FEEL.settle_s / 8)).toBeGreaterThan(FEEL.heave_m * 0.5);
  // Pressed down as the turret lands on it.
  expect(at(impactAfter(FEEL) + FEEL.settle_s / 8)).toBeLessThan(-FEEL.jolt_m * 0.5);
  // Once the turret has landed for good the hull is exactly the wreck's.
  for (const age of [landedAfter(FEEL), 60]) {
    const m = hullMotion(mat4.create(), FEEL, age, 17);
    expect(Array.from(m)).toEqual(Array.from(mat4.create()).map((v) => expect.closeTo(v, 6)));
  }
});
