// @vitest-environment node
// Combat effects at their seam: publications in, the
// effect pass's instances out at a presentation clock. Every effect has a
// published cause, a publication is taken once, a tracer never leaves its
// published stretch, and the street test map's round kinds each look their own. A
// known wreck burns, smoulders and goes out; a moving hull raises dust; every
// life is bounded; what is drawn does not hang on how publications arrive;
// reset clears everything.
import { UNITS } from "./catalog";
import { expect, test } from "vitest";
import game from "@fixtures/game.json";
import { mountMuzzles } from "@packages/scene-assets/src/mountMuzzle";
import { offeredCastLights } from "@packages/battle-renderer/src/light/castLights";
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
import { impactAfter } from "@packages/battle-renderer/src/effects/cookOff";

const HZ = 30;
const DT = 1 / HZ;
const PRESENTATION = game.presentation.effects as unknown as EffectPresentation;
/** The tank's mounts' muzzles, from its type's `mounts` rows. */
const [CANNON, HMG] = mountMuzzles(UNITS.type("test_tank").mounts);
/** The cannon at rest, from the hull origin (forward, left, up): its pivot
 *  sits on the turret axis. */
const MUZZLE = [CANNON!.muzzle[0], CANNON!.muzzle[1], CANNON!.pivot[2] + CANNON!.muzzle[2]];

const frame = () => new EffectFrame({ tickHz: HZ, presentation: PRESENTATION });

test("a tracer can retain brightness at its rear without changing its flight or width", () => {
  const f = new EffectFrame({
    tickHz: HZ,
    presentation: {
      ...PRESENTATION,
      tracers: {
        ...PRESENTATION.tracers,
        rifle: {
          ...PRESENTATION.tracers.default,
          chance: 1,
          line_px: 0.65,
          tail_s: DT,
          tail_m: [1, 10],
          tail_brightness: 0.25,
          core: undefined,
          body: undefined,
          cast: undefined,
        },
      },
    },
  });
  f.note(
    pub(1, {
      segments: [
        segment([
          [0, 0, 2],
          [20, 0, 2],
        ]),
      ],
    }),
  );
  const [i] = drawn(f, DT * 0.75);
  expect(i.a.slice(0, 3)).toEqual([5, 0, 2]);
  expect(i.b.slice(0, 3)).toEqual([15, 0, 2]);
  expect(i.a[3]).toBeCloseTo(-0.65);
  // The pass squares these endpoint amplitudes to get light energy.
  expect(i.misc[1] ** 2).toBeCloseTo(0.25);
  expect(i.misc[2] ** 2).toBeCloseTo(1);
});

test("a screen-width tracer emits only a thin line, without a projectile head", () => {
  const f = new EffectFrame({
    tickHz: HZ,
    presentation: {
      ...PRESENTATION,
      tracers: {
        ...PRESENTATION.tracers,
        rifle: {
          ...PRESENTATION.tracers.default,
          chance: 1,
          line_px: 1,
          core: undefined,
          body: undefined,
          cast: undefined,
        },
      },
    },
  });
  f.note(
    pub(1, {
      segments: [
        segment([
          [0, 0, 2],
          [10, 0, 2],
        ]),
      ],
    }),
  );
  const instances = drawn(f, DT * 0.5);
  expect(instances.length).toBeGreaterThan(0);
  for (const i of instances) {
    expect(i.shape).toBe(SHAPE.streak);
    expect(i.a[3]).toBe(-1); // fixed screen width in the effect-pass packing contract
    expect(i.b[3]).toBe(1);
  }
});

