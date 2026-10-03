import type { NearFar } from "./audioPresentation";
import { SOUNDS } from "./synth";

export interface SoundSource {
  label: string;
  author: string;
  license: string;
  url: string;
  path: string;
  sha256: string;
  notes: string;
}
export interface SoundClip {
  label: string;
  category: string;
  role: "shot" | "launch" | "impact" | "explosion" | "loop" | "reload" | "mechanical" | "other";
  source: string;
  source_rate: number;
  source_frames: [number, number];
  processing: string;
  url: string;
  sha256: string;
  sample_rate: number;
  frames: number;
  loop: boolean;
  notes: string;
}
export interface SoundRecipe {
  label: string;
  clips: string[];
  synth: string | null;
  synth_gain: number;
  gain: number;
  loop: boolean;
}
export interface ShotChoice {
  near: string;
  far: string;
  gain: number;
}
export interface SoundCatalog {
  sources: Record<string, SoundSource>;
  clips: Record<string, SoundClip>;
  sounds: Record<string, SoundRecipe>;
  defaults: Record<string, ShotChoice>;
  units: Record<string, Record<string, ShotChoice>>;
  impacts: Record<string, Record<string, string>>;
  /** Replacements for baseline effect/loop slots; baseline recipes remain selectable. */
  effects: Record<string, string>;
}
const id = /^[a-zA-Z0-9_-]+$/;
const hash = /^[a-f0-9]{64}$/;
function record(value: unknown, where: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error(`${where}: expected an object`);
  const result = value as Record<string, unknown>;
  if (Object.keys(result).some((k) => ["__proto__", "constructor", "prototype"].includes(k)))
    throw new Error(`${where}: invalid key`);
  return result;
}
function finite(value: unknown, low: number, high: number): boolean {
  return typeof value === "number" && Number.isFinite(value) && value >= low && value <= high;
}
export function validateSoundCatalog(value: unknown): SoundCatalog {
  const root = record(value, "sound catalog");
  for (const section of ["sources", "clips", "sounds", "defaults", "units", "impacts", "effects"])
    record(root[section], section);
  const catalog = value as SoundCatalog;
  for (const [name, source] of Object.entries(catalog.sources)) {
    record(source, `source ${name}`);
    if (
      !id.test(name) ||
      typeof source.label !== "string" ||
      typeof source.author !== "string" ||
      !["CC0-1.0", "Public-Domain-US-Gov"].includes(source.license) ||
      !/^https:\/\//.test(source.url) ||
      !/^assets\/third-party\/audio\/[a-zA-Z0-9_.-]+$/.test(source.path) ||
      !hash.test(source.sha256)
    )
      throw new Error(`source ${name}: invalid provenance`);
  }
  const roles = ["shot", "launch", "impact", "explosion", "loop", "reload", "mechanical", "other"];
  for (const [name, clip] of Object.entries(catalog.clips)) {
    record(clip, `clip ${name}`);
    if (
      !id.test(name) ||
      typeof clip.label !== "string" ||
      typeof clip.category !== "string" ||
      !roles.includes(clip.role) ||
      !Object.hasOwn(catalog.sources, clip.source) ||
      !Array.isArray(clip.source_frames) ||
      clip.source_frames.length !== 2 ||
      !clip.source_frames.every(Number.isSafeInteger) ||
      clip.source_frames[0] < 0 ||
      clip.source_frames[1] <= clip.source_frames[0] ||
      !Number.isSafeInteger(clip.source_rate) ||
      clip.source_rate <= 0 ||
      !Number.isSafeInteger(clip.frames) ||
      clip.frames <= 0 ||
      !Number.isSafeInteger(clip.sample_rate) ||
      clip.sample_rate <= 0 ||
      !hash.test(clip.sha256) ||
      clip.url !== `/audio/clips/${name}.wav` ||
      typeof clip.loop !== "boolean" ||
      typeof clip.processing !== "string"
    )
      throw new Error(`clip ${name}: invalid media or source range`);
  }
  for (const [name, sound] of Object.entries(catalog.sounds)) {
    record(sound, `sound ${name}`);
    if (
      !id.test(name) ||
      typeof sound.label !== "string" ||
      !Array.isArray(sound.clips) ||
      sound.clips.some((c) => typeof c !== "string" || !Object.hasOwn(catalog.clips, c)) ||
      new Set(sound.clips).size !== sound.clips.length ||
      (sound.synth !== null && !Object.hasOwn(SOUNDS, sound.synth)) ||
      (!sound.clips.length && sound.synth === null) ||
      !finite(sound.gain, 0, 2) ||
      !finite(sound.synth_gain, 0, 1) ||
      typeof sound.loop !== "boolean"
    )
      throw new Error(`sound ${name}: invalid recipe or unknown clip`);
    if (
      sound.clips.some((c) => catalog.clips[c].loop !== sound.loop) ||
      (sound.synth !== null && SOUNDS[sound.synth].loop !== sound.loop)
    )
      throw new Error(`sound ${name}: loop mismatch`);
  }
  const firing = (choice: ShotChoice, where: string) => {
    record(choice, where);
    if (!finite(choice.gain, 0, 2)) throw new Error(`${where}: invalid firing gain`);
    for (const name of [choice.near, choice.far]) {
      if (!Object.hasOwn(catalog.sounds, name))
        throw new Error(`${where}: unknown firing sound ${name}`);
      const sound = catalog.sounds[name];
      if (sound.loop) throw new Error(`${where}: firing cannot use a loop`);
      const invalid = sound.clips.find((c) => !["shot", "launch"].includes(catalog.clips[c].role));
      if (invalid)
        throw new Error(
          `${where}: firing cannot use ${catalog.clips[invalid].role} clip ${invalid}`,
        );
    }
  };
  for (const [kind, choice] of Object.entries(catalog.defaults)) firing(choice, `default ${kind}`);
  for (const [unit, mounts] of Object.entries(catalog.units)) {
    record(mounts, `unit ${unit}`);
    for (const [mount, choice] of Object.entries(mounts))
      firing(choice, `unit ${unit} mount ${mount}`);
  }
  for (const [hit, kinds] of Object.entries(catalog.impacts)) {
    record(kinds, `impact ${hit}`);
    for (const [kind, name] of Object.entries(kinds))
      if (!Object.hasOwn(catalog.sounds, name) || catalog.sounds[name].loop)
        throw new Error(`impact ${hit} ${kind}: unknown or looping sound`);
  }
  for (const [slot, name] of Object.entries(catalog.effects)) {
    if (
      !Object.hasOwn(SOUNDS, slot) ||
      !Object.hasOwn(catalog.sounds, name) ||
      catalog.sounds[name].loop !== SOUNDS[slot].loop
    )
      throw new Error(`effect ${slot}: unknown sound or loop mismatch`);
  }
  return catalog;
}

/** Only implicit baseline slots follow shared replacements; explicit choices keep their recipe. */
export function resolveEffect(catalog: SoundCatalog, baseline: string): string {
  return catalog.effects[baseline] ?? baseline;
}
export function resolveShot(
  catalog: SoundCatalog,
  base: NearFar,
  unit: string,
  mount: string,
  kind: string,
): NearFar {
  const choice = catalog.units[unit]?.[mount] ?? catalog.defaults[kind] ?? catalog.defaults.default;
  return choice
    ? { ...base, near: choice.near, far: choice.far, gain: base.gain * choice.gain }
    : base;
}
