// @vitest-environment node
// The battle's sound at its seam: publications, drawn motion and
// a presentation clock in, voices on a sink out. Only what the side's
// observation contains makes a positional sound, an unseen enemy is heard
// only as its hearing cue's direction; the voice budget holds at battle
// scale; a standing clock silences transients and holds loops; reset and a
// new battle clear everything; the side's own sounds stay heard at the
// farthest zoom, coloured far.
import { expect, test } from "vitest";
import game from "@fixtures/game.json";
import type { AudioPresentation } from "@packages/battle-audio/src/audioPresentation";
import { cameraListener } from "@packages/battle-audio/src/battleAudio";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import {
  distanceGain,
  SoundFrame,
  type Listener,
  type SoundCue,
  type SoundMotion,
  type VoiceParams,
  type VoiceSink,
  type VoiceSpec,
  vehicleLoops,
} from "@packages/battle-audio/src/soundFrame";
import { synthesize } from "@packages/battle-audio/src/synth";
import { impactAfter } from "@packages/battle-renderer/src/effects/cookOff";
import type {
  EffectPresentation,
  EffectPublication,
  EffectShooter,
} from "@packages/battle-renderer/src/effects/effectFrame";

const HZ = 30;
const DT = 1 / HZ;
const AUDIO = game.presentation.audio as unknown as AudioPresentation;
const SMOKE = (game.presentation.effects as unknown as EffectPresentation).smoke;
const COOK_OFF = (game.presentation.effects as unknown as EffectPresentation).cook_off;
const LISTENER: Listener = { position: [0, -20, 20], forward: [0, 1, 0] };
const STILL: SoundMotion = { vehicles: [], soldiers: [] };

/** A sink that keeps what it was told, on a clock the test sets. */
class FakeSink implements VoiceSink {
  time = 0;
  readonly voices = new Map<number, { spec: VoiceSpec; end: number }>();
  readonly started: VoiceSpec[] = [];
  now() {
    return this.time;
  }
  duration() {
    return 0.5;
  }
  start(id: number, spec: VoiceSpec) {
    this.started.push(spec);
    this.voices.set(id, { spec, end: spec.loop ? Infinity : spec.at + 0.5 });
  }
  set(id: number, p: VoiceParams) {
    const v = this.voices.get(id);
    if (v) v.spec = { ...v.spec, ...p };
  }
  stop(id: number) {
    this.voices.delete(id);
  }
  listen() {}
  /** Voices sounding now. */
  live(loop?: boolean) {
    return [...this.voices.values()].filter(
      (v) => v.end > this.time && (loop === undefined || v.spec.loop === loop),
    );
  }
}

function setup() {
  const sink = new FakeSink();
  const frame = new SoundFrame(
    {
      tickHz: HZ,
      presentation: AUDIO,
      catalog: {
        sources: {},
        clips: {},
        sounds: {},
        defaults: {},
        impacts: {},
        effects: {},
      },
      smokeTimes: SMOKE,
      cookOff: COOK_OFF,
    },
    sink,
  );
  return { sink, frame };
}

const squad = (key: number, member: number, at: number[], shots: number): EffectShooter => ({
  key,
  position: at,
  half: null,
  yaw: 0,
  members: [member],
  mounts: [{ bearing: 0, elevation: 0, shots, kind: "rifle", muzzle: null }],
});

/** A publication where each shooter's one soldier fires a new round at tick `tick`. */
function firefight(tick: number, shooters: { key: number; at: number[] }[]): EffectPublication {
  return {
    tick,
    shooters: shooters.map((s) => squad(s.key, s.key * 10, s.at, tick)),
    segments: shooters.map((s) => ({
      path: [s.at, [s.at[0] + 10, s.at[1], s.at[2]]],
      ricochets: [],
      kind: "rifle",
      shooter: s.key * 10,
      hit: "ground",
      normal: [0, 0, 1],
    })),
    blasts: [],
    smokes: [],
  };
}

/** Run ticks `from..to`, each noted and heard at its own presentation time. */
function run(
  frame: SoundFrame,
  sink: FakeSink,
  from: number,
  to: number,
  pub: (tick: number) => EffectPublication,
  audible: (tick: number) => SoundCue[] = () => [],
  motion: SoundMotion = STILL,
) {
  for (let tick = from; tick <= to; tick++) {
    frame.note({ effects: pub(tick), audible: audible(tick) });
    sink.time = tick * DT;
    frame.update(tick * DT, tick * DT, motion, LISTENER);
  }
}

