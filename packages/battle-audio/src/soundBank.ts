// Recipes are composed and calibrated in Rust (`presentation::sound::bank`),
// through the presentation module, so the page and the native client prepare
// the same samples; the page decodes the recordings and keeps the buffers.
import {
  RecipeBank,
  max_momentary_loudness,
  source_normalization,
  synthesize as rustSynthesize,
} from "@wasm/renderer_browser";
import { loadPresentation } from "@web/battle/sim/presentation";
import { clipCadence, type SoundCatalog } from "./catalog";
import { SOUNDS } from "./synthSounds.gen";

/** One sound's channels, as the module takes them: its layout and samples. */
function packed(channels: readonly Float32Array[]): [Uint32Array, Float32Array] {
  const length = channels[0]?.length ?? 0;
  const samples = new Float32Array(channels.length * length);
  channels.forEach((c, i) => samples.set(c, i * length));
  return [Uint32Array.of(channels.length, length), samples];
}

const channelsOf = (buffer: AudioBuffer) =>
  Array.from({ length: buffer.numberOfChannels }, (_, c) => buffer.getChannelData(c));

/** Loads the presentation module the bank composes with. */
export const loadSoundModule = loadPresentation;

/** The loudest K-weighted 100 ms window, LUFS: a short-effect reference,
 *  not an integrated programme loudness (Rust's, `sound::bank`). */
export function maxMomentaryLoudness(channels: readonly Float32Array[], sampleRate: number) {
  return max_momentary_loudness(...packed(channels), sampleRate);
}

/** The gain bringing `buffer` to the library's reference, times `authoredGain`. */
export function sourceNormalization(buffer: AudioBuffer, authoredGain = 1): number {
  return source_normalization(...packed(channelsOf(buffer)), buffer.sampleRate, authoredGain);
}

/** Synthesize `name` at `sampleRate`: the same samples every time, on both clients. */
export function synthesize(name: string, sampleRate: number) {
  const out = rustSynthesize(name, sampleRate);
  const count = out[0];
  const length = (out.length - 1) / count;
  return {
    channels: Array.from({ length: count }, (_, c) =>
      out.slice(1 + c * length, 1 + (c + 1) * length),
    ),
    loop: SOUNDS[name].loop,
  };
}

/** Prepared recipe variations and raw recordings for one Web Audio context. */
export class SoundBank {
  private readonly buffers = new Map<string, AudioBuffer[]>();
  private readonly preparing = new Map<string, Promise<void>>();
  private readonly clips = new Map<string, Promise<AudioBuffer>>();
  private composer: RecipeBank | null = null;
  private readonly normalization = new WeakMap<AudioBuffer, number>();
  private readonly controller = new AbortController();
  /** Main-thread milliseconds spent synthesizing and composing buffers. */
  ms = 0;

  constructor(
    private readonly context: BaseAudioContext,
    private readonly catalog: SoundCatalog,
    private readonly fetcher: typeof fetch = fetch,
  ) {}

  /** Prepare named recipes, with a task between recipes. */
  async prepare(names: readonly string[], signal?: AbortSignal) {
    const admitted = this.signal(signal);
    for (const name of names) {
      admitted.throwIfAborted();
      if (!this.buffers.has(name)) {
        let task = this.preparing.get(name);
        if (!task) {
          task = this.make(name, admitted).finally(() => this.preparing.delete(name));
          this.preparing.set(name, task);
        }
        await task;
        await new Promise<void>((resolve) => setTimeout(resolve, 0));
      }
    }
    admitted.throwIfAborted();
  }

  /** A recipe's clip list is a variation pool, never a stack of simultaneous shots. */
  get(name: string, variant = 0): AudioBuffer {
    const choices = this.buffers.get(name);
    if (!choices) throw new Error(`Sound ${name} is not prepared`);
    return choices[(variant >>> 0) % choices.length];
  }

  /** Fixed source calibration; playback applies this separately from gameplay gain. */
  normalizationGain(buffer: AudioBuffer): number {
    const gain = this.normalization.get(buffer);
    if (gain === undefined) throw new Error("Sound buffer is not prepared");
    return gain;
  }

  /** Raw clean clips remain available even when no recipe or assignment uses them. */
  async clip(id: string, signal?: AbortSignal): Promise<AudioBuffer> {
    const admitted = this.signal(signal);
    admitted.throwIfAborted();
    const entry = this.catalog.clips[id];
    if (!entry) throw new Error(`Unknown sound clip ${id}`);
    let loaded = this.clips.get(id);
    if (!loaded) {
      loaded = (async () => {
        const fetcher = this.fetcher;
        const response = await fetcher(entry.url, { signal: admitted });
        if (!response.ok) throw new Error(`Sound clip ${id}: HTTP ${response.status}`);
        const [decoded] = await Promise.all([
          response.arrayBuffer().then((bytes) => this.context.decodeAudioData(bytes)),
          loadPresentation(),
        ]);
        admitted.throwIfAborted();
        const started = performance.now();
        this.normalization.set(decoded, sourceNormalization(decoded));
        this.ms += performance.now() - started;
        return decoded;
      })().catch((error) => {
        this.clips.delete(id);
        throw error;
      });
      this.clips.set(id, loaded);
    }
    const result = await loaded;
    admitted.throwIfAborted();
    return result;
  }

  dispose() {
    this.controller.abort();
    this.buffers.clear();
    this.clips.clear();
    this.composer?.free();
    this.composer = null;
  }

  private signal(signal?: AbortSignal): AbortSignal {
    return signal ? AbortSignal.any([signal, this.controller.signal]) : this.controller.signal;
  }

  private async make(name: string, signal: AbortSignal) {
    const recipe = this.catalog.sounds[name];
    if (!recipe) throw new Error(`Unknown sound recipe ${name}`);
    const [core] = await Promise.all([
      Promise.all(recipe.clips.map((id) => this.clip(id, signal))),
      loadPresentation(),
    ]);
    signal.throwIfAborted();
    const started = performance.now();
    const channels = core.flatMap(channelsOf);
    const length = channels.reduce((n, c) => n + c.length, 0);
    const samples = new Float32Array(length);
    let at = 0;
    for (const c of channels) {
      samples.set(c, at);
      at += c.length;
    }
    const composer = (this.composer ??= new RecipeBank(this.context.sampleRate));
    const out = composer.compose(
      JSON.stringify(recipe),
      Float64Array.from(
        recipe.clips.flatMap((id) => {
          const { shots, interval_s } = clipCadence(this.catalog.clips[id]);
          return [shots, interval_s];
        }),
      ),
      Uint32Array.from(core.flatMap((b) => [b.numberOfChannels, b.length])),
      samples,
    );
    const normalizations = composer.normalizations();
    const buffers: AudioBuffer[] = [];
    for (let o = 0; o < out.length; ) {
      const [count, frames] = [out[o], out[o + 1]];
      o += 2;
      const buffer = this.context.createBuffer(count, frames, this.context.sampleRate);
      for (let c = 0; c < count; c++, o += frames)
        buffer.copyToChannel(out.subarray(o, o + frames) as Float32Array<ArrayBuffer>, c);
      this.normalization.set(buffer, normalizations[buffers.length]);
      buffers.push(buffer);
    }
    this.ms += performance.now() - started;
    signal.throwIfAborted();
    this.buffers.set(name, buffers);
  }
}