test("a zoom-readable tracer carries a brightness boost for both its halo and core", () => {
  const base = PRESENTATION.tracers.default;
  const f = new EffectFrame({
    tickHz: HZ,
    presentation: {
      ...PRESENTATION,
      tracers: {
        ...PRESENTATION.tracers,
        rifle: { ...base, chance: 1, zoom_boost: 2, tail_s: DT, tail_m: [1, 10] },
      },
    },
  });
  f.note(
    pub(1, {
      segments: [
        segment([
          [0, 0, 2],
          [20, 0, 2],
        ]),
      ],
    }),
  );
  const streaks = drawn(f, DT * 0.75).filter((i) => i.shape === SHAPE.streak);
  expect(streaks).toHaveLength(2);
  const [halo, core] = streaks;
  // The sign selects a soft halo; magnitude sets its far-zoom brightness boost.
  expect(halo.misc[3]).toBe(-2);
  expect(core.misc[3]).toBe(2);
  expect(halo.b.slice(0, 3)).toEqual([15, 0, 2]);
  expect(core.b.slice(0, 3)).toEqual([15, 0, 2]);
  expect(halo.color.slice(0, 3)).toEqual(
    base.glow.color.map((c) => Math.fround(c * base.glow.intensity)),
  );
});

test("tracer sampling chooses one in five rounds once and carries that choice through flight", () => {
  const f = new EffectFrame({
    tickHz: HZ,
    presentation: {
      ...PRESENTATION,
      tracers: {
        ...PRESENTATION.tracers,
        rifle: { ...PRESENTATION.tracers.rifle, chance: 0.2 },
      },
    },
  });
  let selected: number[] | undefined;
  for (let tick = 1; tick <= 3; tick++) {
    f.note(
      pub(tick, {
        segments: Array.from({ length: 400 }, (_, id) =>
          segment([
            [(tick - 1) * 10, id * 5, 2],
            [tick * 10, id * 5, 2],
          ]),
        ),
      }),
    );
    const visible = [
      ...new Set(
        drawn(f, (tick - 0.5) * DT)
          .filter((i) => i.shape === SHAPE.streak)
          .map((i) => i.a[1]),
      ),
    ].sort((a, b) => a - b);
    expect(visible.length).toBeGreaterThanOrEqual(60);
    expect(visible.length).toBeLessThanOrEqual(100);
    if (selected) expect(visible).toEqual(selected);
    selected = visible;
  }
});

test("coincident rounds retain independent tracer choices when their paths separate", () => {
  const f = new EffectFrame({
    tickHz: HZ,
    presentation: {
      ...PRESENTATION,
      tracers: {
        ...PRESENTATION.tracers,
        rifle: {
          ...PRESENTATION.tracers.rifle,
          chance: 0.2,
        },
      },
    },
  });
  f.note(
    pub(1, {
      segments: Array.from({ length: 400 }, () =>
        segment([
          [0, 0, 2],
          [10, 0, 2],
        ]),
      ),
    }),
  );
  const first = drawn(f, DT * 0.9).filter((i) => i.shape === SHAPE.streak).length;
  expect(first).toBeGreaterThanOrEqual(60);
  expect(first).toBeLessThanOrEqual(100);
  f.note(
    pub(2, {
      segments: Array.from({ length: 400 }, (_, id) =>
        segment([
          [10, 0, 2],
          [20, id, 2],
        ]),
      ),
    }),
  );
  const second = drawn(f, DT * 1.9).filter((i) => i.shape === SHAPE.streak && i.b[0] > 18);
  expect(second.length).toBe(first);
});