const near = (spec: VoiceSpec, at: number[], within: number) =>
  spec.position !== null && Math.hypot(spec.position[0] - at[0], spec.position[1] - at[1]) < within;

test("an unseen enemy's shot makes no positional sound, only its cue", () => {
  const blue = { key: 2, at: [0, 0, 1] };
  const red = { key: 3, at: [60, 40, 1] };
  const cue: SoundCue = { category: "shot", sector: 1, band: "near", moving: false };

  // Seen: the red rifleman is identified, so his shots and impacts sound where they are.
  const seen = setup();
  run(seen.frame, seen.sink, 1, 20, (t) => firefight(t, [blue, red]));
  expect(seen.sink.started.some((v) => near(v, red.at, 15))).toBe(true);

  // Unseen: the same battle, but red is not in the observation; the side
  // hears him only as a cue. Nothing sounds near him, and the only voices
  // without a position (besides the bed) are the cue's, panned by sector.
  const unseen = setup();
  run(
    unseen.frame,
    unseen.sink,
    1,
    20,
    (t) => firefight(t, [blue]),
    () => [cue],
  );
  const positional = unseen.sink.started.filter((v) => v.position !== null);
  expect(positional.some((v) => near(v, red.at, 15))).toBe(false);
  const cues = unseen.sink.started.filter((v) => v.position === null && !v.loop);
  expect(cues.length).toBeGreaterThan(0);
  expect(cues.every((v) => v.sound === AUDIO.cues.sounds.shot && v.pan !== null)).toBe(true);

  // Metamorphic: where the unseen enemy really is (another sector, another
  // band) changes only the cue, never a positional voice.
  const moved = setup();
  run(
    moved.frame,
    moved.sink,
    1,
    20,
    (t) => firefight(t, [blue]),
    () => [{ ...cue, sector: 5, band: "far" }],
  );
  const strip = (v: VoiceSpec) => JSON.stringify(v);
  expect(moved.sink.started.filter((v) => v.position !== null).map(strip)).toEqual(
    positional.map(strip),
  );
  // And an unseen red changes nothing positional: the side hears exactly
  // what it hears with no red there at all. (A seen red may: his sounds
  // share the voice budget with blue's, loudest first.)
  const alone = setup();
  run(alone.frame, alone.sink, 1, 20, (t) => firefight(t, [blue]));
  expect(alone.sink.started.filter((v) => v.position !== null).map(strip)).toEqual(
    positional.map(strip),
  );
});

test("the voice budget holds in a 100-a-side firefight, loudest first", () => {
  const { sink, frame } = setup();
  const shooters = Array.from({ length: 200 }, (_, i) => ({
    key: i + 1,
    at: [(i % 20) * 12 - 120, Math.floor(i / 20) * 15, 1],
  }));
  const vehicles = Array.from({ length: 40 }, (_, i) => ({
    key: 1000 + i,
    vehicleClass: ["tracked_heavy", "wheeled_medium_logistics", "wheeled_light"][i % 3],
    position: [i * 8 - 160, 60, 1],
    travelL: 0,
    travelR: 0,
    turret: 0,
  }));
  const soldiers = shooters.map((s) => ({ id: s.key * 10, position: s.at }));
  let most = 0;
  let mostLoops = 0;
  for (let tick = 1; tick <= 90; tick++) {
    frame.note({ effects: firefight(tick, shooters), audible: [] });
    sink.time = tick * DT;
    for (const v of vehicles) {
      v.travelL += 0.2;
      v.travelR += 0.2;
    }
    for (const s of soldiers) s.position = [s.position[0] + 0.1, s.position[1], 1];
    frame.update(tick * DT, tick * DT, { vehicles, soldiers }, LISTENER);
    most = Math.max(most, sink.live().length);
    mostLoops = Math.max(mostLoops, sink.live(true).length);
  }
  // Every voice at once, the bed included, within the budget (+1 for the bed).
  expect(most).toBeLessThanOrEqual(AUDIO.budget.voices + 1);
  expect(mostLoops).toBeLessThanOrEqual(AUDIO.budget.loops + 1);
  const s = frame.stats();
  expect(s.dropped + s.stolen).toBeGreaterThan(0); // the budget did bite
  // The rifleman nearest the listener is heard.
  expect(sink.started.some((v) => near(v, [0, 0], 3))).toBe(true);
});

