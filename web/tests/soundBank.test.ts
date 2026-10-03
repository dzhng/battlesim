// @vitest-environment node
import { expect, test } from "vitest";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { SoundBank } from "@packages/battle-audio/src/webAudioSink";

function buffer(channels: number, length: number, sampleRate: number): AudioBuffer {
  const data = Array.from({ length: channels }, () => new Float32Array(length));
  return {
    numberOfChannels: channels,
    length,
    sampleRate,
    duration: length / sampleRate,
    getChannelData: (channel: number) => data[channel],
    copyToChannel: (source: Float32Array, channel: number) => data[channel].set(source),
  } as unknown as AudioBuffer;
}

function recordingCatalog(): SoundCatalog {
  return {
    sources: {},
    clips: {
      first: { url: "/first.wav", loop: false },
      second: { url: "/second.wav", loop: false },
      reload: { url: "/reload.wav", loop: false },
    } as unknown as SoundCatalog["clips"],
    sounds: {
      report: {
        label: "Report",
        clips: ["first", "second"],
        synth: null,
        synth_gain: 0,
        gain: 0.5,
        loop: false,
      },
    },
    defaults: {},
    units: {},
    impacts: {},
    effects: {},
  };
}

test("recording fetch keeps the browser function's invocation context", async () => {
  const context = {
    sampleRate: 48000,
    createBuffer: buffer,
    async decodeAudioData() {
      return buffer(1, 2, 48000);
    },
  } as unknown as BaseAudioContext;
  const hostFetch = async function (this: unknown) {
    if (this !== undefined && this !== globalThis) throw new TypeError("Illegal invocation");
    return new Response(Uint8Array.of(1));
  };
  const bank = new SoundBank(context, recordingCatalog(), hostFetch);
  await bank.prepare(["report"]);
  expect(bank.get("report").length).toBe(2);
});

test("recorded variations prepare once and preserve the raw unused clip for audition", async () => {
  const decoded = new Map<string, AudioBuffer>();
  const context = {
    sampleRate: 48000,
    createBuffer: buffer,
    async decodeAudioData(bytes: ArrayBuffer) {
      const key = String(new Uint8Array(bytes)[0]);
      const result = buffer(1, 2, 48000);
      result.getChannelData(0).set(key === "1" ? [0.8, -0.4] : [0.2, -0.6]);
      decoded.set(key, result);
      return result;
    },
  } as unknown as BaseAudioContext;
  const fetcher = async (input: RequestInfo | URL) =>
    new Response(Uint8Array.of(String(input).includes("first") ? 1 : 2));
  const bank = new SoundBank(context, recordingCatalog(), fetcher);
  await bank.prepare(["report"]);
  expect(Array.from(bank.get("report", 0).getChannelData(0))).toEqual([
    expect.closeTo(0.4),
    expect.closeTo(-0.2),
  ]);
  expect(Array.from(bank.get("report", 1).getChannelData(0))).toEqual([
    expect.closeTo(0.1),
    expect.closeTo(-0.3),
  ]);
  expect(bank.get("report", 3)).toBe(bank.get("report", 1));
  const raw = await bank.clip("first");
  expect(raw.getChannelData(0)[0]).toBeCloseTo(0.8);
  expect(await bank.clip("first")).toBe(raw);
  const prepared = bank.get("report");
  await bank.prepare(["report"]);
  expect(bank.get("report")).toBe(prepared);
  expect((await bank.clip("reload")).getChannelData(0)[1]).toBeCloseTo(-0.6);
  expect(decoded.get("1")?.getChannelData(0)[0]).toBeCloseTo(0.8);
});

test("an undecoded recording never becomes a ready recipe after cancellation", async () => {
  let finish!: (value: AudioBuffer) => void;
  let decoding!: () => void;
  const begun = new Promise<void>((resolve) => {
    decoding = resolve;
  });
  const context = {
    sampleRate: 48000,
    createBuffer: buffer,
    decodeAudioData() {
      decoding();
      return new Promise<AudioBuffer>((resolve) => {
        finish = resolve;
      });
    },
  } as unknown as BaseAudioContext;
  const catalog = recordingCatalog();
  catalog.sounds.report.clips = ["first"];
  const bank = new SoundBank(context, catalog, async () => new Response(Uint8Array.of(1)));
  const controller = new AbortController();
  const preparing = bank.prepare(["report"], controller.signal);
  // Attach rejection handling before cancellation settles asynchronous decoding.
  const rejected = expect(preparing).rejects.toMatchObject({ name: "AbortError" });
  await begun;
  expect(() => bank.get("report")).toThrow("not prepared");
  controller.abort();
  finish(buffer(1, 2, 48000));
  await rejected;
  expect(() => bank.get("report")).toThrow("not prepared");
});

