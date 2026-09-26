// @vitest-environment node
// Combat effects at their seam (slice 25): publications in, the effect
// pass's instances out at a presentation clock. Every effect has a published
// cause, a publication is taken once, a tracer never leaves its published
// stretch, and the village's round kinds each look their own.
import { expect, test } from "vitest";
import village from "@fixtures/village.json";
import {
  createEffectBatch,
  EFFECT_FLOATS,
  EffectFrame,
  SHAPE,
  type EffectBatch,
  type EffectPresentation,
  type EffectPublication,
  type EffectSegment,
  type EffectShooter,
} from "@packages/battle-renderer/src/effects/effectFrame";

const HZ = 30;
const DT = 1 / HZ;
const PRESENTATION = village.presentation.effects as unknown as EffectPresentation;
const MUZZLE = village.physics.tank_muzzle_local_m;

const frame = () =>
  new EffectFrame({ tickHz: HZ, presentation: PRESENTATION, vehicleMuzzle: MUZZLE });

const pub = (tick: number, p: Partial<EffectPublication> = {}): EffectPublication => ({
  tick,
  segments: [],
  blasts: [],
  shooters: [],
  ...p,
});

const segment = (path: [number, number, number][], s: Partial<EffectSegment> = {}) => ({
  path,
  ricochets: [],
  kind: "rifle",
  shooter: null,
  hit: "none",
  normal: null,
  ...s,
});

const squad = (key: number, members: number[], shots: number): EffectShooter => ({
  key,
  vehicle: false,
  position: [0, 0, 0],
  members,
  mounts: [{ bearing: 0, elevation: 0, shots, kind: "rifle" }],
});

const tank = (key: number, shots: number): EffectShooter => ({
  key,
  vehicle: true,
  position: [100, 50, 0],
  members: [],
  mounts: [{ bearing: Math.PI / 2, elevation: 0, shots, kind: "tank_ap" }],
});

/** The instances drawn at `clock`, one record each. */
function drawn(f: EffectFrame, clock: number) {
  const batch: EffectBatch = f.build(clock, createEffectBatch(PRESENTATION.capacity));
  const out: { shape: number; a: number[]; b: number[]; color: number[] }[] = [];
  for (let i = 0; i < batch.count; i++) {
    const r = Array.from(batch.data.subarray(i * EFFECT_FLOATS, (i + 1) * EFFECT_FLOATS));
    out.push({ shape: r[12], a: r.slice(0, 4), b: r.slice(4, 8), color: r.slice(8, 12) });
  }
  return out;
}

test("no effect without a published cause", () => {
  const f = frame();
  // Quiet publications: units seen, counters steady, nothing flying.
  f.note(pub(1, { shooters: [squad(1, [10, 11], 4), tank(2, 3)] }));
  f.note(pub(2, { shooters: [squad(1, [10, 11], 4), tank(2, 3)] }));
  for (const t of [0, 1.5 * DT, 2 * DT, 5]) expect(drawn(f, t)).toEqual([]);

  // A unit first seen with rounds already counted shows no shot.
  const g = frame();
  g.note(pub(5, { shooters: [tank(2, 7)] }));
  expect(drawn(g, 4.5 * DT)).toEqual([]);
  // Its counter rising is a shot: a flash at the hull's muzzle, on its bearing.
  g.note(pub(6, { shooters: [tank(2, 8)] }));
  const flash = drawn(g, 5.01 * DT).filter((i) => i.shape === SHAPE.glow);
  expect(flash).toHaveLength(1);
  const [fwd, left, up] = MUZZLE;
  expect(flash[0].a[0]).toBeCloseTo(100 - left, 5);
  expect(flash[0].a[1]).toBeCloseTo(50 + fwd, 5);
  expect(flash[0].a[2]).toBeCloseTo(up, 5);
});