test("a standing clock silences transients and holds loops; it plays on after", () => {
  const { sink, frame } = setup();
  const tank = {
    key: 1,
    vehicleClass: "tracked_heavy",
    position: [20, 20, 1],
    travelL: 0,
    travelR: 0,
    turret: 0,
  };
  const motion = { vehicles: [tank], soldiers: [] };
  run(frame, sink, 1, 10, (t) => firefight(t, [{ key: 2, at: [0, 0, 1] }]), undefined, motion);
  expect(sink.live(false).length).toBeGreaterThan(0);
  const loops = sink.live(true).length;
  expect(loops).toBeGreaterThan(0); // the bed and the tank's engine

  // Paused: the clock stands at tick 10 while wall time runs on.
  const clock = 10 * DT;
  const startedBefore = sink.started.length;
  for (let k = 1; k <= 30; k++) {
    sink.time = clock + k * 0.05;
    frame.update(clock, clock + k * 0.05, motion, LISTENER);
  }
  expect(frame.stats().held).toBe(true);
  expect(sink.live(false)).toHaveLength(0);
  expect(sink.live(true)).toHaveLength(loops);
  expect(sink.started.length).toBe(startedBefore);

  // Resumed: the clock moves and shots sound again.
  for (let tick = 11; tick <= 14; tick++) {
    frame.note({ effects: firefight(tick, [{ key: 2, at: [0, 0, 1] }]), audible: [] });
    sink.time += DT;
    frame.update(tick * DT, sink.time, motion, LISTENER);
  }
  expect(frame.stats().held).toBe(false);
  expect(sink.live(false).length).toBeGreaterThan(0);
});

test("reset, or a new battle, clears every voice and count", () => {
  const { sink, frame } = setup();
  const pub = (t: number) => ({
    ...firefight(t, [{ key: 2, at: [0, 0, 1] }]),
    smokes: [{ key: "w", kind: "wreck", center: [30, 0, 0], yaw: 0, half: [3, 2, 1] }],
  });
  run(frame, sink, 1, 10, pub);
  expect(sink.voices.size).toBeGreaterThan(1); // shots, the fire, the bed
  frame.reset();
  expect(sink.voices.size).toBe(0);
  expect(frame.stats()).toMatchObject({ transients: 0, loops: 0, pending: 0, started: 0 });

  // A publication earlier than the last is a new battle: the same.
  run(frame, sink, 20, 30, pub);
  expect(sink.voices.size).toBeGreaterThan(1);
  frame.note({ effects: firefight(2, []), audible: [] });
  expect(sink.voices.size).toBe(0);
});

test("a known wreck burns, then smoulders quieter, then falls silent", () => {
  const { sink, frame } = setup();
  const wreck = SMOKE.wreck;
  const smokes = [{ key: "w", kind: "wreck", center: [30, 0, 0], yaw: 0, half: [3, 2, 1] }];
  const fireGain = () =>
    [...sink.voices.values()].find((v) => v.spec.sound === AUDIO.fires.default.sound)?.spec.gain ??
    0;
  const at = (s: number) => {
    const tick = Math.round(s * HZ);
    frame.note({ effects: { ...firefight(tick, []), smokes }, audible: [] });
    sink.time = tick * DT;
    frame.update(tick * DT, tick * DT, STILL, LISTENER);
    return fireGain();
  };
  const burning = at(1);
  at(wreck.burn_s * 0.5);
  const smouldering = at(wreck.burn_s + wreck.smoulder_s * 0.5);
  const out = at(wreck.burn_s + wreck.smoulder_s + 5);
  expect(burning).toBeGreaterThan(smouldering);
  expect(smouldering).toBeGreaterThan(0);
  expect(out).toBe(0);
});

/** The camera at its farthest zoom, looking at `target` (the fixture's rig). */
function farthestCamera(target: number[]) {
  const cam = game.presentation.camera;
  const pitch = cam.pitch_curve[cam.pitch_curve.length - 1][1];
  return { target: [target[0], target[1], 0], distance: cam.zoom_max, pitch, yaw: -1.57 };
}

/** One blue rifleman's shots at `at`, heard from `listener`: the positional
 *  transients started. */
