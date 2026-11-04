/**
 * @vitest-environment node
 */
// The HDR encode, end to end through the real ffmpeg: frames that hold an HLG
// signal (as the native client's `--hdr` frames do) come out as a stream tagged
// HLG whose decoded luma is that signal.
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, test } from "vitest";
import {
  blankFrameError,
  hdrEncodeArgs,
  hdrProbeArgs,
  hlgTagErrors,
  lumaStatsArgs,
} from "../capture/hdr.mjs";

const ffmpeg = (args: string[]) => execFileSync("ffmpeg", ["-v", "error", "-y", ...args]);
const SIZE = 64;

/** A 16-bit RGB PNG whose channels hold the HLG signal `rgb`, its lower half
 * `below` when given; answers its path. */
function frame(dir: string, index: number, rgb: number[], below = rgb) {
  const raw = Buffer.alloc(SIZE * SIZE * 6);
  for (let i = 0; i < raw.length; i += 2) {
    const colour = i < raw.length / 2 ? rgb : below;
    raw.writeUInt16BE(Math.round(colour[(i / 2) % 3] * 65535), i);
  }
  const rawFile = join(dir, `frame-${index}.rgb`);
  writeFileSync(rawFile, raw);
  const png = join(dir, `frame-${String(index).padStart(6, "0")}.png`);
  ffmpeg(["-f", "rawvideo", "-pix_fmt", "rgb48be", "-s", `${SIZE}x${SIZE}`, "-i", rawFile, png]);
  return png;
}

test("frames of an HLG signal encode to a tagged HLG stream that decodes to that signal", () => {
  const dir = mkdtempSync(join(tmpdir(), "hdr-encode-"));
  try {
    // Greys below reference white, at it (0.75) and at the 1000-nit peak
    // (1.0), and a red, whose luma only BT.2020's matrix gives.
    const frames = [
      [0.5, 0.5, 0.5],
      [0.75, 0.75, 0.75],
      [1, 1, 1],
      [0.75, 0, 0],
    ];
    frames.forEach((rgb, i) => frame(dir, i, rgb));
    const video = join(dir, "clip.mp4");
    const framePattern = join(dir, "frame-%06d.png");
    execFileSync(
      "ffmpeg",
      hdrEncodeArgs({
        framePattern,
        audio: null,
        output: video,
        fps: 30,
        width: SIZE,
        height: SIZE,
        hevc: true,
      }),
    );

    const probe = JSON.parse(execFileSync("ffprobe", hdrProbeArgs(video), { encoding: "utf8" }));
    expect(hlgTagErrors(probe, { hevc: true })).toEqual([]);

    const yuv = join(dir, "clip.yuv");
    ffmpeg(["-i", video, "-f", "rawvideo", "-pix_fmt", "yuv420p10le", yuv]);
    const planes = readFileSync(yuv);
    const frameBytes = SIZE * SIZE * 2 * 1.5;
    const luma = frames.map((_, i) => {
      let sum = 0;
      for (let p = 0; p < SIZE * SIZE; p++) sum += planes.readUInt16LE(i * frameBytes + p * 2);
      return sum / (SIZE * SIZE);
    });
    // Luma is the BT.2020 weighting of the signal, in 10-bit limited range.
    const expected = frames.map(([r, g, b]) => 64 + 876 * (0.2627 * r + 0.678 * g + 0.0593 * b));
    luma.forEach((y, i) => expect(Math.abs(y - expected[i])).toBeLessThan(4));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}, 60_000);

test("a frame of one flat colour is refused as nothing drawn; one with content is not", () => {
  const dir = mkdtempSync(join(tmpdir(), "hdr-blank-"));
  try {
    const verdict = (png: string) =>
      blankFrameError(png, execFileSync("ffmpeg", lumaStatsArgs(png), { encoding: "utf8" }));
    expect(verdict(frame(dir, 0, [0.6, 0.58, 0.5]))).toMatch(/flat colour/);
    expect(verdict(frame(dir, 1, [0.6, 0.58, 0.5], [0.3, 0.35, 0.2]))).toBeNull();
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}, 60_000);

test("the default encode is 8-bit HLG H.264 High that X uploads, with 48 kHz stereo AAC", () => {
  const dir = mkdtempSync(join(tmpdir(), "hdr-h264-"));
  try {
    const rgb = [0.75, 0, 0];
    frame(dir, 0, rgb);
    frame(dir, 1, rgb);
    // Mono sound at another rate: the clip's audio is still 48 kHz stereo AAC.
    const audio = join(dir, "sound.wav");
    ffmpeg(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=44100:duration=0.1", audio]);
    const video = join(dir, "clip.mp4");
    const framePattern = join(dir, "frame-%06d.png");
    execFileSync(
      "ffmpeg",
      hdrEncodeArgs({ framePattern, audio, output: video, fps: 30, width: SIZE, height: SIZE }),
    );
    const probe = (args: string[]) =>
      JSON.parse(execFileSync("ffprobe", args, { encoding: "utf8" }));
    const tags = probe(hdrProbeArgs(video));
    expect(hlgTagErrors(tags)).toEqual([]);
    expect(tags.streams[0]).toMatchObject({
      codec_name: "h264",
      profile: "High",
      pix_fmt: "yuv420p",
    });
    const audioEntries = "stream=codec_name,sample_rate,channels";
    const sound = probe([
      "-v",
      "error",
      "-select_streams",
      "a:0",
      "-show_entries",
      audioEntries,
      "-of",
      "json",
      video,
    ]).streams[0];
    expect(sound).toMatchObject({ codec_name: "aac", sample_rate: "48000", channels: 2 });

    // Refused for the other mode, and when not tagged HLG.
    expect(hlgTagErrors(tags, { hevc: true })).not.toEqual([]);
    const hevcTags = {
      streams: [
        { ...tags.streams[0], codec_name: "hevc", profile: "Main 10", pix_fmt: "yuv420p10le" },
      ],
    };
    expect(hlgTagErrors(hevcTags, { hevc: true })).toEqual([]);
    expect(hlgTagErrors(hevcTags)).not.toEqual([]);
    const sdr = { streams: [{ ...tags.streams[0], color_transfer: "bt709" }] };
    expect(hlgTagErrors(sdr)).toEqual([expect.stringContaining("color_transfer")]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}, 60_000);
