// @vitest-environment node
import { expect, test } from "vitest";
import units from "@fixtures/catalog.json";
import shipped from "@fixtures/sounds.json";
import {
  battleSounds,
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
  (c.clips.three as { burst: { shots: number[][] } }).burst.shots[2] = [900, 1100];
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

test("a designed clip layers crops of attributed sources; each layer names a real source", () => {
  const c = catalog();
  const source = (license: string) => ({
    label: "Source",
    author: "Author",
    license,
    url: "https://example.com/source",
    path: "assets/third-party/audio/source.mp3",
    sha256: "a".repeat(64),
    notes: "",
  });
  c.sources = { strike: source("CC0-1.0"), whine: source("CC-BY-4.0") };
  const layer = (source: string, at_s: number) => ({
    source,
    source_rate: 44100,
    source_frames: [100, 2000] as [number, number],
    at_s,
    gain: 0.8,
    semitones: -12,
    lowpass_hz: null,
  });
  c.clips.heavy = {
    label: "Heavy ricochet",
    category: "ricochet",
    role: "impact",
    layers: [layer("strike", 0), layer("whine", 0.01)],
    processing: "impact",
    url: "/audio/clips/heavy.wav",
    sha256: "b".repeat(64),
    sample_rate: 48000,
    frames: 4000,
    loop: false,
    notes: "",
  };
  expect(validateSoundCatalog(c)).toBe(c);
  c.sources.whine.license = "Unknown";
  expect(() => validateSoundCatalog(c)).toThrow(/whine: invalid provenance/);
  c.sources.whine.license = "CC-BY-NC-3.0";
  expect(validateSoundCatalog(c)).toBe(c);
  (c.clips.heavy as { layers: { source: string }[] }).layers[1].source = "missing";
  expect(() => validateSoundCatalog(c)).toThrow(/clip heavy/);
});

test("a user-supplied recording retains an explicit reserved-rights receipt", () => {
  const c = catalog();
  c.sources.menu = {
    label: "Battlefield 2 menu",
    author: "EA / DICE",
    license: "All-Rights-Reserved",
    url: "https://www.youtube.com/watch?v=X9ChkYgrBtQ",
    path: "assets/third-party/audio/menu.mp3",
    sha256: "a".repeat(64),
    notes: "User-supplied recording for personal project.",
  };
  expect(validateSoundCatalog(c).sources.menu).toEqual(c.sources.menu);
});

test("a battle prepares the baselines and what is assigned, not the audition-only library", () => {
  const c = catalog();
  const recipe = { label: "r", clips: [], synth: "rifle", synth_gain: 0.5, gain: 1, loop: false };
  for (const name of ["near", "far", "fallback", "hit", "glance", "replacement", "unused"])
    c.sounds[name] = { ...recipe };
  c.units.scout = { rifle: { near: "near", far: "far", gain: 1 } };
  c.defaults.default = { near: "fallback", far: "fallback", gain: 1 };
  c.impacts = { hull: { rifle: "hit" }, ricochet: { tank_ap: "glance" } };
  c.effects = { ricochet: "replacement" };
  const prepared = battleSounds(c);
  expect(prepared).toEqual(expect.arrayContaining(["rifle", "hmg", "near", "far", "fallback"]));
  expect(prepared).toEqual(expect.arrayContaining(["hit", "glance", "replacement"]));
  expect(prepared).not.toContain("unused");
  expect(new Set(prepared).size).toBe(prepared.length);
});