function heardFrom(listener: Listener, at: number[]) {
  const { sink, frame } = setup();
  for (let tick = 1; tick <= 10; tick++) {
    frame.note({ effects: firefight(tick, [{ key: 2, at }]), audible: [] });
    sink.time = tick * DT;
    frame.update(tick * DT, tick * DT, STILL, listener);
  }
  return sink.started.filter((v) => v.position !== null && !v.loop);
}

test("the side's own shot at the farthest zoom sounds at the floor at least, and far away", () => {
  const at = [200, 300, 1];
  const shot = AUDIO.shots.rifle;
  const listener = cameraListener(
    farthestCamera(at) as unknown as Parameters<typeof cameraListener>[0],
    AUDIO.listener_eye_share,
  );
  const far = heardFrom(listener, at).filter((v) => v.sound === shot.far);
  const close = heardFrom({ position: [at[0], at[1] - 5, 6], forward: [0, 1, 0] }, at).filter(
    (v) => v.sound === shot.near,
  );
  expect(far.length).toBeGreaterThan(0);
  expect(close.length).toBeGreaterThan(0);
  for (const v of far) {
    // Never quieter than the floor's share of the shot's gain.
    expect(v.gain).toBeGreaterThanOrEqual(AUDIO.distance.floor * shot.gain - 1e-9);
    // Coloured far: low-passed well below the near air, a reverb tail, a softened onset.
    expect(v.lowpass).toBeLessThan(AUDIO.air.near_hz / 4);
    expect(v.wet).toBeGreaterThan(0);
    expect(v.attack).toBeGreaterThan(0);
  }
  for (const v of close) {
    expect(v.gain).toBeCloseTo(shot.gain, 6);
    expect(v.wet).toBeLessThan(far[0].wet / 10);
    expect(v.lowpass).toBeGreaterThan(far[0].lowpass * 4);
  }
});

test("distance never takes a heard sound below the floor, and falls steadily to it", () => {
  let last = Infinity;
  for (let d = 0; d <= AUDIO.distance.max_m; d += 25) {
    const g = distanceGain(AUDIO, d);
    expect(g).toBeGreaterThanOrEqual(AUDIO.distance.floor);
    expect(g).toBeLessThanOrEqual(last);
    last = g;
  }
  expect(distanceGain(AUDIO, 0)).toBe(1);
});

test("weapon row choices change reports without changing observed cadence or hidden cues", () => {
  const catalog = {
    sources: {},
    clips: {},
    sounds: {},
    defaults: {
      carbine: { near: "crisp", far: "crisp_far", gain: 0.8 },
      battle_rifle: { near: "heavy", far: "heavy_far", gain: 1 },
    },
    impacts: {},
    effects: {},
  };
  const sink = new FakeSink();
  const frame = new SoundFrame(
    { tickHz: HZ, presentation: AUDIO, smokeTimes: SMOKE, cookOff: COOK_OFF, catalog },
    sink,
  );
  const shooters = [
    { key: 2, at: [0, 0, 1] },
    { key: 4, at: [5, 0, 1] },
  ];
  run(
    frame,
    sink,
    1,
    4,
    (tick) => {
      const pub = firefight(tick, shooters);
      const weapon = (i: number) => (i ? "battle_rifle" : "carbine");
      return {
        ...pub,
        segments: pub.segments.map((s, i) => ({ ...s, kind: weapon(i), hit: "none" })),
        shooters: pub.shooters.map((s, i) => ({
          ...s,
          mounts: s.mounts.map((m) => ({ ...m, kind: weapon(i) })),
        })),
      };
    },
    () => [{ category: "shot", sector: 1, band: "near", moving: false }],
  );
  const reports = sink.started.filter((v) => v.position !== null && !v.loop);
  expect(reports.map((v) => v.sound)).toEqual(["heavy", "crisp", "heavy", "crisp"]);
  expect(reports.filter((v) => v.sound === "heavy").map((v) => v.at)).toEqual(
    reports.filter((v) => v.sound === "crisp").map((v) => v.at),
  );
  expect(sink.started.find((v) => !v.loop && v.position === null)?.sound).toBe(
    AUDIO.cues.sounds.shot,
  );
});

