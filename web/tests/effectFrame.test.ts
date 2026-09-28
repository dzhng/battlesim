// @vitest-environment node
// Combat effects at their seam (slices 25 and 26): publications in, the
// effect pass's instances out at a presentation clock. Every effect has a
// published cause, a publication is taken once, a tracer never leaves its
// published stretch, and the village's round kinds each look their own. A
// known wreck burns, smoulders and goes out; a moving hull raises dust; every
// life is bounded; what is drawn does not hang on how publications arrive;
// reset clears everything.
import { expect, test } from "vitest";
import village from "@fixtures/village.json";
import {
  createEffectBatch,
  EFFECT_FLOATS,
  EffectFrame,
  LAYER,
  maxEffectLifetime,
  SHAPE,
  type EffectBatch,
  type EffectPresentation,
  type EffectPublication,
  type EffectSegment,
  type EffectShooter,
  type EffectSmokeSource,
  type MuzzleSource,
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
  smokes: [],
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
  position: [0, 0, 0],
  half: null,
  members,
  mounts: [{ bearing: 0, elevation: 0, shots, kind: "rifle" }],
});

const tank = (key: number, shots: number): EffectShooter => ({
  key,
  position: [100, 50, 0],
  half: [3.5, 1.8, 1.2],
  members: [],
  mounts: [{ bearing: Math.PI / 2, elevation: 0, shots, kind: "tank_ap" }],
});

/** The instances drawn at `clock`, one record each. */
function drawn(f: EffectFrame, clock: number) {
  const batch: EffectBatch = f.build(clock, createEffectBatch(PRESENTATION.capacity));
  const out: { shape: number; a: number[]; b: number[]; color: number[]; misc: number[] }[] = [];
  for (let i = 0; i < batch.count; i++) {
    const r = Array.from(batch.data.subarray(i * EFFECT_FLOATS, (i + 1) * EFFECT_FLOATS));
    out.push({
      shape: r[12],
      a: r.slice(0, 4),
      b: r.slice(4, 8),
      color: r.slice(8, 12),
      misc: r.slice(12, 16),
    });
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
  // The simulation flies a round first on the tick after the one that fired
  // it: soldier 11's counter rises at tick 2, his round starts at tick 3.
  const f = frame();
  const muzzle: [number, number, number] = [3, 4, 1.5];
  const first = segment([muzzle, [31, 4, 1.5]], { shooter: 11 });
  f.note(pub(1, { shooters: [squad(1, [10, 11], 0)] }));
  f.note(pub(2, { shooters: [squad(1, [10, 11], 1)] }));
  f.note(pub(3, { segments: [first], shooters: [squad(1, [10, 11], 1)] }));
  const flashes = (t: number) => drawn(f, t).filter((i) => i.shape === SHAPE.glow);
  expect(flashes(1.5 * DT)).toEqual([]);
  expect(flashes(2.01 * DT).map((i) => i.a.slice(0, 3))).toEqual([muzzle]);
  // The next tick the round flies on from where it was: no second flash.
  const on = segment(
    [
      [31, 4, 1.5],
      [59, 4, 1.5],
    ],
    { shooter: 11 },
  );
  f.note(pub(4, { segments: [on], shooters: [squad(1, [10, 11], 1)] }));
  // (The first shot's flash is still fading at the muzzle.)
  expect(flashes(3.01 * DT).map((i) => i.a.slice(0, 3))).toEqual([muzzle]);
  // A round starting with no rise the tick before is not a shot shown.
  const stray = segment(
    [
      [7, 8, 1.5],
      [35, 8, 1.5],
    ],
    { shooter: 10 },
  );
  f.note(pub(5, { segments: [stray], shooters: [squad(1, [10, 11], 1)] }));
  expect(flashes(4.01 * DT)).toEqual([]);
});

