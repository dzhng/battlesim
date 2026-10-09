import { clipCadence, type SoundCatalog, type SoundRecipe } from "./catalog";
import { seamless, synthesize } from "./synth";
import { sourceNormalization } from "./loudness";

/** Prepared recipe variations and raw recordings for one Web Audio context. */
export class SoundBank {
  private readonly buffers = new Map<string, AudioBuffer[]>();
  private readonly preparing = new Map<string, Promise<void>>();
  private readonly clips = new Map<string, Promise<AudioBuffer>>();
  private readonly synths = new Map<string, AudioBuffer>();
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
        const decoded = await this.context.decodeAudioData(await response.arrayBuffer());
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
    this.synths.clear();
  }

  private signal(signal?: AbortSignal): AbortSignal {
    return signal ? AbortSignal.any([signal, this.controller.signal]) : this.controller.signal;
  }

  private synth(name: string): AudioBuffer {
    let buffer = this.synths.get(name);
    if (!buffer) {
      const started = performance.now();
      const sound = synthesize(name, this.context.sampleRate);
      buffer = this.context.createBuffer(
        sound.channels.length,
        sound.channels[0].length,
        this.context.sampleRate,
      );
      sound.channels.forEach((channel, i) =>
        buffer!.copyToChannel(channel as Float32Array<ArrayBuffer>, i),
      );
      this.synths.set(name, buffer);
      this.ms += performance.now() - started;
    }
    return buffer;
  }

  private async make(name: string, signal: AbortSignal) {
    const recipe = this.catalog.sounds[name];
    if (!recipe) throw new Error(`Unknown sound recipe ${name}`);
    const core = await Promise.all(recipe.clips.map((id) => this.clip(id, signal)));
    signal.throwIfAborted();
    const support = recipe.synth === null ? null : this.synth(recipe.synth);
    const choices = core.length ? core : [null];
    const started = performance.now();
    const buffers = choices.map((clip, i) => {
      const { shots, interval_s } = clipCadence(
        clip ? this.catalog.clips[recipe.clips[i]] : undefined,
      );
      const offsets = Array.from({ length: shots }, (_, k) =>
        Math.round(k * interval_s * this.context.sampleRate),
      );
      return this.mix(recipe, clip, support, offsets);
    });
    const authoredGain = recipe.gain * (core.length ? 1 : recipe.synth_gain);
    for (const buffer of buffers)
      this.normalization.set(buffer, sourceNormalization(buffer, authoredGain));
    this.ms += performance.now() - started;
    signal.throwIfAborted();
    this.buffers.set(name, buffers);
  }

  private mix(
    recipe: SoundRecipe,
    core: AudioBuffer | null,
    support: AudioBuffer | null,
    shots: readonly number[],
  ): AudioBuffer {
    // Baseline synthesis keeps its original bytes, including signed zero.
    if (!core && support && recipe.gain === 1 && recipe.synth_gain === 1) return support;
    const channels = Math.max(core?.numberOfChannels ?? 0, support?.numberOfChannels ?? 0);
    const frames = Math.max(core?.length ?? 0, shots.at(-1)! + (support?.length ?? 0));
    const mixed = this.context.createBuffer(channels, frames, this.context.sampleRate);
    for (let channel = 0; channel < channels; channel++) {
      const out = mixed.getChannelData(channel);
      const recorded = core?.getChannelData(channel % core.numberOfChannels);
      const synth = support?.getChannelData(channel % support.numberOfChannels);
      if (recipe.loop && recorded && synth && recorded.length !== synth.length) {
        // Repeat each layer at its own period, then soften the composed loop's seam.
        out.set(
          seamless(frames, Math.min(frames, Math.round(0.05 * this.context.sampleRate)), (n) =>
            Float32Array.from(
              { length: n },
              (_, frame) =>
                recipe.gain *
                (recorded[frame % recorded.length] +
                  recipe.synth_gain * synth[frame % synth.length]),
            ),
          ),
        );
      } else {
        // Support sounds under each shot: once for a single report.
        if (recorded) out.set(recorded);
        if (synth)
          for (const at of shots)
            for (let frame = 0; frame < synth.length; frame++)
              out[at + frame] += recipe.synth_gain * synth[frame];
        for (let frame = 0; frame < frames; frame++) out[frame] *= recipe.gain;
      }
    }
    return mixed;
  }
}