test("a squad's flash is on the soldier who started a new round, never a continuing one", () => {
  const f = frame();
  const muzzle: [number, number, number] = [3, 4, 1.5];
  const first = segment([muzzle, [31, 4, 1.5]], { shooter: 11 });
  f.note(pub(1, { shooters: [squad(1, [10, 11], 0)] }));
  f.note(pub(2, { segments: [first], shooters: [squad(1, [10, 11], 1)] }));
  const flashes = (t: number) => drawn(f, t).filter((i) => i.shape === SHAPE.glow);
  expect(flashes(1.01 * DT).map((i) => i.a.slice(0, 3))).toEqual([muzzle]);
  // The next tick the round flies on from where it was, and another squad
  // soldier's counter rise names no new round of his: no flash.
  const on = segment(
    [
      [31, 4, 1.5],
      [59, 4, 1.5],
    ],
    { shooter: 11 },
  );
  f.note(pub(3, { segments: [on], shooters: [squad(1, [10, 11], 2)] }));
  // (The first shot's flash is still fading at the muzzle.)
  expect(flashes(2.01 * DT).map((i) => i.a.slice(0, 3))).toEqual([muzzle]);
});

test("a publication is taken once", () => {
  const busy = pub(2, {
    segments: [
      segment(
        [
          [0, 0, 1],
          [28, 0, 1],
        ],
        { hit: "ground", normal: [0, 0, 1] },
      ),
    ],
    blasts: [{ point: [28, 0, 0], radius: 6, kind: "grenade" }],
  });
  const once = frame();
  once.note(pub(1));
  once.note(busy);
  const twice = frame();
  twice.note(pub(1));
  twice.note(busy);
  twice.note(busy);
  twice.note({ ...busy });
  for (const t of [1.5 * DT, 2 * DT, 2.5 * DT, 0.5])
    expect(drawn(twice, t)).toEqual(drawn(once, t));
  // An earlier tick is a new battle: everything before it is forgotten.
  twice.note(pub(1));
  expect(drawn(twice, 2 * DT)).toEqual([]);
});

test("a tracer never leaves its published stretch, corners included", () => {
  // An enemy stretch clipped to seen ground, glancing once at its corner.
  const path: [number, number, number][] = [
    [10, 0, 1],
    [20, 0, 1],
    [26, 8, 3],
  ];
  const f = frame();
  f.note(pub(1));
  f.note(
    pub(2, {
      segments: [segment(path, { kind: "hmg", ricochets: [{ point: 1, normal: [-1, 0, 0] }] })],
    }),
  );
  const onPath = (p: number[]) =>
    [0, 1].some((k) => {
      const [a, b] = [path[k], path[k + 1]];
      const ab = b.map((v, i) => v - a[i]);
      const len2 = ab.reduce((s, v) => s + v * v, 0);
      const u = ab.reduce((s, v, i) => s + v * (p[i] - a[i]), 0) / len2;
      const q = a.map((v, i) => v + ab[i] * u);
      return u >= -1e-4 && u <= 1 + 1e-4 && Math.hypot(...q.map((v, i) => v - p[i])) < 1e-3;
    });
  let streaks = 0;
  for (let k = 0; k <= 40; k++) {
    const t = DT + (k / 20) * DT;
    const style = PRESENTATION.tracers.hmg;
    for (const i of drawn(f, t)) {
      // Tracer streaks are the ones in the round kind's colour.
      if (i.shape !== SHAPE.streak || i.color[0] !== style.color[0] * style.intensity) continue;
      streaks++;
      expect(onPath(i.a)).toBe(true);
      expect(onPath(i.b)).toBe(true);
    }
  }
  expect(streaks).toBeGreaterThan(10);
});

test("the village's round kinds each have a tracer and flash of their own", () => {
  const kinds = Object.keys(village.weapons);
  const look = (kind: string) =>
    JSON.stringify([PRESENTATION.tracers[kind], PRESENTATION.flashes[kind]]);
  for (const kind of kinds) {
    expect(PRESENTATION.tracers[kind], kind).toBeDefined();
    expect(PRESENTATION.flashes[kind], kind).toBeDefined();
  }
  // Rounds of different weapons differ in colour, size or length, except the
  // one cannon's two ammunition kinds, whose flash is the same gun's.
  const tracerLooks = new Set(kinds.map((k) => JSON.stringify(PRESENTATION.tracers[k])));
  expect(tracerLooks.size).toBe(kinds.length);
  expect(new Set(kinds.map(look)).size).toBe(kinds.length);
});
