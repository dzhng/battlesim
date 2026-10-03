// @vitest-environment node
import { expect, test } from "vitest";
import {
  resolveShot,
  validateSoundCatalog,
  type SoundCatalog,
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
  expect(resolveShot(c, base, "new-unit", "new-mount", "new-kind")).toBe(base);
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
