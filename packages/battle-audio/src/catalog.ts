import type { NearFar } from "./audioPresentation";
import { MOODS, type MusicCatalog } from "./music/musicCatalog";
import { SOUNDS } from "./synthSounds.gen";

export interface Generated {
  kind: "Generated";
  generator: string;
  model: string;
  text?: string;
  prompt?: string;
  voice_id?: string;
  voice_description?: string;
  date: string;
  plan: "Duet ElevenLabs";
}
/** A generated master in the library: how it was made, where its raw bytes live. */
export interface GeneratedSource extends Generated {
  /** What the generator was asked for. */
  prompt: string;
  /** The requested length, part of what the master is a function of. */
  length_s: number;
  /** The raw master, under `assets/source/audio/`. */
  path: string;
  sha256: string;
}
/** A pinned third-party recording, cut into clips like a generated sound effect. */
export interface SoundSource {
  label: string;
  author: string;
  license: string;
  url: string;
  path: string;
  sha256: string;
  notes: string;
}
/**
 * A source candidate kept out of the playable source bank. A downloaded
 * candidate records its pinned bytes and path, but remains unassigned until
 * its rights and crop are accepted.
 */
export interface SourceProvenance {
  label: string;
  author: string;
  license: string;
  url: string;
  sha256: string | null;
  source_frames: [number, number];
  processing: string;
  download: {
    tool: string;
    version: string;
    status: "candidate-not-downloaded" | "downloaded-unassigned";
  };
  /** Present only after the candidate bytes have been downloaded. */
  path?: string;
  notes: string;
}
/** Source receipts stay explicit, including user-supplied reserved-rights media. */
const LICENSES = [
  "CC0-1.0",
  "Public-Domain-US-Gov",
  "CC-BY-3.0",
  "CC-BY-4.0",
  "CC-BY-NC-3.0",
  "All-Rights-Reserved",
] as const;
/** A span of one pinned recording, in its own frames. */
export interface SourceCrop {
  source: string;
  source_rate: number;
  source_frames: [number, number];
}
/** One layer of a designed clip: a crop placed `at_s` in, scaled, pitched by
 *  resampling (lower is also slower) and optionally low-passed. */
export interface Layer extends SourceCrop {
  at_s: number;
  gain: number;
  semitones: number;
  lowpass_hz: number | null;
}
interface ClipMedia {
  label: string;
  category: string;
  role: "shot" | "launch" | "impact" | "explosion" | "loop" | "reload" | "mechanical" | "other";
  processing: string;
  url: string;
  sha256: string;
  sample_rate: number;
  frames: number;
  loop: boolean;
  notes: string;
}
/** A recording's crop, optionally a burst of its shots, or a design layering several crops. */
export type SoundClip = ClipMedia &
  (
    | (SourceCrop & {
        /** A recorded burst: these source shots, re-laid `interval_s` apart. */ burst?: Burst;
      })
    | { layers: Layer[] }
  );
/** Shots one recording plays, at the gun's cadence; single reports have none. */
export interface Burst {
  /** Each shot's source frames: its attack to the end of its retained tail. */
  shots: [number, number][];
  interval_s: number;
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
  sources: Record<string, SoundSource | GeneratedSource>;
  /** Selected external receipts kept out of the playable source bank. */
  provenance?: Record<string, SourceProvenance>;
  clips: Record<string, SoundClip>;
  sounds: Record<string, SoundRecipe>;
  /** Firing choices by weapon row; `default` for a row without one. */
  defaults: Record<string, ShotChoice>;
  impacts: Record<string, Record<string, string>>;
  /** Replacements for baseline effect/loop slots; baseline recipes remain selectable. */
  effects: Record<string, string>;
  music: MusicCatalog;
}
export const isGenerated = (source: SoundSource | GeneratedSource): source is GeneratedSource =>
  (source as GeneratedSource).kind === "Generated";
/** Where generated sound-effect masters are kept; what makes a generated source cuttable. */
export const EFFECT_MASTERS = "assets/source/audio/effects";
/** Whether clips may be cut from `source`: a recording or a generated sound effect; a
 *  generated music master streams whole and is never cut. */
export const cuttable = (source: SoundSource | GeneratedSource) =>
  !isGenerated(source) || source.path.startsWith(`${EFFECT_MASTERS}/`);
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

