// @vitest-environment node
import { existsSync } from "node:fs";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import units from "@fixtures/catalog.json";
import shipped from "@fixtures/sounds.json";
import {
  battleSounds,
  firingCadence,
  recording,
  resolveShot,
  validateSoundCatalog,
  type SoundCatalog,
  type SoundClip,
} from "@packages/battle-audio/src/catalog";
import { musicUrl } from "@packages/battle-audio/src/music/musicCatalog";

const catalog = (): SoundCatalog => ({
  sources: {},
  clips: {},
  sounds: {
    rifle: { label: "Synth rifle", clips: [], synth: "rifle", synth_gain: 1, gain: 1, loop: false },
    hmg: { label: "Synth HMG", clips: [], synth: "hmg", synth_gain: 1, gain: 1, loop: false },
  },
  defaults: {},
  impacts: {},
  effects: {},
  music: { moods: { menu: [], tension: [], combat: [] }, tracks: {} },
});

test("a weapon row's own firing choice wins over the global default", () => {
  const c = catalog();
  c.defaults.default = { near: "rifle", far: "rifle", gain: 0.7 };
  c.defaults.hmg = { near: "hmg", far: "hmg", gain: 0.8 };
  const base = { near: "rifle", far: "rifle", gain: 0.5, far_m: 350 };
  expect(resolveShot(c, base, "hmg")).toEqual({ near: "hmg", far: "hmg", gain: 0.4, far_m: 350 });
  expect(validateSoundCatalog(c)).toBe(c);
});

test("a catalog with a section the game no longer reads is refused, not ignored", () => {
  const c = { ...catalog(), units: { jeep: { HMG: { near: "hmg", far: "hmg", gain: 1 } } } };
  expect(() => validateSoundCatalog(c)).toThrow(/units/);
});