test("a hull or prop hit sparks and flashes only from a tracer, a ricochet always sparks, every hit raises dust", () => {
  const withChance = (chance: number) =>
    new EffectFrame({
      tickHz: HZ,
      presentation: {
        ...PRESENTATION,
        tracers: { ...PRESENTATION.tracers, rifle: { ...PRESENTATION.tracers.rifle, chance } },
      },
    });
  const hit = (hit: string) =>
    segment(
      [
        [0, 0, 2],
        [10, 0, 2],
      ],
      { hit, normal: [-1, 0, 0] },
    );
  const ricochet = segment(
    [
      [0, 0, 2],
      [10, 0, 2],
      [10, 10, 2],
    ],
    { ricochets: [{ point: 1, normal: [-1, 0, 0] }] },
  );
  // Past the tracer's own short tail, any streak left is a spark.
  const sparks = (s: EffectSegment, chance: number) => {
    const f = withChance(chance);
    f.note(pub(1, { segments: [s] }));
    return drawn(f, DT + 0.1).filter((i) => i.shape === SHAPE.streak).length;
  };
  for (const surface of ["hull", "prop"]) {
    expect(sparks(hit(surface), 1)).toBeGreaterThan(0);
    expect(sparks(hit(surface), 0)).toBe(0);
  }
  expect(sparks(ricochet, 0)).toBeGreaterThan(0);

  // The hot flash on armour, and its light, come with the sparks.
  const hot = (chance: number) => {
    const f = withChance(chance);
    f.note(pub(1, { segments: [hit("hull")] }));
    const clock = DT + 0.01;
    return {
      flashes: drawn(f, clock).filter((i) => i.shape === SHAPE.glow).length,
      lights: lightsAt(f, clock).filter((l) => l.cause === "impact:hull").length,
    };
  };
  expect(hot(1)).toEqual({ flashes: 1, lights: 1 });
  expect(hot(0)).toEqual({ flashes: 0, lights: 0 });

  // Every hit kicks up dust; off a hull or prop, less than off the ground.
  const dust = (surface: string) => {
    const f = withChance(0);
    f.note(pub(1, { segments: [hit(surface)] }));
    const puffs = drawn(f, DT + 1e-3).filter((i) => i.shape === SHAPE.flipbook);
    expect(puffs).toHaveLength(1);
    return puffs[0].a[3];
  };
  const ground = dust("ground");
  for (const surface of ["hull", "prop"]) {
    expect(dust(surface)).toBeGreaterThan(0);
    expect(dust(surface)).toBeLessThan(ground);
  }
});

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
  yaw: 0,
  members,
  mounts: [{ bearing: 0, elevation: 0, shots, kind: "rifle", muzzle: null }],
});

