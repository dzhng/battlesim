import { SoundBank } from "../../../packages/battle-audio/src/soundBank";
import type { SoundCatalog } from "../../../packages/battle-audio/src/catalog";

// A memoryless ceiling preserves safe short attacks; compressor startup can
// attenuate a brief report before its gain has settled.
const AUDITION_LIMIT_CURVE = Float32Array.from({ length: 2049 }, (_, index) =>
  Math.max(-0.98, Math.min(0.98, index / 1024 - 1)),
);

/** A sounding loop of a live mix, keyed so a slot's level and rate can glide. */
export interface LiveLoop {
  key: string;
  sound: string;
  gain: number;
  rate: number;
}

/** A running audition whose loops follow the controls, and which can play one-shots. */
export interface LiveMix {
  /** Sets each loop's level and rate; a key no longer listed falls silent. */
  loops(layers: readonly LiveLoop[]): void;
  /** Plays a one-shot `delay` seconds from now. */
  hit(sound: string, gain: number, delay: number): void;
}

export interface Auditioner {
  /** Prepares `sounds` and opens a mix that sounds nothing until told. */
  live(catalog: SoundCatalog, sounds: readonly string[]): Promise<LiveMix>;
  play(
    catalog: SoundCatalog,
    kind: "clip" | "sound",
    id: string,
    variant?: number,
    gain?: number,
  ): Promise<void>;
  stop(): void;
}

/** User-triggered audition uses the production bank, including its variation and support mixing. */
export class SoundAuditioner implements Auditioner {
  private context: AudioContext | null = null;
  private controller: AbortController | null = null;
  private bank: SoundBank | null = null;
  private sources = new Set<AudioBufferSourceNode>();

  stop() {
    this.controller?.abort();
    this.controller = null;
    for (const source of this.sources) source.stop();
    this.sources.clear();
    this.bank?.dispose();
    this.bank = null;
    void this.context?.close();
    this.context = null;
  }

  /** A fresh context and bank with `prepare` loaded, and the audition's output:
   *  calibrated level, safety ceiling, mono centred in stereo. */
  private async open<T>(
    catalog: SoundCatalog,
    prepare: (bank: SoundBank, signal: AbortSignal) => Promise<T>,
  ) {
    this.stop();
    const controller = (this.controller = new AbortController());
    const context = (this.context = new AudioContext({ latencyHint: "interactive" }));
    try {
      await context.resume();
      const bank = (this.bank = new SoundBank(context, catalog));
      const prepared = await prepare(bank, controller.signal);
      controller.signal.throwIfAborted();
      const output = new WaveShaperNode(context, { curve: AUDITION_LIMIT_CURVE });
      output.connect(new StereoPannerNode(context, { pan: 0 })).connect(context.destination);
      const level = (buffer: AudioBuffer, gain: number) =>
        0.4 * gain * bank.normalizationGain(buffer);
      /** Sounds `buffer` `delay` seconds from now, from `offset` into it. */
      const start = (buffer: AudioBuffer, gain: number, loop: boolean, delay = 0, offset = 0) => {
        const source = new AudioBufferSourceNode(context, { buffer, loop });
        const volume = new GainNode(context, { gain: level(buffer, gain) });
        source.connect(volume).connect(output);
        source.onended = () => this.sources.delete(source);
        this.sources.add(source);
        source.start(context.currentTime + delay, offset);
        return { source, volume };
      };
      // Whether this audition still sounds: a later one, or a stop, ends it.
      const current = () => this.controller === controller;
      return { context, bank, prepared, level, start, current };
    } catch (error) {
      if (this.controller === controller) this.stop();
      throw error;
    }
  }

  async live(catalog: SoundCatalog, sounds: readonly string[]): Promise<LiveMix> {
    const { context, bank, level, start, current } = await this.open(catalog, (bank, signal) =>
      bank.prepare([...new Set(sounds)], signal),
    );
    const voices = new Map<string, ReturnType<typeof start>>();
    return {
      loops: (layers) => {
        if (!current()) return;
        const now = context.currentTime;
        const wanted = new Set(layers.map((l) => l.key));
        for (const [key, voice] of voices)
          if (!wanted.has(key)) {
            voice.volume.gain.setTargetAtTime(0, now, 0.05);
            voice.source.stop(now + 0.3);
            voices.delete(key);
          }
        for (const layer of layers) {
          const buffer = bank.get(layer.sound);
          let voice = voices.get(layer.key);
          if (!voice) {
            // Silent until the glide below; loops start out of phase, as the battle staggers them.
            voice = start(buffer, 0, true, 0, Math.random() * buffer.duration);
            voices.set(layer.key, voice);
          }
          voice.volume.gain.setTargetAtTime(level(buffer, layer.gain), now, 0.05);
          voice.source.playbackRate.setTargetAtTime(layer.rate, now, 0.05);
        }
      },
      hit: (sound, gain, delay) => {
        if (current())
          start(bank.get(sound, Math.floor(Math.random() * 0x7fffffff)), gain, false, delay);
      },
    };
  }

  async play(catalog: SoundCatalog, kind: "clip" | "sound", id: string, variant = 0, gain = 1) {
    const { prepared, start, current } = await this.open(catalog, (bank, signal) =>
      kind === "clip"
        ? bank.clip(id, signal)
        : bank.prepare([id], signal).then(() => bank.get(id, variant)),
    );
    const loop = kind === "clip" ? catalog.clips[id].loop : catalog.sounds[id].loop;
    const { source } = start(prepared, gain, loop);
    source.onended = () => {
      if (current()) this.stop();
    };
  }
}
