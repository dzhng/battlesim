import { SoundBank } from "../../../packages/battle-audio/src/soundBank";
import type { SoundCatalog } from "../../../packages/battle-audio/src/catalog";

// A memoryless ceiling preserves safe short attacks; compressor startup can
// attenuate a brief report before its gain has settled.
const AUDITION_LIMIT_CURVE = Float32Array.from({ length: 2049 }, (_, index) =>
  Math.max(-0.98, Math.min(0.98, index / 1024 - 1)),
);

export interface Auditioner {
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

  stop() {
    this.controller?.abort();
    this.controller = null;
    this.bank?.dispose();
    this.bank = null;
    this.source?.stop();
    this.source = null;
    void this.context?.close();
    this.context = null;
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