/** One rifleman with weapon row `kind` at the origin, firing a round on each of `ticks`. */
function rifleman(kind: string, ticks: readonly number[]) {
  return (tick: number): EffectPublication => {
    const pub = firefight(tick, [{ key: 2, at: [0, 0, 1] }]);
    return {
      ...pub,
      shooters: pub.shooters.map((s) => ({
        ...s,
        mounts: s.mounts.map((m) => ({
          ...m,
          kind,
          shots: ticks.filter((t) => t <= tick).length,
        })),
      })),
      // A squad's round leaves its muzzle on the tick after the one that fired it.
      segments: ticks.includes(tick - 1) ? pub.segments.map((s) => ({ ...s, kind })) : [],
    };
  };
}

test("a burst recording sounds once per burst, its shots on the gun's launches", () => {
  const clip = (shots: number) => ({
    burst: {
      shots: Array.from({ length: shots }, (_, k) => [k * 10, k * 10 + 10]),
      interval_s: 0.1,
    },
  });
  const recipe = (clip: string) => ({ clips: [clip], synth: null });
  const catalog = {
    sources: {},
    clips: { three: clip(3) },
    sounds: { burst: recipe("three"), burst_far: recipe("three") },
    defaults: {
      gunner: { near: "burst", far: "burst_far", gain: 1 },
      marksman: { near: "single", far: "single_far", gain: 1 },
    },
    impacts: {},
    effects: {},
  } as unknown as SoundCatalog;
  const heard = (kind: string, ticks: number[]) => {
    const sink = new FakeSink();
    const frame = new SoundFrame(
      { tickHz: HZ, presentation: AUDIO, smokeTimes: SMOKE, cookOff: COOK_OFF, catalog },
      sink,
    );
    run(frame, sink, 1, 70, rifleman(kind, ticks));
    return sink.started
      .filter((v) => v.sound.startsWith(kind === "gunner" ? "burst" : "single"))
      .map((v) => Math.round(v.at / DT));
  };
  // Shots every 0.1 s (3 ticks); each round is heard as its launch is published.
  const bursts = [10, 13, 16, 40, 43, 46];
  // One-shot reports sound once a round...
  expect(heard("marksman", bursts)).toEqual([11, 14, 17, 41, 44, 47]);
  // ...where a 3-shot recording covers each 3-round burst from its first round.
  expect(heard("gunner", bursts)).toEqual([11, 41]);
  // Sustained fire chains recordings: the fourth round starts the next one.
  expect(heard("gunner", [10, 13, 16, 19, 22, 25])).toEqual([11, 20]);
  // A round off the recording's cadence is not hidden inside it.
  expect(heard("gunner", [10, 13, 18])).toEqual([11, 19]);
});

test("a hull's mounts keep their weapons' explicit recipes while implicit impacts follow shared replacements", () => {
  const catalog = {
    sources: {},
    clips: {},
    sounds: {},
    defaults: {
      tank_ap: { near: "cannon", far: "cannon_far", gain: 1 },
      hmg: { near: "roof_report", far: "roof_far", gain: 1 },
    },
    impacts: { ground: { hmg: "dirt_hit" } },
    effects: { cannon: "recorded_cannon", rifle: "recorded_rifle", impact_hull: "hard_hit" },
  };
  const sink = new FakeSink();
  const frame = new SoundFrame(
    { tickHz: HZ, presentation: AUDIO, smokeTimes: SMOKE, cookOff: COOK_OFF, catalog },
    sink,
  );
  const pub = (tick: number): EffectPublication => ({
    tick,
    shooters: [
      {
        key: 2,
        half: [3, 1, 1],
        position: [0, 0, 1],
        yaw: 0,
        members: [],
        mounts: [
          {
            bearing: 0,
            elevation: 0,
            shots: tick - 1,
            kind: "tank_ap",
            muzzle: null,
          },
          { bearing: 0, elevation: 0, shots: tick - 1, kind: "hmg", muzzle: null },
        ],
      },
    ],
    segments:
      tick === 2
        ? [
            {
              path: [
                [0, 0, 1],
                [1, 0, 0],
              ],
              ricochets: [],
              kind: "hmg",
              shooter: null,
              hit: "ground",
              normal: null,
            },
            {
              path: [
                [0, 0, 1],
                [2, 0, 0],
              ],
              ricochets: [],
              kind: "rifle",
              shooter: null,
              hit: "hull",
              normal: null,
            },
          ]
        : [],
    blasts: [],
    smokes: [],
  });
  run(frame, sink, 1, 2, pub);
  expect(
    sink.started
      .filter((v) => !v.loop)
      .map((v) => v.sound)
      .sort(),
  ).toEqual(["cannon", "dirt_hit", "hard_hit", "roof_report"]);
});

