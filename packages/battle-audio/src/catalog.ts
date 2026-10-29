import type { NearFar } from "./audioMix";
import { type MusicCatalog } from "./music/musicCatalog";
import { check_sound_catalog } from "../../../web/src/wasm/renderer_browser/renderer_browser.js";
import { loadPresentationModule } from "../../../web/src/presentationModule";
await loadPresentationModule();

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
/** Rust admits source metadata, recipes and routing for every player and tool. */
export function validateSoundCatalog(value: unknown): SoundCatalog {
  check_sound_catalog(JSON.stringify(value));
  return value as SoundCatalog;
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
