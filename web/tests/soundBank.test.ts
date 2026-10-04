// @vitest-environment node
import { expect, test } from "vitest";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { SoundBank } from "@packages/battle-audio/src/soundBank";

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

test("source calibration equalizes quiet and loud reports while preserving recipe volume and stored samples", async () => {
  const sampleRate = 48000;
  const decoded = [0.02, 0.3].map((amplitude) => {
    const result = buffer(1, 2400, sampleRate);
    result
      .getChannelData(0)
      .set(
        Float32Array.from(
          { length: 2400 },
          (_, i) => amplitude * Math.sin((2 * Math.PI * 1000 * i) / sampleRate),
        ),
      );
    return result;
  });
  const context = {
    sampleRate,
    createBuffer: buffer,
    async decodeAudioData(bytes: ArrayBuffer) {
      return decoded[new Uint8Array(bytes)[0]];
    },
  } as unknown as BaseAudioContext;
  const catalog = recordingCatalog();
  const bank = new SoundBank(
    context,
    catalog,
    async (input) => new Response(Uint8Array.of(String(input).includes("first") ? 0 : 1)),
  );
  await bank.prepare(["report"]);
  const level = (audio: AudioBuffer) => {
    const gain = bank.normalizationGain(audio);
    const samples = audio.getChannelData(0);
    return Math.sqrt(
      samples.reduce((sum, sample) => sum + (sample * gain) ** 2, 0) / samples.length,
    );
  };
  const quiet = await bank.clip("first");
  const loud = await bank.clip("second");
  expect(level(quiet)).toBeCloseTo(level(loud), 6);
  expect(level(quiet)).toBeGreaterThan(0.05);
  expect(level(bank.get("report", 0))).toBeCloseTo(level(quiet) * 0.5, 6);
  expect(level(bank.get("report", 1))).toBeCloseTo(level(loud) * 0.5, 6);
  expect(quiet).toBe(decoded[0]);
  expect(loud).toBe(decoded[1]);
  expect(Math.max(...quiet.getChannelData(0))).toBeCloseTo(0.02);
});

test("calibration preserves the authored level of a synthesis-only custom recipe", async () => {
  const context = { sampleRate: 48000, createBuffer: buffer } as unknown as BaseAudioContext;
  const catalog = recordingCatalog();
  catalog.sounds = {
    baseline: { label: "Baseline", clips: [], synth: "rifle", synth_gain: 1, gain: 1, loop: false },
    quiet: { label: "Quiet", clips: [], synth: "rifle", synth_gain: 0.2, gain: 0.25, loop: false },
  };
  const bank = new SoundBank(context, catalog);
  await bank.prepare();
  const peak = (name: string) => {
    const audio = bank.get(name);
    return Math.max(...audio.getChannelData(0).map(Math.abs)) * bank.normalizationGain(audio);
  };
  expect(peak("quiet") / peak("baseline")).toBeCloseTo(0.2 * 0.25, 6);
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

test.each(["caller abort", "bank disposal"])(
  "late decoding cannot admit a recipe after %s",
  async (cancel) => {
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
    const controller = new AbortController();
    const preparing = bank.prepare(
      ["report"],
      cancel === "caller abort" ? controller.signal : undefined,
    );
    // Attach rejection handling before cancellation settles asynchronous decoding.
    const rejected = expect(preparing).rejects.toMatchObject({ name: "AbortError" });
    await begun;
    expect(() => bank.get("report")).toThrow("not prepared");
    if (cancel === "caller abort") controller.abort();
    else {
      bank.dispose();
      expect(fetchSignal?.aborted).toBe(true);
    }
    finish(buffer(1, 2, 48000));
    await rejected;
    expect(() => bank.get("report")).toThrow("not prepared");
  },
);

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

test("a recorded core and synthesized support retain the tail without corrupting the same bank’s baseline", async () => {
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
  // Preparing a mixed recipe must not mutate the bank’s cached synthesis.
  expect(Array.from(bank.get("rifle").getChannelData(0))).toEqual(Array.from(baseline));
  const expected = Float32Array.from(
    baseline,
    (value, i) => 0.5 * ((core.getChannelData(0)[i] ?? 0) + 0.05 * value),
  );
  expect(Array.from(bank.get("supported").getChannelData(0))).toEqual(Array.from(expected));
  expect(bank.get("supported").length).toBe(baseline.length);
});

test("synthesized support sounds under every shot of a recorded burst", async () => {
  const { synthesize } = await import("@packages/battle-audio/src/synth");
  const sampleRate = 8000;
  // A three-shot recording at 0.1 s, silent but for its length.
  const core = buffer(1, 4000, sampleRate);
  const context = {
    sampleRate,
    createBuffer: buffer,
    async decodeAudioData() {
      return core;
    },
  } as unknown as BaseAudioContext;
  const catalog = recordingCatalog();
  (catalog.clips.first as { burst: unknown }).burst = {
    shots: [
      [0, 1],
      [1, 2],
      [2, 3],
    ],
    interval_s: 0.1,
  };
  catalog.sounds.supported = {
    label: "Supported",
    clips: ["first"],
    synth: "rifle",
    synth_gain: 0.5,
    gain: 1,
    loop: false,
  };
  const bank = new SoundBank(context, catalog, async () => new Response(Uint8Array.of(1)));
  await bank.prepare(["supported"]);
  const report = synthesize("rifle", sampleRate).channels[0];
  const mixed = bank.get("supported").getChannelData(0);
  const step = 0.1 * sampleRate;
  const expected = new Float32Array(Math.max(4000, 2 * step + report.length));
  for (let k = 0; k < 3; k++) report.forEach((v, i) => (expected[k * step + i] += 0.5 * v));
  expect(mixed.length).toBe(expected.length);
  for (let i = 0; i < expected.length; i++) expect(mixed[i]).toBeCloseTo(expected[i], 6);
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

test("a shorter recorded loop keeps its body throughout a longer synthesized layer", async () => {
  const sampleRate = 8000;
  const core = buffer(1, 800, sampleRate);
  core.getChannelData(0).fill(0.4);
  const context = {
    sampleRate,
    createBuffer: buffer,
    async decodeAudioData() {
      return core;
    },
  } as unknown as BaseAudioContext;
  const catalog = recordingCatalog();
  catalog.sounds.motor = {
    label: "Motor",
    clips: ["first"],
    synth: "motor",
    synth_gain: 0.01,
    gain: 1,
    loop: true,
  };
  const bank = new SoundBank(context, catalog, async () => new Response(Uint8Array.of(1)));
  await bank.prepare(["motor"]);
  const samples = bank.get("motor").getChannelData(0);
  expect(samples.length).toBeGreaterThan(core.length);
  expect(Math.min(...samples)).toBeGreaterThan(0.39);
  expect(Math.max(...samples)).toBeLessThan(0.41);
});
