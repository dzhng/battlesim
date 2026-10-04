// @vitest-environment node
import { expect, test } from "vitest";
import units from "@fixtures/catalog.json";
import shipped from "@fixtures/sounds.json";
import {
  firingCadence,
  resolveShot,
  validateSoundCatalog,
  type SoundCatalog,
  type SoundClip,
} from "@packages/battle-audio/src/catalog";

const catalog = (): SoundCatalog => ({
  sources: {},
  clips: {},
  sounds: {
    rifle: { label: "Synth rifle", clips: [], synth: "rifle", synth_gain: 1, gain: 1, loop: false },
    hmg: { label: "Synth HMG", clips: [], synth: "hmg", synth_gain: 1, gain: 1, loop: false },
  },
  defaults: {},
  units: {},
  impacts: {},
  effects: {},
});

test("two unit types sharing one weapon can select different firing sounds", () => {
  const c = catalog();
  c.units.tank = { HMG: { near: "hmg", far: "hmg", gain: 0.8 } };
  c.units.jeep = { HMG: { near: "rifle", far: "rifle", gain: 0.7 } };
  const base = { near: "rifle", far: "rifle", gain: 0.5, far_m: 350 };
  expect(resolveShot(c, base, "tank", "HMG", "hmg")).toEqual({
    near: "hmg",
    far: "hmg",
    gain: 0.4,
    far_m: 350,
  });
  expect(resolveShot(c, base, "jeep", "HMG", "hmg")).toEqual({
    near: "rifle",
    far: "rifle",
    gain: 0.35,
    far_m: 350,
  });
  expect(validateSoundCatalog(c)).toBe(c);
});

test("an unassigned weapon follows the editable global default before the synth baseline", () => {
  const c = catalog();
  const base = { near: "hmg", far: "hmg", gain: 0.5, far_m: 350 };
  expect(resolveShot(c, base, "new-unit", "new-mount", "new-kind")).toEqual(base);
  c.defaults.default = { near: "rifle", far: "rifle", gain: 0.6 };
  expect(resolveShot(c, base, "new-unit", "new-mount", "new-kind")).toEqual({
    near: "rifle",
    far: "rifle",
    gain: 0.3,
    far_m: 350,
  });
});

test("a firing assignment cannot use a stored reload or a looping sound", () => {
  const c = catalog();
  c.sources.test = {
    label: "Test source",
    author: "Test",
    license: "CC0-1.0",
    url: "https://example.com/audio",
    path: "assets/third-party/audio/test.wav",
    sha256: "a".repeat(64),
    notes: "",
  };
  c.clips.reload = {
    label: "Reload",
    category: "reload",
    role: "reload",
    source: "test",
    source_rate: 48000,
    source_frames: [0, 4800],
    processing: "trim",
    url: "/audio/clips/reload.wav",
    sha256: "b".repeat(64),
    sample_rate: 48000,
    frames: 4800,
    loop: false,
    notes: "",
  };
  c.sounds.reload = {
    label: "Reload",
    clips: ["reload"],
    synth: null,
    synth_gain: 0,
    gain: 1,
    loop: false,
  };
  c.defaults.rifle = { near: "reload", far: "rifle", gain: 1 };
  expect(() => validateSoundCatalog(c)).toThrow(/firing.*reload/i);
  c.sounds.reload = {
    label: "Motor",
    clips: [],
    synth: "motor",
    synth_gain: 1,
    gain: 1,
    loop: true,
  };
  expect(() => validateSoundCatalog(c)).toThrow(/firing.*loop/i);
});

test("a firing sound's alternatives and near/far pair cover the same rounds", () => {
  const c = catalog();
  c.sources.test = {
    label: "Test source",
    author: "Test",
    license: "CC0-1.0",
    url: "https://example.com/source",
    path: "assets/third-party/audio/test.wav",
    sha256: "a".repeat(64),
    notes: "",
  };
  const clip = (name: string, shots: number): SoundClip => ({
    label: name,
    category: "rifle",
    role: "shot",
    source: "test",
    source_rate: 48000,
    source_frames: [0, 1000],
    processing: "weighted-shot",
    url: `/audio/clips/${name}.wav`,
    sha256: "b".repeat(64),
    sample_rate: 48000,
    frames: 1000,
    loop: false,
    notes: "",
    ...(shots > 1 && {
      burst: {
        shots: Array.from({ length: shots }, (_, k) => [k * 100, k * 100 + 100]),
        interval_s: 0.1,
      },
    }),
  });
  c.clips = { one: clip("one", 1), three: clip("three", 3), alt: clip("alt", 3) };
  const recipe = (clips: string[]) => ({
    label: "r",
    clips,
    synth: null,
    synth_gain: 0,
    gain: 1,
    loop: false,
  });
  c.sounds.burst = recipe(["three", "alt"]);
  c.sounds.single = recipe(["one"]);
  c.units.gunner = { rifle: { near: "burst", far: "burst", gain: 1 } };
  expect(validateSoundCatalog(c)).toBe(c);
  expect(firingCadence(c, "burst")).toEqual({ shots: 3, interval_s: 0.1 });
  expect(firingCadence(c, "single").shots).toBe(1);

  c.units.gunner.rifle.far = "single";
  expect(() => validateSoundCatalog(c)).toThrow(/near and far/);
  c.units.gunner.rifle.far = "burst";
  c.sounds.burst.clips = ["three", "one"];
  expect(() => validateSoundCatalog(c)).toThrow(/different bursts/);
  c.sounds.burst.clips = ["three"];
  c.clips.three.burst!.shots[2] = [900, 1100];
  expect(() => validateSoundCatalog(c)).toThrow(/invalid burst/);
});

test("every shipped burst recording fires at its gun's cadence and divides its bursts", () => {
  const sounds = validateSoundCatalog(structuredClone(shipped));
  const weapons = units.weapons as Record<
    string,
    { magazine: { shot_interval_s: number; burst: { rounds: number } | null } | null }
  >;
  const assigned = [
    // `default` names no weapon: it is the fallback for a kind without a row.
    ...Object.entries(sounds.defaults)
      .filter(([kind]) => kind !== "default")
      .map(([kind, choice]) => ({ kinds: [kind], choice })),
    ...units.units.flatMap((unit) =>
      unit.mounts
        .filter((mount) => sounds.units[unit.id]?.[mount.name])
        .map((mount) => ({ kinds: mount.weapons, choice: sounds.units[unit.id][mount.name] })),
    ),
  ];
  let bursts = 0;
  for (const { kinds, choice } of assigned) {
    const burst = firingCadence(sounds, choice.near);
    if (burst.shots === 1) continue;
    bursts++;
    for (const kind of kinds) {
      const magazine = weapons[kind]?.magazine;
      expect(magazine?.shot_interval_s, `${choice.near} on ${kind}`).toBe(burst.interval_s);
      expect((magazine?.burst?.rounds ?? 1) % burst.shots, `${choice.near} on ${kind}`).toBe(0);
    }
  }
  expect(bursts).toBeGreaterThan(0);
});
