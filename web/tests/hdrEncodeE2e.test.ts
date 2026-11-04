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
      }),
    );

    const probe = JSON.parse(execFileSync("ffprobe", hdrProbeArgs(video), { encoding: "utf8" }));
    expect(hlgTagErrors(probe)).toEqual([]);
    expect(probe.streams[0].profile).toBe("Main 10");

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