function validateSources(catalog: SoundCatalog) {
  for (const [name, source] of Object.entries(catalog.sources)) {
    record(source, `source ${name}`);
    if (
      !id.test(name) ||
      !hash.test(source.sha256) ||
      !(isGenerated(source)
        ? typeof source.generator === "string" &&
          typeof source.model === "string" &&
          typeof source.prompt === "string" &&
          finite(source.length_s, 1, 600) &&
          /^\d{4}-\d{2}-\d{2}$/.test(source.date) &&
          source.plan === "Duet ElevenLabs" &&
          /^assets\/source\/audio\/(music|effects)\/[a-zA-Z0-9_.-]+$/.test(source.path)
        : typeof source.label === "string" &&
          typeof source.author === "string" &&
          (LICENSES as readonly string[]).includes(source.license) &&
          source.url.startsWith("https://") &&
          /^assets\/third-party\/audio\/[a-zA-Z0-9_.-]+$/.test(source.path))
    )
      throw new Error(`source ${name}: invalid provenance`);
  }
}

function validateProvenance(catalog: SoundCatalog) {
  for (const [name, source] of Object.entries(catalog.provenance ?? {})) {
    record(source, `provenance ${name}`);
    const frames = source.source_frames;
    if (
      !id.test(name) ||
      typeof source.label !== "string" ||
      typeof source.author !== "string" ||
      !(LICENSES as readonly string[]).includes(source.license) ||
      !source.url.startsWith("https://") ||
      (source.sha256 !== null && !hash.test(source.sha256)) ||
      !Array.isArray(frames) ||
      frames.length !== 2 ||
      !frames.every(Number.isSafeInteger) ||
      frames[0] < 0 ||
      frames[1] <= frames[0] ||
      typeof source.processing !== "string" ||
      source.processing.length === 0 ||
      !source.download ||
      typeof source.download.tool !== "string" ||
      typeof source.download.version !== "string" ||
      !["candidate-not-downloaded", "downloaded-unassigned"].includes(source.download.status) ||
      (source.download.status === "candidate-not-downloaded" &&
        (source.sha256 !== null || source.path !== undefined)) ||
      (source.download.status === "downloaded-unassigned" &&
        (source.sha256 === null ||
          !hash.test(source.sha256) ||
          typeof source.path !== "string" ||
          !/^assets\/third-party\/audio\/[a-zA-Z0-9_.-]+$/.test(source.path))) ||
      typeof source.notes !== "string"
    )
      throw new Error(`provenance ${name}: invalid source receipt`);
  }
}

function validateClips(catalog: SoundCatalog) {
  const roles = ["shot", "launch", "impact", "explosion", "loop", "reload", "mechanical", "other"];
  const crop = (c: SourceCrop) =>
    Object.hasOwn(catalog.sources, c.source) &&
    cuttable(catalog.sources[c.source]) &&
    Array.isArray(c.source_frames) &&
    c.source_frames.length === 2 &&
    c.source_frames.every(Number.isSafeInteger) &&
    c.source_frames[0] >= 0 &&
    c.source_frames[1] > c.source_frames[0] &&
    Number.isSafeInteger(c.source_rate) &&
    c.source_rate > 0;
  for (const [name, clip] of Object.entries(catalog.clips)) {
    record(clip, `clip ${name}`);
    if (
      !id.test(name) ||
      typeof clip.label !== "string" ||
      typeof clip.category !== "string" ||
      !roles.includes(clip.role) ||
      !("layers" in clip
        ? Array.isArray(clip.layers) &&
          clip.layers.length > 0 &&
          clip.layers.every(
            (l) =>
              crop(l) &&
              finite(l.at_s, 0, 10) &&
              finite(l.gain, 0, 4) &&
              finite(l.semitones, -36, 36) &&
              (l.lowpass_hz === null || finite(l.lowpass_hz, 20, 24000)),
          )
        : crop(clip)) ||
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
    const burst = "burst" in clip ? clip.burst : undefined;
    if (
      burst !== undefined &&
      ("layers" in clip ||
        !(clip.role === "shot" && !clip.loop) ||
        !Array.isArray(burst.shots) ||
        burst.shots.length < 2 ||
        !burst.shots.every(
          (shot) =>
            Array.isArray(shot) &&
            shot.length === 2 &&
            shot.every(Number.isSafeInteger) &&
            clip.source_frames[0] <= shot[0] &&
            shot[0] < shot[1] &&
            shot[1] <= clip.source_frames[1],
        ) ||
        !finite(burst.interval_s, 0.02, 1))
    )
      throw new Error(`clip ${name}: invalid burst`);
  }
}

function validateSounds(catalog: SoundCatalog) {
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
    const first = clipCadence(catalog.clips[sound.clips[0]]);
    if (sound.clips.some((c) => !same(clipCadence(catalog.clips[c]), first)))
      throw new Error(`sound ${name}: alternatives play different bursts`);
    if (
      sound.clips.some((c) => catalog.clips[c].loop !== sound.loop) ||
      (sound.synth !== null && SOUNDS[sound.synth].loop !== sound.loop)
    )
      throw new Error(`sound ${name}: loop mismatch`);
  }
}

