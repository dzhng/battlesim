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
} from "@packages/battle-audio/src/soundFrame";
import type {
  EffectPresentation,
  EffectPublication,
  EffectShooter,
} from "@packages/battle-renderer/src/effects/effectFrame";

const HZ = 30;
const DT = 1 / HZ;
const AUDIO = game.presentation.audio as unknown as AudioPresentation;
const SMOKE = (game.presentation.effects as unknown as EffectPresentation).smoke;
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
        units: {},
        impacts: {},
        effects: {},
      },
      smokeTimes: SMOKE,
    },
    sink,
  );
  return { sink, frame };
}

const squad = (key: number, member: number, at: number[], shots: number): EffectShooter => ({
  key,
  kind: "rifle",
  position: at,
  half: null,
  yaw: 0,
  members: [member],
  mounts: [{ name: "rifle", bearing: 0, elevation: 0, shots, kind: "rifle", muzzle: null }],
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
    kind: i % 3 === 0 ? "tank" : i % 3 === 1 ? "supply" : "jeep",
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
    kind: "tank",
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

test("unit and named mount choices change reports without changing observed cadence or hidden cues", () => {
  const catalog = {
    sources: {},
    clips: {},
    sounds: {},
    defaults: {},
    impacts: {},
    effects: {},
    units: {
      scout: { rifle: { near: "crisp", far: "crisp_far", gain: 0.8 } },
      assault: { rifle: { near: "heavy", far: "heavy_far", gain: 1 } },
    },
  };
  const sink = new FakeSink();
  const frame = new SoundFrame(
    { tickHz: HZ, presentation: AUDIO, smokeTimes: SMOKE, catalog },
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
      return {
        ...pub,
        segments: pub.segments.map((s) => ({ ...s, hit: "none" })),
        shooters: pub.shooters.map((s, i) => ({
          ...s,
          kind: i ? "assault" : "scout",
          mounts: s.mounts.map((m) => ({ ...m, name: "rifle" })),
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

/** One rifleman of `kind` at the origin, firing a round on each of `ticks`. */
function rifleman(kind: string, ticks: readonly number[]) {
  return (tick: number): EffectPublication => {
    const pub = firefight(tick, [{ key: 2, at: [0, 0, 1] }]);
    return {
      ...pub,
      shooters: pub.shooters.map((s) => ({
        ...s,
        kind,
        mounts: s.mounts.map((m) => ({ ...m, shots: ticks.filter((t) => t <= tick).length })),
      })),
      // A squad's round leaves its muzzle on the tick after the one that fired it.
      segments: ticks.includes(tick - 1) ? pub.segments : [],
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
    defaults: {},
    impacts: {},
    effects: {},
    units: {
      gunner: { rifle: { near: "burst", far: "burst_far", gain: 1 } },
      marksman: { rifle: { near: "single", far: "single_far", gain: 1 } },
    },
  } as unknown as SoundCatalog;
  const heard = (kind: string, ticks: number[]) => {
    const sink = new FakeSink();
    const frame = new SoundFrame(
      { tickHz: HZ, presentation: AUDIO, smokeTimes: SMOKE, catalog },
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

test("separate named mounts preserve explicit recipes while implicit impacts follow shared replacements", () => {
  const catalog = {
    sources: {},
    clips: {},
    sounds: {},
    defaults: {},
    units: {
      tank: {
        cannon: { near: "cannon", far: "cannon_far", gain: 1 },
        HMG: { near: "roof_report", far: "roof_far", gain: 1 },
      },
    },
    impacts: { ground: { hmg: "dirt_hit" } },
    effects: { cannon: "recorded_cannon", rifle: "recorded_rifle", impact_hull: "hard_hit" },
  };
  const sink = new FakeSink();
  const frame = new SoundFrame(
    { tickHz: HZ, presentation: AUDIO, smokeTimes: SMOKE, catalog },
    sink,
  );
  const pub = (tick: number): EffectPublication => ({
    tick,
    shooters: [
      {
        key: 2,
        kind: "tank",
        half: [3, 1, 1],
        position: [0, 0, 1],
        yaw: 0,
        members: [],
        mounts: [
          {
            name: "cannon",
            bearing: 0,
            elevation: 0,
            shots: tick - 1,
            kind: "tank_ap",
            muzzle: null,
          },
          { name: "HMG", bearing: 0, elevation: 0, shots: tick - 1, kind: "hmg", muzzle: null },
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
    units: {},
    impacts: { ricochet: { tank_ap: "shell_glance" } },
    effects: { ricochet: "bullet_glance" },
  } as unknown as SoundCatalog;
  const glance = (kind: string) => {
    const sink = new FakeSink();
    const frame = new SoundFrame(
      { tickHz: HZ, presentation: AUDIO, smokeTimes: SMOKE, catalog },
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