test("failed decoding is explicit and a later retry can prepare the recipe", async () => {
  let broken = true;
  const context = {
    sampleRate: 48000,
    createBuffer: buffer,
    async decodeAudioData() {
      if (broken) throw new Error("Invalid WAV");
      const result = buffer(1, 2, 48000);
      result.getChannelData(0).set([0.8, -0.4]);
      return result;
    },
  } as unknown as BaseAudioContext;
  const bank = new SoundBank(
    context,
    recordingCatalog(),
    async () => new Response(Uint8Array.of(1)),
  );
  await expect(bank.prepare(["report"])).rejects.toThrow("Invalid WAV");
  expect(() => bank.get("report")).toThrow("not prepared");
  broken = false;
  await bank.prepare(["report"]);
  expect(bank.get("report").getChannelData(0)[0]).toBeCloseTo(0.4);
});

test("disposal aborts fetching and cannot admit a late decoded buffer", async () => {
  let finish!: (value: AudioBuffer) => void;
  let decoding!: () => void;
  let fetchSignal: AbortSignal | undefined;
  const begun = new Promise<void>((resolve) => {
    decoding = resolve;
  });
  const context = {
    sampleRate: 48000,
    createBuffer: buffer,
    decodeAudioData() {
      decoding();
      return new Promise<AudioBuffer>((resolve) => {
        finish = resolve;
      });
    },
  } as unknown as BaseAudioContext;
  const catalog = recordingCatalog();
  catalog.sounds.report.clips = ["first"];
  const bank = new SoundBank(context, catalog, async (_input, init) => {
    fetchSignal = init?.signal ?? undefined;
    return new Response(Uint8Array.of(1));
  });
  const preparing = bank.prepare(["report"]);
  const rejected = expect(preparing).rejects.toMatchObject({ name: "AbortError" });
  await begun;
  bank.dispose();
  expect(fetchSignal?.aborted).toBe(true);
  finish(buffer(1, 2, 48000));
  await rejected;
  expect(() => bank.get("report")).toThrow("not prepared");
});

test("a recorded core and quiet synthesized support share one prepared buffer and retain the longer tail", async () => {
  const { synthesize } = await import("@packages/battle-audio/src/synth");
  const sampleRate = 8000;
  const core = buffer(1, 2, sampleRate);
  core.getChannelData(0).set([0.8, -0.4]);
  const context = {
    sampleRate,
    createBuffer: buffer,
    async decodeAudioData() {
      return core;
    },
  } as unknown as BaseAudioContext;
  const catalog = recordingCatalog();
  catalog.sounds.supported = {
    label: "Supported",
    clips: ["first"],
    synth: "rifle",
    synth_gain: 0.05,
    gain: 0.5,
    loop: false,
  };
  catalog.sounds.rifle = {
    label: "Baseline",
    clips: [],
    synth: "rifle",
    synth_gain: 1,
    gain: 1,
    loop: false,
  };
  catalog.effects.rifle = "supported";
  const bank = new SoundBank(context, catalog, async () => new Response(Uint8Array.of(1)));
  await bank.prepare(["supported", "rifle"]);
  const baseline = synthesize("rifle", sampleRate).channels[0];
  expect(Array.from(bank.get("rifle").getChannelData(0))).toEqual(Array.from(baseline));
  const expected = Float32Array.from(
    baseline,
    (value, i) => 0.5 * ((core.getChannelData(0)[i] ?? 0) + 0.05 * value),
  );
  expect(Array.from(bank.get("supported").getChannelData(0))).toEqual(Array.from(expected));
  expect(bank.get("supported").length).toBe(baseline.length);
});

test("every synthesized baseline retains its exact samples, including stereo ambience", async () => {
  const { SOUNDS, synthesize } = await import("@packages/battle-audio/src/synth");
  const sampleRate = 8000;
  const context = { sampleRate, createBuffer: buffer } as unknown as BaseAudioContext;
  const catalog = recordingCatalog();
  catalog.sounds = Object.fromEntries(
    Object.entries(SOUNDS).map(([name, sound]) => [
      name,
      {
        label: name,
        clips: [],
        synth: name,
        synth_gain: 1,
        gain: 1,
        loop: sound.loop,
      },
    ]),
  );
  const bank = new SoundBank(context, catalog);
  await bank.prepare();
  for (const name of Object.keys(SOUNDS)) {
    const original = synthesize(name, sampleRate);
    const prepared = bank.get(name);
    expect(prepared.numberOfChannels).toBe(original.channels.length);
    for (const [channel, samples] of original.channels.entries()) {
      const actual = prepared.getChannelData(channel);
      expect(
        Buffer.from(actual.buffer, actual.byteOffset, actual.byteLength).equals(
          Buffer.from(samples.buffer, samples.byteOffset, samples.byteLength),
        ),
        `${name} channel ${channel}`,
      ).toBe(true);
    }
  }
});