test("a glancing round's ricochet follows its kind, heavier rounds louder", () => {
  const catalog = {
    sources: {},
    clips: {},
    sounds: {},
    defaults: {},
    impacts: { ricochet: { tank_ap: "shell_glance" } },
    effects: { ricochet: "bullet_glance" },
  } as unknown as SoundCatalog;
  const glance = (kind: string) => {
    const sink = new FakeSink();
    const frame = new SoundFrame(
      { tickHz: HZ, presentation: AUDIO, smokeTimes: SMOKE, cookOff: COOK_OFF, catalog },
      sink,
    );
    run(frame, sink, 1, 3, (tick) => ({
      tick,
      shooters: [],
      segments:
        tick === 2
          ? [
              {
                path: [
                  [0, -20, 1],
                  [0, -15, 1],
                  [5, -10, 4],
                ],
                ricochets: [{ point: 1, normal: [0, 1, 0] }],
                kind,
                shooter: null,
                hit: "none",
                normal: null,
              },
            ]
          : [],
      blasts: [],
      smokes: [],
    }));
    return sink.started.filter((v) => !v.loop).map((v) => ({ sound: v.sound, gain: v.gain }));
  };
  const shell = glance("tank_ap");
  const bullet = glance("rifle");
  expect(shell.map((v) => v.sound)).toEqual(["shell_glance"]);
  expect(bullet.map((v) => v.sound)).toEqual(["bullet_glance"]);
  // Both glance at the same place: only the round's weight sets the level.
  expect(shell[0].gain / bullet[0].gain).toBeCloseTo(
    AUDIO.impact_scale.tank_ap / (AUDIO.impact_scale.rifle ?? AUDIO.impact_scale.default),
  );
});

test("a missile striking armour sounds the strike and its explosion; tank rounds hit harder", () => {
  /** Shipped sounds heard for one round of `kind` hitting a hull, with or without its blast. */
  const strike = (kind: string, blast: boolean) => {
    const sink = new FakeSink();
    const frame = new SoundFrame(
      {
        tickHz: HZ,
        presentation: AUDIO,
        smokeTimes: SMOKE,
        cookOff: COOK_OFF,
        catalog: gameSounds,
      },
      sink,
    );
    const point = [0, 0, 1.5];
    run(frame, sink, 1, 3, (tick) => ({
      tick,
      shooters: [],
      segments:
        tick === 2
          ? [
              {
                path: [[0, -10, 1.5], point],
                ricochets: [],
                kind,
                shooter: null,
                hit: "hull",
                normal: [0, -1, 0],
              },
            ]
          : [],
      blasts: tick === 2 && blast ? [{ point, radius: 4, kind }] : [],
      smokes: [],
    }));
    return sink.started.filter((v) => !v.loop);
  };
  const missile = strike("atgm", true);
  expect(missile.map((v) => v.sound).sort()).toEqual(["armour-shell-hit", "recorded-explosion"]);
  expect(new Set(missile.map((v) => v.at)).size).toBe(1);
  // The same armour, struck at the same place: a shell lands 2.5 times harder than a rifle round.
  const [shell] = strike("tank_ap", false);
  const [bullet] = strike("rifle", false);
  expect(shell.sound).toBe("armour-shell-hit");
  expect(bullet.sound).toBe("armour-rifle-hit");
  expect(shell.gain / bullet.gain).toBeCloseTo(
    AUDIO.impact_scale.tank_ap / AUDIO.impact_scale.default,
  );
});

test("a cook-off booms as its ammunition goes and clangs where its turret lands", () => {
  const { sink, frame } = setup();
  const landing = [1, 10, 2];
  const hit = 2;
  const ticks = Math.ceil((impactAfter(COOK_OFF) + 0.5) * HZ) + hit;
  run(frame, sink, 1, ticks, (tick) => ({
    tick,
    shooters: [],
    segments: [],
    blasts: [],
    smokes: [],
    cookOffs: tick === hit ? [{ center: [0, 10, 0], height: 2.4, landing }] : [],
  }));
  const heard = sink.started.filter((v) => !v.loop);
  const killed = (hit - 1) * DT;
  const booms = heard.filter((v) => v.sound === AUDIO.cook_off.blast.near);
  // Not with the killing hit: as the ammunition goes, one per fireball.
  expect(booms.length).toBe(COOK_OFF.fireballs.length);
  expect(Math.min(...booms.map((v) => v.at))).toBeGreaterThanOrEqual(
    killed + COOK_OFF.delay_s - DT,
  );
  for (const b of booms) expect(near(b, [0, 10], 3)).toBe(true);
  // The turret striking the deck, where and when it lands.
  const clang = heard.filter((v) => v.sound === AUDIO.impacts.hull.sound);
  expect(clang.length).toBe(1);
  expect(clang[0].at).toBeGreaterThanOrEqual(killed + impactAfter(COOK_OFF) - DT);
  expect(clang[0].at).toBeLessThanOrEqual(killed + impactAfter(COOK_OFF) + DT);
  expect(near(clang[0], landing, 0.5)).toBe(true);
});