const tank = (key: number, shots: number): EffectShooter => ({
  key,
  position: [100, 50, 0],
  half: [3.5, 1.8, 1.2],
  yaw: 0,
  members: [],
  mounts: [{ bearing: Math.PI / 2, elevation: 0, shots, kind: "tank_ap", muzzle: CANNON }],
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

test("ground-impact round scaling changes size and lifetime without prolonging hull impacts", () => {
  const f = new EffectFrame({
    tickHz: HZ,
    presentation: {
      ...PRESENTATION,
      impact_scale: { default: 1 },
      impacts: {
        ...PRESENTATION.impacts,
        ground: {
          color: [1, 1, 1],
          opacity: 0.5,
          size_m: 1,
          duration_s: 1,
          sparks: 0,
          flash: 0,
          round_scale: { large: { size: 5, duration: 5 } },
        },
        hull: { color: [1, 1, 1], opacity: 0.5, size_m: 1, duration_s: 1, sparks: 0, flash: 0 },
      },
    },
  });
  f.note(
    pub(1, {
      segments: [
        segment(
          [
            [0, 0, 2],
            [0, 0, 0],
          ],
          { kind: "small", hit: "ground" },
        ),
        segment(
          [
            [100, 0, 2],
            [100, 0, 0],
          ],
          { kind: "large", hit: "ground" },
        ),
        segment(
          [
            [200, 0, 2],
            [200, 0, 0],
          ],
          { kind: "large", hit: "hull" },
        ),
      ],
    }),
  );
  const small = drawn(f, DT + 0.2).find((i) => i.shape === SHAPE.flipbook && i.a[0] === 0)!;
  const later = drawn(f, DT + 1);
  const large = later.find((i) => i.shape === SHAPE.flipbook && i.a[0] === 100)!;
  expect(small).toBeDefined();
  expect(large).toBeDefined();
  expect(large.a[3]).toBeCloseTo(small.a[3] * 5);
  expect(large.color[3]).toBeCloseTo(small.color[3]);
  expect(later.some((i) => i.shape === SHAPE.flipbook && i.a[0] === 0)).toBe(false);
  expect(later.some((i) => i.shape === SHAPE.flipbook && i.a[0] === 200)).toBe(false);
  expect(drawn(f, DT + 4.9).some((i) => i.shape === SHAPE.flipbook && i.a[0] === 100)).toBe(true);
  expect(drawn(f, DT + 5).some((i) => i.shape === SHAPE.flipbook && i.a[0] === 100)).toBe(false);
});

test("the effect lifetime bound includes scaled impact durations", () => {
  const presentation: EffectPresentation = {
    ...PRESENTATION,
    impacts: {
      ...PRESENTATION.impacts,
      ground: {
        ...PRESENTATION.impacts.ground,
        duration_s: 2,
        round_scale: { long: { size: 1, duration: 50 } },
      },
    },
  };
  expect(maxEffectLifetime(presentation)).toBeGreaterThanOrEqual(100);
});

test("a tank's roof HMG flashes at its own muzzle, turned away from the cannon", () => {
  // The hull faces 0.3 rad, the turret with it; the HMG fires backwards. Its
  // pivot rides the turret roof, its short gun points behind: never the
  // cannon's tip, nor a point out in the air.
  const [yaw, back] = [0.3, 0.3 + Math.PI];
  const hmgTank = (shots: number): EffectShooter => ({
    key: 2,
    position: [100, 50, 0],
    half: [3.5, 1.8, 1.2],
    yaw,
    members: [],
    mounts: [
      { bearing: yaw, elevation: 0, shots: 0, kind: "tank_ap", muzzle: CANNON },
      { bearing: back, elevation: 0, shots, kind: "hmg", muzzle: HMG },
    ],
  });
  const f = frame();
  f.note(pub(5, { shooters: [hmgTank(7)] }));
  f.note(pub(6, { shooters: [hmgTank(8)] }));
  const flash = drawn(f, 5.01 * DT).filter((i) => i.shape === SHAPE.glow);
  expect(flash).toHaveLength(1);
  const [px, py, pz] = HMG!.pivot;
  const [mx, my, mz] = HMG!.muzzle;
  const turn = (x: number, y: number, a: number) => [
    x * Math.cos(a) - y * Math.sin(a),
    x * Math.sin(a) + y * Math.cos(a),
  ];
  const [ax, ay] = turn(px, py, yaw);
  const [bx, by] = turn(mx, my, back);
  expect(flash[0].a[0]).toBeCloseTo(100 + ax + bx, 5);
  expect(flash[0].a[1]).toBeCloseTo(50 + ay + by, 5);
  expect(flash[0].a[2]).toBeCloseTo(pz + mz, 5);
  // On the turret roof: well inside the hull's length, above its top.
  expect(Math.hypot(flash[0].a[0] - 100, flash[0].a[1] - 50)).toBeLessThan(3.5);
  expect(flash[0].a[2]).toBeGreaterThan(2.4);
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

/** The lights cast at `clock`: where, how far, what colour and what cast them. */
function lightsAt(f: EffectFrame, clock: number, muzzles: MuzzleSource | null = null) {
  return offeredCastLights(
    f.build(clock, createEffectBatch(PRESENTATION.capacity), muzzles).lights,
  );
}

test("a shot lights what is round its drawn muzzle, by its round kind's row, until it burns out", () => {
  const row = PRESENTATION.flashes.tank_ap.cast!;
  const f = frame();
  f.note(pub(5, { shooters: [tank(2, 7)] }));
  f.note(pub(6, { shooters: [tank(2, 8)] }));
  const muzzles: MuzzleSource = {
    muzzle(_shooter, _mount, _soldier, at) {
      at[0] = 99;
      at[1] = 55.5;
      at[2] = 1.8;
      return true;
    },
  };
  // Born at the tick's start, a light at the drawn muzzle, carried
  // `forward_m` along the shot (the tank fires along +y), at full strength.
  const [born] = lightsAt(f, 5 * DT, muzzles);
  expect(born.cause).toBe("flash:tank_ap");
  expect(born.at[0]).toBeCloseTo(99, 4);
  expect(born.at[1]).toBeCloseTo(55.5 + (row.forward_m ?? 0), 4);
  expect(born.at[2]).toBeCloseTo(1.8, 4);
  expect(born.radius).toBeCloseTo(row.radius_m, 5);
  for (let k = 0; k < 3; k++) expect(born.rgb[k]).toBeCloseTo(row.color[k] * row.intensity, 4);
  // Dimmer as it burns, then out.
  const half = lightsAt(f, 5 * DT + row.duration_s / 2, muzzles);
  expect(half[0].rgb[0]).toBeLessThan(born.rgb[0]);
  expect(half[0].rgb[0]).toBeGreaterThan(0);
  expect(lightsAt(f, 5 * DT + row.duration_s + 1e-4, muzzles)).toEqual([]);
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
      segments: [segment(path, { kind: "tank_ap", ricochets: [{ point: 1, normal: [-1, 0, 0] }] })],
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
  // The round's streaks (glow and core): its kind's colours.
  const { glow, core } = PRESENTATION.tracers.tank_ap;
  const tint = (l: { color: number[]; intensity: number }) => Math.fround(l.color[1] * l.intensity);
  const streakTints = [tint(glow), tint(core!)];
  let streaks = 0;
  for (let k = 0; k <= 40; k++) {
    const t = DT + (k / 20) * DT;
    for (const i of drawn(f, t)) {
      if (i.shape !== SHAPE.streak || !streakTints.includes(i.color[1])) continue;
      streaks++;
      expect(onPath(i.a)).toBe(true);
      expect(onPath(i.b)).toBe(true);
    }
  }
  expect(streaks).toBeGreaterThan(10);
});

/** How far behind the head a round's glow reaches at the end of its tick,
 *  flying `speed` m/s along x. */
function tailBehind(kind: string, speed: number): number {
  const f = frame();
  const step = speed * DT;
  f.note(
    pub(1, {
      segments: [
        segment(
          [
            [0, 0, 1],
            [step, 0, 1],
          ],
          { kind },
        ),
      ],
    }),
  );
  f.note(
    pub(2, {
      segments: [
        segment(
          [
            [step, 0, 1],
            [2 * step, 0, 1],
          ],
          { kind },
        ),
      ],
    }),
  );
  const streaks = drawn(f, 2 * DT).filter((i) => i.shape === SHAPE.streak);
  return 2 * step - Math.min(...streaks.map((i) => i.a[0]));
}

test("a fast round draws a long bolt, a slow one a short point", () => {
  const { tail_s, tail_m } = PRESENTATION.tracers.tank_ap;
  // Within the row's bounds the tail is the flight of its last tail_s.
  const speed = (tail_m[0] + tail_m[1]) / 2 / tail_s;
  expect(tailBehind("tank_ap", speed)).toBeCloseTo(speed * tail_s, 3);
  expect(tailBehind("tank_ap", speed * 0.5)).toBeCloseTo(speed * 0.5 * tail_s, 3);
  // Past them it holds at the bound.
  expect(tailBehind("tank_ap", 1e5)).toBeCloseTo(tail_m[1], 3);
  expect(tailBehind("tank_ap", 1)).toBeCloseTo(Math.min(tail_m[0], 2 / HZ), 3);
});

test("paused at a tick's end, a round seen as an object is drawn at its head, once", () => {
  const f = frame();
  for (let t = 1; t <= 9; t++)
    f.note(
      pub(t, {
        segments: [
          segment(
            [
              [t * 9 - 9, 0, 1],
              [t * 9, 0, 1],
            ],
            { kind: "grenade" },
          ),
        ],
      }),
    );
  for (let t = 2; t <= 9; t++) {
    // The grenade's body: a disc, blended over (its colour carries an opacity).
    const heads = drawn(f, t * DT).filter((i) => i.shape === SHAPE.glow && i.color[3] > 0);
    expect(heads.map((i) => i.a[0])).toEqual([t * 9]);
  }
});

test("a round whose look casts light lights the ground from its head as it flies; one without casts none", () => {
  const f = frame();
  const hops = (kind: string) =>
    [1, 2, 3].map((t) =>
      segment(
        [
          [t * 2 - 2, 0, 1.5],
          [t * 2, 0, 1.5],
        ],
        { kind },
      ),
    );
  const atgm = hops("atgm");
  const grenade = hops("grenade").map((s) => ({ ...s, path: s.path.map(([x]) => [x, 50, 3]) }));
  for (let t = 1; t <= 3; t++)
    f.note(pub(t, { segments: [atgm[t - 1], grenade[t - 1] as EffectSegment] }));
  const row = PRESENTATION.tracers.atgm.cast!;
  expect(PRESENTATION.tracers.grenade.cast).toBeUndefined();
  // Mid-tick, and paused at a tick's end: one light each time, on the head.
  for (const [clock, head] of [
    [1.5 * DT, 3],
    [2 * DT, 4],
    [2.25 * DT, 4.5],
  ]) {
    const lights = lightsAt(f, clock);
    expect(lights.map((l) => l.cause)).toEqual(["round:atgm"]);
    expect(lights[0].at[0]).toBeCloseTo(head, 4);
    expect(lights[0].at[1]).toBeCloseTo(0, 4);
    expect(lights[0].radius).toBeCloseTo(row.radius_m, 5);
  }
});

test("a burst lights round it within its life, a bigger one farther", () => {
  const f = frame();
  f.note(pub(1));
  // The burst the style's numbers describe, one smaller and one bigger.
  const style = PRESENTATION.blast;
  const reference = style.reference_size_m / style.size_per_radius;
  f.note(
    pub(2, {
      blasts: [
        { point: [0, 0, 0], radius: reference / 2, kind: "tank_he" },
        { point: [250, 0, 0], radius: reference, kind: "grenade" },
        { point: [500, 0, 0], radius: 20, kind: "tank_he" },
      ],
    }),
  );
  const row = PRESENTATION.blast.cast;
  const [small, middle, big] = lightsAt(f, 2 * DT + 0.01).sort((a, b) => a.at[0] - b.at[0]);
  expect(small.cause).toBe("blast");
  expect(middle.radius).toBeCloseTo(row.radius_m);
  expect(small.radius).toBeLessThan(middle.radius);
  expect(big.radius).toBeGreaterThan(middle.radius);
  expect(lightsAt(f, 2 * DT + row.duration_s + 1e-4)).toEqual([]);
});

test("a burst smaller than the style's draws a smaller fireball, fewer sparks and less dirt and smoke", () => {
  const style = PRESENTATION.blast;
  const reference = style.reference_size_m / style.size_per_radius;
  const burst = (radius: number) => {
    const f = frame();
    f.note(pub(1));
    f.note(pub(2, { blasts: [{ point: [0, 0, 0], radius, kind: "tank_he" }] }));
    const d = drawn(f, 2 * DT + 0.05);
    // A flipbook's radius is its fourth float; sparks are the only streaks.
    return {
      fire: Math.max(...flames(d).map((i) => i.a[3])),
      sparks: d.filter((i) => i.shape === SHAPE.streak).length,
      puff: Math.max(...puffs(drawn(f, 2 * DT + 1)).map((i) => i.a[3])),
    };
  };
  const [small, full, big] = [reference / 2, reference, reference * 2].map(burst);
  expect(small.fire).toBeLessThan(full.fire);
  expect(small.sparks).toBeLessThan(full.sparks);
  expect(small.puff).toBeLessThan(full.puff);
  // Bigger bursts throw no more sparks than the style's: theirs are its count.
  expect(big.sparks).toBe(full.sparks);
  expect(big.fire).toBeGreaterThan(full.fire);
});

test("a round's blast scale shrinks its burst's fireball, light and dirt from its radius's", () => {
  const presentation = { ...PRESENTATION, blast_scale: { default: 1, small: 0.5 } };
  const burst = (kind: string) => {
    const f = new EffectFrame({ tickHz: HZ, presentation });
    f.note(pub(1));
    f.note(pub(2, { blasts: [{ point: [0, 0, 0], radius: 6, kind }] }));
    return {
      fire: Math.max(...flames(drawn(f, 2 * DT + 0.05)).map((i) => i.a[3])),
      light: lightsAt(f, 2 * DT + 0.01)[0].radius,
    };
  };
  const [small, plain] = [burst("small"), burst("grenade")];
  expect(small.fire).toBeLessThan(plain.fire);
  expect(small.light).toBeLessThan(plain.light);
});

test("a missile leaves a smoke trail along its flight that lingers after it", () => {
  const trail = PRESENTATION.tracers.atgm.smoke!;
  const f = frame();
  const path: [number, number, number][] = [
    [0, 0, 1.5],
    [6, 0, 1.5],
  ];
  f.note(pub(1, { segments: [segment(path, { kind: "atgm" })] }));
  const smoke = (t: number) =>
    drawn(f, t).filter((i) => i.shape === SHAPE.flipbook && i.misc[2] === 1);
  const born = smoke(DT);
  expect(born.length).toBeGreaterThanOrEqual(Math.floor(6 / trail.spacing_m));
  for (const p of born) expect(p.a[0]).toBeGreaterThanOrEqual(-0.5);
  // The round has long gone (no light left); its smoke still hangs where it flew.
  const later = DT + trail.life_s * 0.5;
  const streaks = (t: number) => drawn(f, t).filter((i) => i.shape === SHAPE.streak);
  expect(streaks(later).filter((i) => i.color[3] === 0)).toEqual([]);
  expect(streaks(later).length).toBe(1);
  expect(smoke(later).length).toBe(born.length);
  expect(smoke(2 * DT + trail.life_s)).toEqual([]);
  expect(streaks(2 * DT + trail.life_s)).toEqual([]);
});

test("a smoke trail's ribbon runs unbroken along the whole flight, however it bends", () => {
  const f = frame();
  // An arc over six ticks, slowing: stretches of different lengths and headings.
  const at = (k: number): [number, number, number] => [
    k * (12 - k),
    k * 3,
    1.5 + k * (6 - k) * 0.4,
  ];
  for (let k = 1; k <= 6; k++)
    f.note(pub(k, { segments: [segment([at(k - 1), at(k)], { kind: "atgm" })] }));
  for (const t of [6 * DT, 6 * DT + 2, 6 * DT + 8]) {
    const ribbon = drawn(f, t).filter((i) => i.shape === SHAPE.streak && i.color[3] > 0);
    expect(ribbon.length).toBe(6);
    // Laid in flight order, each piece starts exactly where the last ends.
    for (let k = 1; k < ribbon.length; k++)
      for (let j = 0; j < 3; j++) expect(ribbon[k].a[j]).toBeCloseTo(ribbon[k - 1].b[j], 5);
    // All of it under every puff: no later stretch's ribbon covers an earlier one's puffs.
    const all = drawn(f, t);
    const kinds = all.map((i) => (i.shape === SHAPE.streak && i.color[3] > 0 ? "ribbon" : i.shape));
    expect(kinds.lastIndexOf("ribbon")).toBeLessThan(kinds.indexOf(SHAPE.flipbook));
  }
});

test("a smoke trail runs on evenly from one tick's stretch to the next", () => {
  const trail = PRESENTATION.tracers.atgm.smoke!;
  const f = frame();
  const at = (x: number): [number, number, number] => [x, 3, 1.5];
  f.note(pub(1, { segments: [segment([at(0.37), at(7.9)], { kind: "atgm" })] }));
  f.note(pub(2, { segments: [segment([at(7.9), at(19.3)], { kind: "atgm" })] }));
  // Born, not yet drifted: their places as laid.
  const xs = drawn(f, 2 * DT)
    .filter((i) => i.shape === SHAPE.flipbook && i.misc[2] === 1)
    .map((i) => i.a[0])
    .sort((a, b) => a - b);
  const gaps = xs.slice(1).map((x, k) => x - xs[k]);
  expect(xs.length).toBeGreaterThan(4);
  for (const g of gaps) expect(g).toBeCloseTo(trail.spacing_m, 0);
});

test("a tracer row that cannot draw is refused", () => {
  const bad = (row: object) => () =>
    new EffectFrame({
      tickHz: HZ,
      presentation: {
        ...PRESENTATION,
        tracers: { ...PRESENTATION.tracers, rifle: { ...PRESENTATION.tracers.rifle, ...row } },
      },
    });
  expect(bad({})).not.toThrow();
  expect(bad({ tail_s: 0 })).toThrow(/tail_s/);
  expect(bad({ tail_m: [5, 1] })).toThrow(/tail_m/);
  expect(bad({ zoom_boost: 0 })).toThrow(/zoom_boost/);
  expect(bad({ zoom_boost: Infinity })).toThrow(/zoom_boost/);
  expect(bad({ core: { ...PRESENTATION.tracers.rifle.core, share: 1.5 } })).toThrow(/share/);
});

// ---- Smoke, fire and dust. ----

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

test("tracer core and glow retain their own pixel widths", () => {
  const rifle = PRESENTATION.tracers.default;
  const f = new EffectFrame({
    tickHz: HZ,
    presentation: {
      ...PRESENTATION,
      min_px: 9,
      tracers: {
        ...PRESENTATION.tracers,
        rifle: {
          ...rifle,
          glow: { ...rifle.glow, min_px: 1.125 },
          core: { ...rifle.core!, min_px: 0.375 },
        },
      },
    },
  });
  f.note(pub(1));
  f.note(
    pub(2, {
      segments: [
        segment([
          [0, 0, 2],
          [10, 0, 2],
        ]),
      ],
    }),
  );
  const streaks = drawn(f, 1.5 * DT).filter((i) => i.shape === SHAPE.streak);
  expect(streaks.map((i) => i.b[3]).sort()).toEqual([0.375, 1.125]);
});

test("a hull watched die cooks off a beat after the hit, and its turret lands in dust", () => {
  const c = { ...PRESENTATION.cook_off, delay_s: 0.35 };
  const f = new EffectFrame({
    tickHz: HZ,
    presentation: { ...PRESENTATION, cook_off: c },
  });
  f.note(pub(1));
  // Its turret, in this test, lands well clear of the fireballs' own dust.
  f.note(pub(2, { cookOffs: [{ center: [0, 0, 0], height: 2.4, landing: [30, 0, 0] }] }));
  const hit = DT;
  const fireballs = (t: number) => drawn(f, t).filter((i) => i.shape === SHAPE.flipbook);
  expect(fireballs(hit + c.delay_s * 0.5)).toEqual([]);
  const roar = fireballs(hit + c.delay_s + 0.05);
  expect(roar.length).toBeGreaterThan(0);
  for (const b of roar) expect(b.a[2]).toBeGreaterThan(2.4);
  const landingDust = (t: number) =>
    puffs(drawn(f, t)).filter((i) => Math.hypot(i.a[0] - 30, i.a[1]) < 6);
  expect(landingDust(hit + impactAfter(c) - 0.05)).toEqual([]);
  expect(landingDust(hit + impactAfter(c) + 0.3).length).toBeGreaterThan(0);
  expect(drawn(f, hit + LONGEST + 1)).toEqual([]);
});