function validateRouting(catalog: SoundCatalog) {
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
    // Near or far is chosen per voice; which rounds a voice covers is not.
    if (!same(firingCadence(catalog, choice.near), firingCadence(catalog, choice.far)))
      throw new Error(`${where}: near and far play different bursts`);
  };
  for (const [kind, choice] of Object.entries(catalog.defaults)) firing(choice, `default ${kind}`);
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
}

function validateMusic(catalog: SoundCatalog) {
  const { moods, tracks } = catalog.music;
  record(tracks, "music tracks");
  for (const [name, track] of Object.entries(tracks)) {
    record(track, `music track ${name}`);
    if (
      !id.test(name) ||
      !Object.hasOwn(catalog.sources, track.source) ||
      !hash.test(track.sha256) ||
      !finite(track.gain, 0, 2)
    )
      throw new Error(`music track ${name}: invalid track or unknown source`);
  }
  record(moods, "music moods");
  const unknownMood = Object.keys(moods).find(
    (mood) => !(MOODS as readonly string[]).includes(mood),
  );
  if (unknownMood) throw new Error(`music: unknown mood ${unknownMood}`);
  for (const mood of MOODS)
    if (!Array.isArray(moods[mood]) || moods[mood].some((t) => !Object.hasOwn(tracks, t)))
      throw new Error(`music mood ${mood}: missing or unknown track`);
}

export function validateSoundCatalog(value: unknown): SoundCatalog {
  const root = record(value, "sound catalog");
  const sections = [
    "sources",
    "provenance",
    "clips",
    "sounds",
    "defaults",
    "impacts",
    "effects",
    "music",
  ];
  for (const section of sections) {
    if (section === "provenance" && root[section] === undefined) continue;
    record(root[section], section);
  }
  const unknown = Object.keys(root).find((key) => !sections.includes(key));
  if (unknown) throw new Error(`sound catalog: unknown section ${unknown}`);
  const catalog = value as SoundCatalog;
  validateSources(catalog);
  validateProvenance(catalog);
  validateClips(catalog);
  validateSounds(catalog);
  validateRouting(catalog);
  validateMusic(catalog);
  return catalog;
}

/** The rounds one firing voice covers, `interval_s` apart. */
export interface Cadence {
  shots: number;
  interval_s: number;
}
/** The rounds one clip sounds: its burst's shots, or one. */
export const clipCadence = (clip: SoundClip | undefined): Cadence =>
  clip && "burst" in clip && clip.burst
    ? { shots: clip.burst.shots.length, interval_s: clip.burst.interval_s }
    : { shots: 1, interval_s: 0 };
const same = (a: Cadence, b: Cadence) => a.shots === b.shots && a.interval_s === b.interval_s;

/** A firing sound's cadence: its recordings' burst, or one round for single
 *  reports and synthesis. A recipe's alternatives agree. */
export function firingCadence(catalog: SoundCatalog, sound: string): Cadence {
  return clipCadence(catalog.clips[catalog.sounds[sound]?.clips[0]]);
}

/** Only implicit baseline slots follow shared replacements; explicit choices keep their recipe. */
export function resolveEffect(catalog: SoundCatalog, baseline: string): string {
  return catalog.effects[baseline] ?? baseline;
}
/** How a round of weapon row `kind` fires: its chosen recording, else the
 *  `default` choice, else the synthesized `base` (with effect replacements). */
export function resolveShot(catalog: SoundCatalog, base: NearFar, kind: string): NearFar {
  const choice = catalog.defaults[kind] ?? catalog.defaults.default;
  if (choice) return { ...base, near: choice.near, far: choice.far, gain: base.gain * choice.gain };
  const near = resolveEffect(catalog, base.near);
  const far = resolveEffect(catalog, base.far);
  return near === base.near && far === base.far ? base : { ...base, near, far };
}
