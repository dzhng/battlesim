// @vitest-environment node
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { expect, test } from "vitest";

test("audio rebuild refuses a changed pinned source before replacing a clip", () => {
  const root = mkdtempSync(join(tmpdir(), "battle-audio-assets-"));
  try {
    mkdirSync(join(root, "fixtures"));
    mkdirSync(join(root, "assets/third-party/audio"), { recursive: true });
    mkdirSync(join(root, "assets/runtime/audio/clips"), { recursive: true });
    writeFileSync(join(root, "assets/third-party/audio/source.wav"), "changed source");
    writeFileSync(join(root, "assets/runtime/audio/clips/report.wav"), "retained clip");
    writeFileSync(
      join(root, "fixtures/sounds.json"),
      JSON.stringify({
        sources: {
          source: { path: "assets/third-party/audio/source.wav", sha256: "a".repeat(64) },
        },
        clips: {
          report: {
            source: "source",
            source_rate: 48000,
            source_frames: [0, 100],
            processing: "weighted-shot",
            url: "/audio/clips/report.wav",
          },
        },
      }),
    );
    const run = spawnSync(
      "python3",
      [resolve("../packages/battle-audio/tools/assets.py"), "build", "--root", root],
      { encoding: "utf8" },
    );
    expect(run.status).not.toBe(0);
    expect(run.stderr).toMatch(/source.*hash/i);
    expect(readFileSync(join(root, "assets/runtime/audio/clips/report.wav"), "utf8")).toBe(
      "retained clip",
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

const hasFfmpeg = ["ffmpeg", "ffprobe"].every(
  (tool) => spawnSync(tool, ["-version"], { stdio: "ignore" }).status === 0,
);

/** Mono 16-bit PCM WAV bytes. */
function wav(samples: Float32Array, rate: number): Buffer {
  const data = Buffer.alloc(samples.length * 2);
  samples.forEach((v, i) => data.writeInt16LE(Math.round(v * 32767), i * 2));
  const head = Buffer.alloc(44);
  head.write("RIFF", 0);
  head.writeUInt32LE(36 + data.length, 4);
  head.write("WAVEfmt ", 8);
  head.writeUInt32LE(16, 16);
  head.writeUInt16LE(1, 20);
  head.writeUInt16LE(1, 22);
  head.writeUInt32LE(rate, 24);
  head.writeUInt32LE(rate * 2, 28);
  head.writeUInt16LE(2, 32);
  head.writeUInt16LE(16, 34);
  head.write("data", 36);
  head.writeUInt32LE(data.length, 40);
  return Buffer.concat([head, data]);
}

test.skipIf(!hasFfmpeg)(
  "a clip cut out of a sustained sound fades in and out instead of starting or stopping on the cut",
  () => {
    const root = mkdtempSync(join(tmpdir(), "battle-audio-fade-"));
    try {
      const rate = 48000;
      // A steady 440 Hz tone for 3 s; the clip takes the middle second. Then a crack
      // that rises out of a ringing tail, as a shot does out of the shot before it.
      const source = new Float32Array(4 * rate);
      for (let i = 0; i < 3 * rate; i++) source[i] = 0.5 * Math.sin((2 * Math.PI * 440 * i) / rate);
      for (let i = 3 * rate; i < 3.5 * rate; i++)
        source[i] = 0.08 * Math.sin((2 * Math.PI * 440 * i) / rate);
      for (let i = 0; i < 0.5 * rate; i++)
        source[3.5 * rate + i] = 0.9 * Math.exp(-i / 2000) * (i % 2 ? 1 : -1);
      const bytes = wav(source, rate);
      mkdirSync(join(root, "fixtures"));
      mkdirSync(join(root, "assets/third-party/audio"), { recursive: true });
      writeFileSync(join(root, "assets/third-party/audio/tone.wav"), bytes);
      writeFileSync(
        join(root, "fixtures/sounds.json"),
        JSON.stringify({
          sources: {
            tone: {
              path: "assets/third-party/audio/tone.wav",
              sha256: createHash("sha256").update(bytes).digest("hex"),
            },
          },
          clips: {
            cut: {
              source: "tone",
              source_rate: rate,
              source_frames: [rate, 2 * rate],
              processing: "launch",
              url: "/audio/clips/cut.wav",
              loop: false,
            },
            crack: {
              source: "tone",
              source_rate: rate,
              source_frames: [3.5 * rate, 4 * rate],
              processing: "launch",
              url: "/audio/clips/crack.wav",
              loop: false,
            },
            // The tone from its first sample, stopping while it still sounds, as a
            // generated master stops at its requested length.
            whole: {
              source: "tone",
              source_rate: rate,
              source_frames: [0, 3 * rate],
              processing: "launch",
              url: "/audio/clips/whole.wav",
              loop: false,
            },
          },
        }),
      );
      const run = spawnSync(
        "python3",
        [resolve("../packages/battle-audio/tools/assets.py"), "build", "--root", root],
        { encoding: "utf8" },
      );
      expect(run.stderr).toBe("");
      expect(run.status).toBe(0);
      const out = readFileSync(join(root, "assets/runtime/audio/clips/cut.wav"));
      const pcm = new Int16Array(out.buffer, out.byteOffset + 44, (out.length - 44) / 2);
      const rms = (from: number, to: number) =>
        Math.sqrt(pcm.slice(from, to).reduce((sum, v) => sum + v * v, 0) / (to - from));
      const steady = rms(pcm.length / 2 - 4800, pcm.length / 2 + 4800);
      const ms = rate / 1000;
      // It ends at near-zero level, and its last 50 ms are already well down: a
      // fade, not a cut with a click guard.
      expect(Math.max(...Array.from(pcm.slice(-ms), Math.abs))).toBeLessThan(0.02 * 32767);
      expect(rms(pcm.length - 50 * ms, pcm.length)).toBeLessThan(0.5 * steady);
      // It starts mid-tone, so it rises in over a few milliseconds rather than
      // starting at full level; short, so an attack in its place would survive.
      expect(rms(0, 2 * ms)).toBeLessThan(0.5 * steady);
      expect(rms(10 * ms, 20 * ms)).toBeGreaterThan(0.9 * steady);
      // The tone itself is kept between the fades.
      expect(rms(200 * ms, 800 * ms)).toBeGreaterThan(0.9 * steady);

      const whole = readFileSync(join(root, "assets/runtime/audio/clips/whole.wav"));
      const all = new Int16Array(whole.buffer, whole.byteOffset + 44, (whole.length - 44) / 2);
      const level = (from: number, to: number) =>
        Math.sqrt(all.slice(from, to).reduce((sum, v) => sum + v * v, 0) / (to - from));
      const held = level(all.length / 2 - 4800, all.length / 2 + 4800);
      expect(level(all.length - 50 * ms, all.length)).toBeLessThan(0.5 * held);
      // Nothing sounded before its first sample, so its start is left as recorded.
      expect(level(ms, 5 * ms)).toBeGreaterThan(0.9 * held);

      // A crack cut where it rises out of a ringing tail keeps its attack whole.
      const crack = readFileSync(join(root, "assets/runtime/audio/clips/crack.wav"));
      const hit = new Int16Array(crack.buffer, crack.byteOffset + 44, (crack.length - 44) / 2);
      expect(Math.max(...Array.from(hit.slice(0, 2 * ms), Math.abs))).toBeGreaterThan(0.5 * 32767);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  },
);

test.skipIf(!hasFfmpeg)("a burst clip re-lays its source shots at the gun's interval", () => {
  const root = mkdtempSync(join(tmpdir(), "battle-audio-burst-"));
  try {
    const rate = 48000;
    // Three decaying cracks 62.5 ms apart: faster than the 0.1 s the gun fires at.
    const onsets = [1000, 4000, 7000];
    const source = new Float32Array(11000);
    for (const at of onsets)
      for (let i = 0; i < 1500; i++) source[at + i] = 0.8 * Math.exp(-i / 150) * (i % 2 ? 1 : -1);
    const bytes = wav(source, rate);
    mkdirSync(join(root, "fixtures"));
    mkdirSync(join(root, "assets/third-party/audio"), { recursive: true });
    writeFileSync(join(root, "assets/third-party/audio/source.wav"), bytes);
    const clip = {
      source: "source",
      source_rate: rate,
      source_frames: [1000, 10000],
      processing: "impact",
      url: "/audio/clips/burst.wav",
      loop: false,
      burst: { shots: onsets.map((at) => [at, at + 3000]), interval_s: 0.1 },
    };
    writeFileSync(
      join(root, "fixtures/sounds.json"),
      JSON.stringify({
        sources: {
          source: {
            path: "assets/third-party/audio/source.wav",
            sha256: createHash("sha256").update(bytes).digest("hex"),
          },
        },
        clips: { burst: clip },
      }),
    );
    const run = spawnSync(
      "python3",
      [resolve("../packages/battle-audio/tools/assets.py"), "build", "--root", root],
      { encoding: "utf8" },
    );
    expect(run.stderr).toBe("");
    expect(run.status).toBe(0);
    const out = readFileSync(join(root, "assets/runtime/audio/clips/burst.wav"));
    const pcm = new Int16Array(out.buffer, out.byteOffset + 44, (out.length - 44) / 2);
    // Each crack starts where the gun's round does, and the last keeps its tail.
    const attacks: number[] = [];
    for (let i = 1; i < pcm.length; i++)
      if (Math.abs(pcm[i]) > 8000 && (!attacks.length || i - attacks.at(-1)! > 2000))
        attacks.push(i);
    expect(attacks.map((i) => Math.round(i / 48))).toEqual([0, 100, 200]);
    expect(pcm.length).toBe(2 * 4800 + 3000);
    const catalog = JSON.parse(readFileSync(join(root, "fixtures/sounds.json"), "utf8"));
    expect(catalog.clips.burst.frames).toBe(pcm.length);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test.skipIf(!hasFfmpeg)(
  "a layered clip mixes crops of several sources at their offsets, gains and pitch",
  () => {
    const root = mkdtempSync(join(tmpdir(), "battle-audio-layers-"));
    try {
      const crack = (rate: number, length: number) => {
        const x = new Float32Array(length);
        for (let i = 0; i < 1200; i++) x[200 + i] = 0.8 * Math.exp(-i / 200) * (i % 2 ? 1 : -1);
        return wav(x, rate);
      };
      // Two sources at different rates; the layered clip owns no single source.
      const strike = crack(48000, 4000);
      const whine = crack(44100, 4000);
      mkdirSync(join(root, "fixtures"));
      mkdirSync(join(root, "assets/third-party/audio"), { recursive: true });
      writeFileSync(join(root, "assets/third-party/audio/strike.wav"), strike);
      writeFileSync(join(root, "assets/third-party/audio/whine.wav"), whine);
      const sha = (b: Buffer) => createHash("sha256").update(b).digest("hex");
      const layer = (source: string, rate: number, at_s: number, semitones: number) => ({
        source,
        source_rate: rate,
        source_frames: [200, 1600],
        at_s,
        gain: 1,
        semitones,
        lowpass_hz: null,
      });
      writeFileSync(
        join(root, "fixtures/sounds.json"),
        JSON.stringify({
          sources: {
            strike: { path: "assets/third-party/audio/strike.wav", sha256: sha(strike) },
            whine: { path: "assets/third-party/audio/whine.wav", sha256: sha(whine) },
          },
          clips: {
            layered: {
              processing: "impact",
              url: "/audio/clips/layered.wav",
              loop: false,
              layers: [layer("strike", 48000, 0, 0), layer("whine", 44100, 0.05, -12)],
            },
          },
        }),
      );
      const run = spawnSync(
        "python3",
        [resolve("../packages/battle-audio/tools/assets.py"), "build", "--root", root],
        { encoding: "utf8" },
      );
      expect(run.stderr).toBe("");
      expect(run.status).toBe(0);
      const out = readFileSync(join(root, "assets/runtime/audio/clips/layered.wav"));
      const pcm = new Int16Array(out.buffer, out.byteOffset + 44, (out.length - 44) / 2);
      const attacks: number[] = [];
      for (let i = 1; i < pcm.length; i++)
        if (Math.abs(pcm[i]) > 6000 && (!attacks.length || i - attacks.at(-1)! > 1200))
          attacks.push(i);
      expect(attacks.map((i) => Math.round(i / 48))).toEqual([0, 50]);
      // An octave down plays the 1400-frame crop twice as long, after its offset.
      expect(pcm.length).toBe(Math.round(0.05 * 48000 + (2 * 1400 * 48000) / 44100));
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  },
);

/** Integrated loudness (LUFS) and channel count of an encoded file, as ffmpeg reads them. */
function measure(path: string) {
  const loudness = spawnSync(
    "ffmpeg",
    ["-hide_banner", "-nostats", "-i", path, "-af", "ebur128", "-f", "null", "-"],
    { encoding: "utf8" },
  ).stderr;
  const probe = spawnSync(
    "ffprobe",
    ["-v", "error", "-show_entries", "stream=codec_name,channels,bit_rate", "-of", "json", path],
    { encoding: "utf8" },
  );
  const stream = JSON.parse(probe.stdout).streams[0];
  return {
    lufs: Number(/I:\s+(-?[\d.]+) LUFS/.exec(loudness.split("Summary:")[1])![1]),
    codec: stream.codec_name as string,
    channels: stream.channels as number,
    kbps: Number(stream.bit_rate) / 1000,
  };
}

test.skipIf(!hasFfmpeg)(
  "a voice take comes out mono MP3 at its profile's loudness, over its bed",
  () => {
    const root = mkdtempSync(join(tmpdir(), "battle-audio-voice-"));
    try {
      const assets = resolve("../packages/battle-audio/tools/assets.py");
      const rate = 48000;
      // A quiet stereo "line": syllables of a voiced buzz, well under any target.
      const line = new Float32Array(2 * rate);
      for (let i = 0; i < line.length; i++)
        line[i] =
          0.05 *
          Math.sin((2 * Math.PI * 180 * i) / rate) *
          (Math.sin((Math.PI * 4 * i) / rate) > 0 ? 1 : 0.1);
      writeFileSync(join(root, "line.wav"), wav(line, rate));
      const engine = new Float32Array(3 * rate).map(
        (_, i) => 0.3 * Math.sin((2 * Math.PI * 60 * i) / rate),
      );
      writeFileSync(join(root, "engine.wav"), wav(engine, rate));
      const { profiles, encoder } = JSON.parse(
        spawnSync("python3", [assets, "voice-profiles"], { encoding: "utf8" }).stdout,
      );
      const voice = (profile: string, ...more: string[]) =>
        spawnSync(
          "python3",
          [
            assets,
            "voice",
            "--profile",
            profile,
            "--input",
            join(root, "line.wav"),
            "--output",
            join(root, `${profile}.mp3`),
            ...more,
          ],
          { encoding: "utf8" },
        );
      for (const [profile, spec] of Object.entries<{
        bed: { name: string } | null;
        loudness_lufs: number;
      }>(profiles)) {
        const run = spec.bed ? voice(profile, "--bed", join(root, "engine.wav")) : voice(profile);
        expect(run.stderr).toBe("");
        expect(JSON.parse(run.stdout).duration_s).toBeCloseTo(2, 1);
        const out = measure(join(root, `${profile}.mp3`));
        expect(out).toMatchObject({ codec: "mp3", channels: 1 });
        expect(Math.abs(out.kbps - parseInt(encoder.bitrate))).toBeLessThanOrEqual(2);
        expect(Math.abs(out.lufs - spec.loudness_lufs)).toBeLessThanOrEqual(1);
      }
      // A profile heard over an engine refuses to render without one.
      const bedded = Object.keys(profiles).find((name) => profiles[name].bed)!;
      const missing = voice(bedded);
      expect(missing.status).not.toBe(0);
      expect(missing.stderr).toMatch(/bed/);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  },
);

test("every voice bed is the game's own catalog clip or a generated bed still in use", () => {
  const { profiles } = JSON.parse(
    spawnSync("python3", [resolve("../packages/battle-audio/tools/assets.py"), "voice-profiles"], {
      encoding: "utf8",
    }).stdout,
  ) as { profiles: Record<string, { bed: { clip?: string; generated?: string } | null }> };
  const clips = JSON.parse(readFileSync(resolve("../fixtures/sounds.json"), "utf8")).clips;
  const lock = JSON.parse(readFileSync(resolve("../fixtures/voices.lock.json"), "utf8"));
  const beds = Object.values(profiles).flatMap((profile) => (profile.bed ? [profile.bed] : []));
  expect(beds.length).toBeGreaterThan(0);
  for (const bed of beds) {
    // Exactly one source: a clip the battle already plays, or a generated raw master.
    expect(Boolean(bed.clip) !== Boolean(bed.generated)).toBe(true);
    if (bed.clip) expect(clips[bed.clip]?.url).toMatch(/^\/audio\/clips\//);
    else expect(lock.beds[bed.generated!]).toBeDefined();
  }
  // A generated bed no profile names is a stale master: the lock holds none.
  const generated = new Set(beds.map((bed) => bed.generated).filter(Boolean));
  expect(Object.keys(lock.beds).filter((name) => !generated.has(name))).toEqual([]);
});