test("an unassigned weapon follows the editable global default before the synth baseline", () => {
  const c = catalog();
  const base = { near: "hmg", far: "hmg", gain: 0.5, far_m: 350 };
  expect(resolveShot(c, base, "new-kind")).toEqual(base);
  c.defaults.default = { near: "rifle", far: "rifle", gain: 0.6 };
  expect(resolveShot(c, base, "new-kind")).toEqual({
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
  c.defaults.rifle = { near: "burst", far: "burst", gain: 1 };
  expect(validateSoundCatalog(c)).toBe(c);
  expect(firingCadence(c, "burst")).toEqual({ shots: 3, interval_s: 0.1 });
  expect(firingCadence(c, "single").shots).toBe(1);

  c.defaults.rifle.far = "single";
  expect(() => validateSoundCatalog(c)).toThrow(/near and far/);
  c.defaults.rifle.far = "burst";
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
  let bursts = 0;
  for (const [kind, choice] of Object.entries(sounds.defaults)) {
    // `default` names no weapon: it is the fallback for a kind without a row.
    if (kind === "default") continue;
    const burst = firingCadence(sounds, choice.near);
    if (burst.shots === 1) continue;
    bursts++;
    const magazine = weapons[kind]?.magazine;
    expect(magazine?.shot_interval_s, `${choice.near} on ${kind}`).toBe(burst.interval_s);
    expect((magazine?.burst?.rounds ?? 1) % burst.shots, `${choice.near} on ${kind}`).toBe(0);
  }
  expect(bursts).toBeGreaterThan(0);
});

test("air-war audio keeps downloaded rights-unassigned receipts beside placeholders", () => {
  const sounds = validateSoundCatalog(structuredClone(shipped));
  const receipts = sounds.provenance ?? {};
  expect(Object.keys(receipts)).toEqual([
    "airwar-jet-approach",
    "airwar-air-alarm",
    "airwar-missile-warning",
    "airwar-flare-deploy",
    "airwar-aa-launch",
  ]);
  for (const receipt of Object.values(receipts)) {
    expect(receipt.license).toBe("All-Rights-Reserved");
    expect(receipt.sha256).toMatch(/^[a-f0-9]{64}$/);
    expect(receipt.download.status).toBe("downloaded-unassigned");
    expect(receipt.path).toMatch(/^assets\/third-party\/audio\//);
    const path = fileURLToPath(new URL(`../../${receipt.path}`, import.meta.url));
    expect(existsSync(path), path).toBe(true);
    expect(createHash("sha256").update(readFileSync(path)).digest("hex")).toBe(receipt.sha256);
    expect(receipt.source_frames[1]).toBeGreaterThan(receipt.source_frames[0]);
  }
  expect(sounds.sounds["airwar-jet-approach-placeholder"]).toMatchObject({
    synth: "rotor",
    loop: true,
  });
  expect(sounds.sounds["airwar-air-alarm-placeholder"]).toMatchObject({
    synth: "cue_engine",
    loop: false,
  });
  expect(
    Object.values(sounds.defaults).flatMap((choice) => [choice.near, choice.far]),
  ).not.toContain("airwar-jet-approach-placeholder");
  expect(battleSounds(sounds)).not.toEqual(
    expect.arrayContaining(["airwar-jet-approach-placeholder", "airwar-air-alarm-placeholder"]),
  );
});

test("air-war audio has workbench recipes for every authored air cue", () => {
  const sounds = validateSoundCatalog(structuredClone(shipped));
  for (const name of ["airwar-missile-warning", "airwar-flare-deploy", "airwar-aa-launch"]) {
    expect(sounds.sounds[name], name).toBeDefined();
    expect(sounds.sounds[name].clips.length, name).toBeGreaterThan(0);
  }
  for (const kind of [
    "stinger",
    "igla_s",
    "fn_6",
    "aa_medium",
    "aa_long",
    "air_missile",
    "sead_missile",
  ]) {
    expect(sounds.defaults[kind]?.near, kind).toBe("airwar-aa-launch");
  }
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
  recording(c, "whine").license = "Unknown";
  expect(() => validateSoundCatalog(c)).toThrow(/whine: invalid provenance/);
  recording(c, "whine").license = "CC-BY-NC-3.0";
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
  c.defaults.rifle = { near: "near", far: "far", gain: 1 };
  c.defaults.default = { near: "fallback", far: "fallback", gain: 1 };
  c.impacts = { hull: { rifle: "hit" }, ricochet: { tank_ap: "glance" } };
  c.effects = { ricochet: "replacement" };
  const prepared = battleSounds(c);
  expect(prepared).toEqual(expect.arrayContaining(["rifle", "hmg", "near", "far", "fallback"]));
  expect(prepared).toEqual(expect.arrayContaining(["hit", "glance", "replacement"]));
  expect(prepared).not.toContain("unused");
  expect(new Set(prepared).size).toBe(prepared.length);
});

test("music tracks name a real source and moods name real tracks; the section is required", () => {
  const c = catalog();
  c.sources.bed = {
    label: "Bed",
    author: "Author",
    license: "CC0-1.0",
    url: "https://example.com/bed",
    path: "assets/third-party/audio/bed.mp3",
    sha256: "a".repeat(64),
    notes: "",
  };
  c.music.tracks.bed = { source: "bed", sha256: "c".repeat(64), gain: 0.5 };
  c.music.moods.menu = ["bed"];
  expect(validateSoundCatalog(c)).toBe(c);
  c.music.moods.combat = ["missing"];
  expect(() => validateSoundCatalog(c)).toThrow(/mood combat/);
  c.music.moods.combat = [];
  c.music.tracks.bed.source = "missing";
  expect(() => validateSoundCatalog(c)).toThrow(/music track bed/);
  const { music: _, ...withoutMusic } = catalog();
  expect(() => validateSoundCatalog(withoutMusic)).toThrow(/music/);
});

test("a generated track records how it was made; only a recording can be cropped into a clip", () => {
  const c = catalog();
  c.sources.combat_1 = {
    kind: "Generated",
    generator: "ElevenLabs music",
    model: "music_v1",
    prompt: "Background underscore",
    length_s: 150,
    date: "2026-10-12",
    plan: "Duet ElevenLabs",
    path: "assets/source/audio/music/combat_1.0a1b2c3d.mp3",
    sha256: "a".repeat(64),
  };
  c.music.tracks.combat_1 = { source: "combat_1", sha256: "c".repeat(64), gain: 0.3 };
  c.music.moods.combat = ["combat_1"];
  expect(validateSoundCatalog(c)).toBe(c);
  c.clips.cut = {
    label: "Cut",
    category: "music",
    role: "other",
    processing: "impact",
    url: "/audio/clips/cut.wav",
    sha256: "b".repeat(64),
    sample_rate: 48000,
    frames: 4000,
    loop: false,
    notes: "",
    source: "combat_1",
    source_rate: 44100,
    source_frames: [0, 4000],
  };
  expect(() => validateSoundCatalog(c)).toThrow(/clip cut/);
  delete c.clips.cut;
  c.sources.combat_1 = { ...c.sources.combat_1, path: "assets/third-party/audio/x.mp3" };
  expect(() => validateSoundCatalog(c)).toThrow(/combat_1: invalid provenance/);
  c.sources.combat_1 = { ...c.sources.combat_1, path: "assets/source/audio/music/x.mp3" };
  delete (c.sources.combat_1 as { prompt?: string }).prompt;
  expect(() => validateSoundCatalog(c)).toThrow(/combat_1: invalid provenance/);
});

test("every shipped music track is served at <track>.<sha8>.mp3 and the menu has a bed", () => {
  const sounds = validateSoundCatalog(structuredClone(shipped));
  expect(sounds.music.moods.menu.length).toBeGreaterThan(0);
  for (const [id, track] of Object.entries(sounds.music.tracks)) {
    const served = fileURLToPath(
      new URL(`../../assets/runtime${musicUrl(id, track)}`, import.meta.url),
    );
    expect(existsSync(served), served).toBe(true);
  }
});
