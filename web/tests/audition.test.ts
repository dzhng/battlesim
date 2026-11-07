// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";
import { SoundAuditioner } from "../../apps/sound-workbench/src/audition";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { SoundBank } from "@packages/battle-audio/src/soundBank";

const loadPresentation = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("@web/battle/sim/presentation", () => ({ loadPresentation }));

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  loadPresentation.mockReset().mockResolvedValue(undefined);
});

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

test("stale resume and module-load completion cannot replace the current bank", async () => {
  let releaseModule!: () => void;
  const moduleLoading = new Promise<void>((resolve) => (releaseModule = resolve));
  loadPresentation.mockReset().mockReturnValueOnce(moduleLoading).mockResolvedValue(undefined);
  const contexts: { close: ReturnType<typeof vi.fn> }[] = [];
  const started = vi.fn();
  class Context {
    sampleRate = 48000;
    destination = {};
    resume = async () => {};
    close: ReturnType<typeof vi.fn>;
    decodeAudioData: ReturnType<typeof vi.fn>;
    constructor() {
      this.close = vi.fn(async () => {});
      this.decodeAudioData = vi.fn(async () => ({
        numberOfChannels: 1,
        sampleRate: 48000,
        getChannelData: () => Float32Array.from({ length: 4800 }, () => 0.1),
      }));
      contexts.push(this);
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
    vi.fn(async () => ({ ok: true, arrayBuffer: async () => new ArrayBuffer(1) })),
  );
  const dispose = vi.spyOn(SoundBank.prototype, "dispose");
  const clip = vi.spyOn(SoundBank.prototype, "clip");
  const catalog = {
    clips: { report: { url: "/audio/clips/report.wav", loop: false } },
  } as unknown as SoundCatalog;
  const audition = new SoundAuditioner();
  try {
    const first = audition.play(catalog, "clip", "report");
    await Promise.resolve();
    const second = audition.play(catalog, "clip", "report");
    expect(contexts[0]?.close).toHaveBeenCalledOnce();
    releaseModule();
    await Promise.all([first, second]);
    expect(started).toHaveBeenCalledOnce();
  } finally {
    audition.stop();
    expect(dispose).toHaveBeenCalledOnce();
    expect(new Set(dispose.mock.instances)).toEqual(new Set(clip.mock.instances));
  }
});

test("a replacement disposes the old bank before its decode completion", async () => {
  const firstDecode = (() => {
    let resolve!: (buffer: AudioBuffer) => void;
    const promise = new Promise<AudioBuffer>((done) => (resolve = done));
    return { promise, resolve };
  })();
  let decodeStarted!: () => void;
  const decoding = new Promise<void>((resolve) => (decodeStarted = resolve));
  const started = vi.fn();
  const contexts: { close: ReturnType<typeof vi.fn> }[] = [];
  const buffer = () =>
    ({
      numberOfChannels: 1,
      sampleRate: 48000,
      getChannelData: () => Float32Array.from({ length: 4800 }, () => 0.1),
    }) as unknown as AudioBuffer;
  class Context {
    sampleRate = 48000;
    destination = {};
    resume = async () => {};
    close: ReturnType<typeof vi.fn>;
    decodeAudioData: ReturnType<typeof vi.fn>;
    constructor() {
      const index = contexts.length;
      this.close = vi.fn(async () => {});
      this.decodeAudioData = vi.fn(() => {
        if (index === 0) {
          decodeStarted();
          return firstDecode.promise;
        }
        return Promise.resolve(buffer());
      });
      contexts.push({ close: this.close });
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
    vi.fn(async () => ({ ok: true, arrayBuffer: async () => new ArrayBuffer(1) })),
  );
  const dispose = vi.spyOn(SoundBank.prototype, "dispose");
  const clip = vi.spyOn(SoundBank.prototype, "clip");
  const catalog = {
    clips: { report: { url: "/audio/clips/report.wav", loop: false } },
  } as unknown as SoundCatalog;
  const audition = new SoundAuditioner();
  try {
    const first = audition.play(catalog, "clip", "report");
    await decoding;
    const second = audition.play(catalog, "clip", "report");
    expect(dispose.mock.instances[0]).toBe(clip.mock.instances[0]);
    expect(contexts[0]?.close).toHaveBeenCalledOnce();
    firstDecode.resolve(buffer());
    await Promise.all([first, second]);
    expect(started).toHaveBeenCalledOnce();
  } finally {
    audition.stop();
    expect(new Set(dispose.mock.instances)).toEqual(new Set(clip.mock.instances));
  }
});