test("the turret's landing clang is the catalog's tank shell striking armour", () => {
  const sink = new FakeSink();
  const frame = new SoundFrame(
    { tickHz: HZ, presentation: AUDIO, smokeTimes: SMOKE, cookOff: COOK_OFF, catalog: gameSounds },
    sink,
  );
  const ticks = Math.ceil((impactAfter(COOK_OFF) + 0.5) * HZ) + 2;
  run(frame, sink, 1, ticks, (tick) => ({
    tick,
    shooters: [],
    segments: [],
    blasts: [],
    smokes: [],
    cookOffs: tick === 2 ? [{ center: [0, 10, 0], height: 2.4, landing: [1, 10, 2] }] : [],
  }));
  const last = sink.started.filter((v) => !v.loop).at(-1)!;
  expect(last.sound).toBe(gameSounds.impacts.hull.tank_ap);
});

test("a hovering helicopter keeps sounding its rotor, never a ground vehicle's engine or running gear", () => {
  const { sink, frame } = setup();
  // Airborne, nothing rolls: its travel never moves, whether it hovers or flies.
  const heli = {
    key: 1,
    vehicleClass: "air_light",
    position: [20, 20, 30],
    travelL: 0,
    travelR: 0,
    turret: 0,
  };
  const ground = Object.entries(AUDIO.vehicles).filter(([cls]) => !cls.startsWith("air_"));
  const groundSounds = new Set(ground.flatMap(([, row]) => [row.engine, row.running]));
  for (let tick = 1; tick <= 60; tick++) {
    sink.time = tick * DT;
    frame.update(tick * DT, tick * DT, { vehicles: [heli], soldiers: [] }, LISTENER);
    const rotor = sink.live(true).filter((v) => near(v.spec, heli.position, 1));
    expect(rotor.length, `tick ${tick}`).toBeGreaterThan(0);
    for (const v of rotor) {
      expect(groundSounds.has(v.spec.sound), v.spec.sound).toBe(false);
      expect(v.spec.gain).toBeGreaterThan(0);
    }
  }
});

test("a reversing vehicle beeps at a steady rate and level, whatever its speed", () => {
  const row = AUDIO.vehicles.tracked_heavy;
  const beeper = (load: number) =>
    vehicleLoops(row, load, 0, true).find((l) => l.slot === "reverse");
  // An electronic back-up alarm: creeping or backing fast, the same beep.
  expect(beeper(0.1)).toEqual(beeper(1));
  expect(beeper(1)!.rate).toBe(1);
  // Standing still, or driving forward, it is silent.
  expect(beeper(0)).toBeUndefined();
  expect(vehicleLoops(row, 1, 0, false).some((l) => l.slot === "reverse")).toBe(false);
});

test("the reverse sound beeps: tone, then silence, each period", () => {
  const sampleRate = 48000;
  const sound = synthesize(AUDIO.vehicles.default.reverse!, sampleRate);
  const x = sound.channels[0];
  expect(sound.loop).toBe(true);
  // 10 ms levels: a beep's level is near the peak, the gap between beeps silent.
  const n = sampleRate / 100;
  const levels = Array.from({ length: Math.floor(x.length / n) }, (_, i) =>
    Math.sqrt(x.subarray(i * n, (i + 1) * n).reduce((sum, v) => sum + v * v, 0) / n),
  );
  const top = Math.max(...levels);
  const on = levels.filter((l) => l > top / 2).length / levels.length;
  const off = levels.filter((l) => l < top / 100).length / levels.length;
  expect(on).toBeGreaterThan(0.3);
  expect(off).toBeGreaterThan(0.3);
});
