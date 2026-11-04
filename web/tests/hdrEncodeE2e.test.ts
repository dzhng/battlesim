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
import { hdrEncodeArgs, hdrProbeArgs, hlgTagErrors } from "../capture/hdr.mjs";

const ffmpeg = (args: string[]) => execFileSync("ffmpeg", ["-v", "error", "-y", ...args]);
const SIZE = 64;

/** A flat 16-bit RGB PNG whose channels hold the HLG signal `rgb`. */
function frame(dir: string, index: number, rgb: number[]) {
  const raw = Buffer.alloc(SIZE * SIZE * 6);
  for (let i = 0; i < raw.length; i += 2)
    raw.writeUInt16BE(Math.round(rgb[(i / 2) % 3] * 65535), i);
  const rawFile = join(dir, `frame-${index}.rgb`);
  writeFileSync(rawFile, raw);
  const png = join(dir, `frame-${String(index).padStart(6, "0")}.png`);
  ffmpeg(["-f", "rawvideo", "-pix_fmt", "rgb48be", "-s", `${SIZE}x${SIZE}`, "-i", rawFile, png]);
}

const ffprobe = (args: string[]) => JSON.parse(execFileSync("ffprobe", args, { encoding: "utf8" }));
/** Each encode's streams, kept for the refusal check below. */
const probes: Record<string, { streams: Record<string, unknown>[] }> = {};

test.each([
  { mode: "H.264 (default)", hevc: false, bits: 8 },
  { mode: "HEVC", hevc: true, bits: 10 },
])(
  "$mode: frames of an HLG signal encode to a tagged HLG stream that decodes to that signal",
  ({ mode, hevc, bits }) => {
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
      // Mono sound at another rate: the clip's audio is still 48 kHz stereo AAC.
      const audio = join(dir, "sound.wav");
      ffmpeg(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=44100:duration=0.2", audio]);
      const video = join(dir, "clip.mp4");
      const framePattern = join(dir, "frame-%06d.png");
      execFileSync(
        "ffmpeg",
        hdrEncodeArgs({
          framePattern,
          audio,
          output: video,
          fps: 30,
          width: SIZE,
          height: SIZE,
          hevc,
        }),
      );

      const probe = ffprobe(hdrProbeArgs(video));
      probes[mode] = probe;
      expect(hlgTagErrors(probe, { hevc })).toEqual([]);
      const sound = ffprobe([
        "-v",
        "error",
        "-select_streams",
        "a:0",
        "-show_entries",
        "stream=codec_name,profile,sample_rate,channels",
        "-of",
        "json",
        video,
      ]).streams[0];
      expect(sound).toMatchObject({
        codec_name: "aac",
        profile: "LC",
        sample_rate: "48000",
        channels: 2,
      });

      const yuv = join(dir, "clip.yuv");
      ffmpeg(["-i", video, "-f", "rawvideo", "-pix_fmt", "yuv420p10le", yuv]);
      const planes = readFileSync(yuv);
      const frameBytes = SIZE * SIZE * 2 * 1.5;
      const luma = frames.map((_, i) => {
        let sum = 0;
        for (let p = 0; p < SIZE * SIZE; p++) sum += planes.readUInt16LE(i * frameBytes + p * 2);
        return sum / (SIZE * SIZE);
      });
      // Luma is the BT.2020 weighting of the signal, in 10-bit limited range
      // (an 8-bit stream decodes to it shifted up, so within one 8-bit step).
      const expected = frames.map(([r, g, b]) => 64 + 876 * (0.2627 * r + 0.678 * g + 0.0593 * b));
      const tolerance = bits === 8 ? 8 : 4;
      luma.forEach((y, i) => expect(Math.abs(y - expected[i])).toBeLessThan(tolerance));
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  },
  60_000,
);

test("a stream tagged for the other mode, or not as HLG, is refused", () => {
  const h264 = probes["H.264 (default)"];
  const hevc = probes.HEVC;
  // Both encodes ran above; a 10-bit HEVC file is no default clip, and an
  // 8-bit H.264 file is no --hevc clip.
  expect(hlgTagErrors(hevc, { hevc: false })).not.toEqual([]);
  expect(hlgTagErrors(h264, { hevc: true })).not.toEqual([]);
  const sdr = { streams: [{ ...h264.streams[0], color_transfer: "bt709" }] };
  expect(hlgTagErrors(sdr)).toEqual([expect.stringContaining("color_transfer")]);
  expect(hlgTagErrors({ streams: [] })).not.toEqual([]);
});
