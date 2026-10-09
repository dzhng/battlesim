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
  private source: AudioBufferSourceNode | null = null;
  private bank: SoundBank | null = null;
  private voices = new Map<string, { source: AudioBufferSourceNode; volume: GainNode }>();
  private output: AudioNode | null = null;

  stop() {
    for (const voice of this.voices.values()) voice.source.stop();
    this.voices.clear();
    this.output = null;
    this.controller?.abort();
    this.controller = null;
    this.bank?.dispose();
    this.bank = null;
    this.source?.stop();
    this.source = null;
    void this.context?.close();
    this.context = null;
  }

  async live(catalog: SoundCatalog, sounds: readonly string[]): Promise<LiveMix> {
    this.stop();
    const controller = (this.controller = new AbortController());
    const context = (this.context = new AudioContext({ latencyHint: "interactive" }));
    await context.resume();
    const bank = (this.bank = new SoundBank(context, catalog));
    await bank.prepare([...new Set(sounds)], controller.signal);
    if (controller.signal.aborted) throw new DOMException("Audition stopped", "AbortError");
    const output = (this.output = new WaveShaperNode(context, { curve: AUDITION_LIMIT_CURVE }));
    output.connect(context.destination);
    const level = (buffer: AudioBuffer, gain: number) =>
      0.4 * gain * bank.normalizationGain(buffer);
    return {
      loops: (layers) => {
        if (this.output !== output) return;
        const now = context.currentTime;
        const wanted = new Set(layers.map((l) => l.key));
        for (const [key, voice] of this.voices)
          if (!wanted.has(key)) {
            voice.volume.gain.setTargetAtTime(0, now, 0.05);
            voice.source.stop(now + 0.3);
            this.voices.delete(key);
          }
        for (const layer of layers) {
          const buffer = bank.get(layer.sound);
          let voice = this.voices.get(layer.key);
          if (!voice) {
            const source = new AudioBufferSourceNode(context, { buffer, loop: true });
            const volume = new GainNode(context, { gain: 0 });
            source.connect(volume).connect(output);
            // Loops start out of phase, as the battle staggers them.
            source.start(now, Math.random() * buffer.duration);
            voice = { source, volume };
            this.voices.set(layer.key, voice);
          }
          voice.volume.gain.setTargetAtTime(level(buffer, layer.gain), now, 0.05);
          voice.source.playbackRate.setTargetAtTime(layer.rate, now, 0.05);
        }
      },
      hit: (sound, gain, delay) => {
        if (this.output !== output) return;
        const buffer = bank.get(sound, Math.floor(Math.random() * 0x7fffffff));
        const source = new AudioBufferSourceNode(context, { buffer });
        source.connect(new GainNode(context, { gain: level(buffer, gain) })).connect(output);
        source.start(context.currentTime + Math.max(0, delay));
      },
    };
  }

  async play(catalog: SoundCatalog, kind: "clip" | "sound", id: string, variant = 0, gain = 1) {
    this.stop();
    const controller = (this.controller = new AbortController());
    const context = (this.context = new AudioContext({ latencyHint: "interactive" }));
    try {
      await context.resume();
      const bank = (this.bank = new SoundBank(context, catalog));
      const buffer =
        kind === "clip"
          ? await bank.clip(id, controller.signal)
          : await bank.prepare([id], controller.signal).then(() => bank.get(id, variant));
      if (controller.signal.aborted) return;
      const source = (this.source = new AudioBufferSourceNode(context, {
        buffer,
        loop: kind === "clip" ? catalog.clips[id].loop : catalog.sounds[id].loop,
      }));
      const volume = new GainNode(context, { gain: 0.4 * gain * bank.normalizationGain(buffer) });
      const limiter = new WaveShaperNode(context, { curve: AUDITION_LIMIT_CURVE });
      source
        .connect(volume)
        .connect(limiter)
        .connect(new StereoPannerNode(context, { pan: 0 }))
        .connect(context.destination);
      source.onended = () => {
        if (this.source === source) this.stop();
      };
      source.start();
    } catch (error) {
      if (controller.signal.aborted) return;
      this.stop();
      throw error;
    }
  }
}