test("a flash sits on the muzzle as drawn at each frame, else where the round was launched", () => {
  const f = frame();
  f.note(pub(5, { shooters: [tank(2, 7)] }));
  f.note(pub(6, { shooters: [tank(2, 8)] }));
  const asked: [number, number, number | null][] = [];
  // The model has moved on and its gun recoiled: the drawn muzzle, per frame.
  let drawnAt: [number, number, number] | null = [99, 55.5, 1.8];
  const muzzles: MuzzleSource = {
    muzzle(shooter, mount, soldier, at) {
      asked.push([shooter, mount, soldier]);
      if (!drawnAt) return false;
      at[0] = drawnAt[0];
      at[1] = drawnAt[1];
      at[2] = drawnAt[2];
      return true;
    },
  };
  const at = (clock: number) => {
    const batch = f.build(clock, createEffectBatch(PRESENTATION.capacity), muzzles);
    const glows: number[][] = [];
    const tails: number[][] = [];
    for (let i = 0; i < batch.count; i++) {
      const r = Array.from(batch.data.subarray(i * EFFECT_FLOATS, (i + 1) * EFFECT_FLOATS));
      if (r[12] === SHAPE.glow) glows.push(r.slice(0, 3));
      if (r[12] === SHAPE.streak) tails.push(r.slice(0, 3));
    }
    return { glows, tails };
  };
  const one = at(5.01 * DT);
  expect(asked[0]).toEqual([2, 0, null]);
  expect(one.glows[0][0]).toBeCloseTo(99, 4);
  expect(one.glows[0][1]).toBeCloseTo(55.5, 4);
  expect(one.glows[0][2]).toBeCloseTo(1.8, 4);
  // The tongue starts there too.
  expect(one.tails[0][0]).toBeCloseTo(99, 4);
  expect(one.tails[0][1]).toBeCloseTo(55.5, 4);
  // A frame later the model has moved: the flash moves with it.
  drawnAt = [99.3, 55.5, 1.8];
  expect(at(5.5 * DT).glows[0][0]).toBeCloseTo(99.3, 4);
  // Nothing of the shooter drawn: the published launch point.
  drawnAt = null;
  const [fwd, left, up] = MUZZLE;
  const fallback = at(5.6 * DT).glows[0];
  expect(fallback[0]).toBeCloseTo(100 - left, 4);
  expect(fallback[1]).toBeCloseTo(50 + fwd, 4);
  expect(fallback[2]).toBeCloseTo(up, 4);
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

// ---- Slice 26: smoke, fire and dust. ----

const WRECK = PRESENTATION.smoke.wreck;
const LONGEST = maxEffectLifetime(PRESENTATION);

const wreck = (x: number, y: number): EffectSmokeSource => ({
  key: `wreck:${x},${y}`,
  kind: "wreck",
  center: [x, y, 0],
  yaw: 0.4,
  half: [3.5, 1.8, 1.2],
});

type Drawn = ReturnType<typeof drawn>;
/** Fire: the fire sheet's sprites. Smoke and dust: sun-lit sprites. */
const flames = (d: Drawn) => d.filter((i) => i.shape === SHAPE.flipbook && i.b[2] === LAYER.fire);
const puffs = (d: Drawn) => d.filter((i) => i.shape === SHAPE.flipbook && i.misc[2] === 1);
/** Instances in a stable order, for comparing two frames' sets. */
const sorted = (d: Drawn) => d.map((i) => JSON.stringify(i)).sort();

/** Note ticks `from`..`to` (every `step`th), each built by `at(tick)`. */
function run(
  f: EffectFrame,
  from: number,
  to: number,
  at: (tick: number) => EffectPublication,
  step = 1,
) {
  for (let t = from; t <= to; t += step) f.note(at(t));
}

test("a known wreck burns, then smoulders, then is out", () => {
  const f = frame();
  const src = wreck(100, 200);
  const at = (t: number) => pub(t, { smokes: [src] });
  // Burning: flames licking over the hull's top, a smoke column above it.
  run(f, 1, 20 * HZ, at);
  let d = drawn(f, 20 * HZ * DT);
  expect(flames(d).length).toBeGreaterThan(3);
  for (const i of flames(d)) {
    expect(Math.hypot(i.a[0] - 100, i.a[1] - 200)).toBeLessThan(5);
    expect(i.a[2]).toBeGreaterThan(src.half[2] * 2 - 0.5);
  }
  const burning = puffs(d);
  expect(burning.length).toBeGreaterThan(20);
  expect(Math.max(...burning.map((i) => i.a[2]))).toBeGreaterThan(15);
  expect(d.some((i) => i.shape === SHAPE.glow)).toBe(true); // the fire's light
  expect(f.stats().sources).toBe(1);

  // Smouldering: no fire, thinner smoke.
  const smoulder = Math.round((WRECK.burn_s + 20) * HZ);
  run(f, 20 * HZ + 1, smoulder, at);
  d = drawn(f, smoulder * DT);
  expect(flames(d)).toEqual([]);
  expect(d.some((i) => i.shape === SHAPE.glow)).toBe(false);
  expect(puffs(d).length).toBeGreaterThan(0);
  expect(puffs(d).length).toBeLessThan(burning.length);

  // Out: nothing left, though the wreck is still known.
  const out = Math.ceil((WRECK.burn_s + WRECK.smoulder_s + LONGEST) * HZ);
  run(f, smoulder + 1, out, at);
  expect(drawn(f, out * DT)).toEqual([]);
  expect(f.stats()).toMatchObject({ live: 0, sources: 0 });
});

test("every effect ends within its bounded life, and a frame never exceeds capacity", () => {
  const f = frame();
  const wrecks = Array.from({ length: 150 }, (_, i) => wreck(10 * i, 5 * i));
  const busy = (t: number) =>
    pub(t, {
      smokes: wrecks,
      blasts: t % 10 === 0 ? [{ point: [t, 0, 0], radius: 8, kind: "tank_he" }] : [],
      shooters: [{ ...tank(7, 0), position: [4 * t * DT, 30, 0] }],
    });
  const last = 30 * HZ;
  for (let t = 1; t <= last; t++) {
    f.note(busy(t));
    const batch = f.build(t * DT, createEffectBatch(PRESENTATION.capacity));
    expect(batch.count).toBeLessThanOrEqual(PRESENTATION.capacity);
    expect(batch.dropped).toBe(0);
  }
  // The side's publications stop (the battle ends, the tab closes): gone.
  expect(drawn(f, last * DT + LONGEST)).toEqual([]);
  expect(f.stats().live).toBe(0);
});

test("what a wreck draws does not hang on how publications arrive", () => {
  const src = wreck(-40, 12);
  const at = (t: number) => pub(t, { smokes: [src] });
  const steady = frame();
  run(steady, 1, 300, at);
  // A hidden tab or a slow worker: publications late and far apart.
  const sparse = frame();
  sparse.note(at(1));
  run(sparse, 40, 300, at, 13);
  sparse.note(at(300));
  for (const clock of [300 * DT, 299.5 * DT])
    expect(sorted(drawn(sparse, clock))).toEqual(sorted(drawn(steady, clock)));
});

test("after a long gap only what would still be alive is made", () => {
  const src = wreck(0, 0);
  const f = frame();
  f.note(pub(1, { smokes: [src] }));
  // Ten minutes on, long after the wreck went out: nothing to catch up on.
  const later = 1 + 600 * HZ;
  f.note(pub(later, { smokes: [src] }));
  expect(drawn(f, later * DT)).toEqual([]);
  expect(f.stats()).toMatchObject({ live: 0, sources: 0 });
});

test("paused, the smoke holds; reset clears it and a new battle burns afresh", () => {
  const src = wreck(50, 50);
  const f = frame();
  run(f, 1, 90, (t) => pub(t, { smokes: [src] }));
  // Paused: the clock holds and nothing is published; the same frame again.
  const held = drawn(f, 90 * DT);
  f.note(pub(90, { smokes: [src] })); // a republished tick is ignored
  expect(drawn(f, 90 * DT)).toEqual(held);
  expect(held.length).toBeGreaterThan(0);

  f.reset();
  expect(drawn(f, 90 * DT)).toEqual([]);
  expect(f.stats()).toMatchObject({ live: 0, sources: 0 });

  // A new battle (an earlier tick) with the wreck known from its start.
  const g = frame();
  run(g, 1, 400, (t) => pub(t, { smokes: [src] }));
  g.note(pub(2, { smokes: [src] }));
  g.note(pub(3, { smokes: [src] }));
  expect(flames(drawn(g, 3 * DT)).length).toBeGreaterThan(0);
});

test("a moving hull raises dust behind it, by the distance it covers", () => {
  /** Dust puffs drawn after `seconds` of a hull moving east at `speed`. */
  const dust = (speed: number, half: EffectShooter["half"], seconds = 2) => {
    const f = frame();
    const x = (t: number) => speed * t * DT;
    const last = seconds * HZ;
    run(f, 1, last, (t) => pub(t, { shooters: [{ ...tank(3, 0), position: [x(t), 0, 0], half }] }));
    return { puffs: puffs(drawn(f, last * DT)), at: x(last) };
  };
  expect(dust(0, [3.5, 1.8, 1.2]).puffs).toEqual([]);
  expect(dust(6, null).puffs).toEqual([]); // infantry raises none
  const slow = dust(3, [3.5, 1.8, 1.2]);
  const fast = dust(6, [3.5, 1.8, 1.2]);
  expect(slow.puffs.length).toBeGreaterThan(0);
  // By distance, not by publication: twice the speed, twice the puffs.
  expect(fast.puffs.length / slow.puffs.length).toBeGreaterThan(1.6);
  expect(fast.puffs.length / slow.puffs.length).toBeLessThan(2.4);
  // Behind the hull, off both tracks.
  for (const i of fast.puffs) expect(i.a[0]).toBeLessThan(fast.at - 2);
  expect(fast.puffs.some((i) => i.a[1] > 0.5)).toBe(true);
  expect(fast.puffs.some((i) => i.a[1] < -0.5)).toBe(true);
});

test("a blast throws up a dirt column and smoke that outlast its fire", () => {
  const f = frame();
  f.note(pub(1));
  f.note(pub(2, { blasts: [{ point: [0, 0, 0], radius: 6, kind: "tank_he" }] }));
  const after = 2 * DT + PRESENTATION.blast.duration_s + 0.5;
  const d = drawn(f, after);
  expect(flames(d)).toEqual([]);
  const column = puffs(d);
  expect(column.length).toBeGreaterThan(0);
  expect(Math.max(...column.map((i) => i.a[2]))).toBeGreaterThan(5);
  expect(drawn(f, 2 * DT + LONGEST)).toEqual([]);
});

test("built every frame, each running effect is drawn once", () => {
  // Frames between publications, as the browser draws them.
  const f = frame();
  const src = wreck(0, 0);
  const batch = createEffectBatch(PRESENTATION.capacity);
  for (let t = 1; t <= 20 * HZ; t++) {
    f.note(pub(t, { smokes: [src] }));
    for (const k of [0.25, 0.5, 1]) f.build((t - 1 + k) * DT, batch);
  }
  const d = drawn(f, 20 * HZ * DT);
  expect(new Set(sorted(d)).size).toBe(d.length);
  expect(puffs(d).length).toBeLessThanOrEqual(WRECK.smoke_hz * WRECK.smoke.life_s + 1);
});

test("past the smoke budget every wreck thins alike, and none goes missing", () => {
  // An aftermath: 400 wrecks learned at once, 50 m apart.
  const wrecks = Array.from({ length: 400 }, (_, i) =>
    wreck((i % 20) * 50, Math.floor(i / 20) * 50),
  );
  const f = frame();
  run(f, 1, 15 * HZ, (t) => pub(t, { smokes: wrecks }));
  const d = drawn(f, 15 * HZ * DT);
  const smoke = [...puffs(d), ...flames(d)];
  expect(smoke.length).toBeLessThanOrEqual(PRESENTATION.smoke_budget * 1.1);
  expect(f.stats().dropped).toBe(0);
  // Every wreck still smokes: a sprite within 25 m of each.
  const near = (w: EffectSmokeSource) =>
    smoke.some((i) => Math.hypot(i.a[0] - w.center[0], i.a[1] - w.center[1]) < 25);
  expect(wrecks.filter((w) => !near(w))).toEqual([]);
});
