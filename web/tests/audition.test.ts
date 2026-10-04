// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";
import { SoundAuditioner } from "../../apps/sound-workbench/src/audition";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";

afterEach(() => vi.unstubAllGlobals());

test.each([false, true])(
  "a decoded audition starts only if still requested (stopped: %s)",
  async (stopped) => {
    let finish!: (buffer: AudioBuffer) => void;
    let began!: () => void;
    const decoding = new Promise<void>((resolve) => {
      began = resolve;
    });
    const started = vi.fn();
    const close = vi.fn(async () => {});
    class Context {
      sampleRate = 48000;
      destination = {};
      resume = async () => {};
      close = close;
      decodeAudioData() {
        began();
        return new Promise<AudioBuffer>((resolve) => {
          finish = resolve;
        });
      }
    }
    class Node {
      start = started;
      stop = vi.fn();
      connect(node: Node) {
        return node;
      }
    }
    vi.stubGlobal("AudioContext", Context);
    for (const name of ["AudioBufferSourceNode", "GainNode", "WaveShaperNode", "StereoPannerNode"])
      vi.stubGlobal(name, Node);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(Uint8Array.of(1))),
    );
    const catalog = {
      clips: { report: { url: "/audio/clips/report.wav", loop: false } },
    } as unknown as SoundCatalog;
    const audition = new SoundAuditioner();
    try {
      const playing = audition.play(catalog, "clip", "report");
      await decoding;
      if (stopped) audition.stop();
      const samples = Float32Array.from({ length: 4800 }, (_, frame) => 0.1 * Math.sin(frame));
      finish({
        numberOfChannels: 1,
        sampleRate: 48000,
        getChannelData: () => samples,
      } as unknown as AudioBuffer);
      await playing;
      expect(started).toHaveBeenCalledTimes(stopped ? 0 : 1);
      if (stopped) expect(close).toHaveBeenCalledOnce();
    } finally {
      audition.stop();
    }
  },
);
