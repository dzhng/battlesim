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
