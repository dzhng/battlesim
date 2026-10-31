#!/usr/bin/env node
// HDR capture pipeline: browser record → native HDR replay → HLG encode.
//
// Usage:
//   bun run --cwd web hdr-capture <run-dir> <scene> [flags]
//
// The run-dir must contain a native parity run (capture/*.json files).
// The script:
//   1. Runs the native client with --hdr --all-frames to render HDR frames + sound
//   2. Encodes the EXR frames + WAV to HLG HEVC Main10 MP4
//
// Flags:
//   --fps=<n>        Frame rate (default 60)
//   --output=<dir>   Output directory (default throwaway/hdr-capture/)
//   --h264           Also emit H.264 HLG variant (for X playback)
//   --from-ms=<n>    Start time in milliseconds (default: start of reel)
//   --to-ms=<n>      End time in milliseconds (default: end of reel)
//   --no-music       Disable music during sound rendering

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { hdrEncodeArgs, hdrProbeArgs, verifyHdrMetadata } from "./hdr.mjs";

const REPO_ROOT = resolve(new URL("../..", import.meta.url).pathname);
const NATIVE_CLIENT = join(REPO_ROOT, "target/release/native-client");

const USAGE = `usage: bun run --cwd web hdr-capture <run-dir> <scene> [flags]

  <run-dir>      Path to a native parity run directory (parent of <scene>/)
  <scene>        Scene/map name to render

Flags:
  --fps=<n>      Frame rate (default 60)
  --output=<dir> Output directory (default throwaway/hdr-capture/)
  --h264         Also emit H.264 HLG variant (for X playback)
  --keep-frames  Keep intermediate EXR frames after encoding
  --from-ms=<n>  Start time in milliseconds (default: start of reel)
  --to-ms=<n>    End time in milliseconds (default: end of reel)
  --no-music     Disable music during sound rendering`;

function parseArgs(argv) {
  const args = {
    runDir: null,
    scene: null,
    fps: 60,
    output: join(REPO_ROOT, "throwaway/hdr-capture"),
    h264: false,
    keepFrames: false,
    fromMs: null,
    toMs: null,
    noMusic: false,
  };

  for (const arg of argv) {
    if (arg === "--") continue;
    if (arg === "--h264") {
      args.h264 = true;
    } else if (arg === "--keep-frames") {
      args.keepFrames = true;
    } else if (arg === "--no-music") {
      args.noMusic = true;
    } else if (arg.startsWith("--fps=")) {
      args.fps = Number(arg.slice("--fps=".length));
      if (!Number.isSafeInteger(args.fps) || args.fps < 1 || args.fps > 120) {
        throw new Error(`--fps must be an integer 1-120, got ${arg.slice(6)}`);
      }
    } else if (arg.startsWith("--output=")) {
      args.output = resolve(REPO_ROOT, arg.slice("--output=".length));
    } else if (arg.startsWith("--from-ms=")) {
      args.fromMs = Number(arg.slice("--from-ms=".length));
      if (!Number.isFinite(args.fromMs) || args.fromMs < 0) {
        throw new Error(`--from-ms must be a non-negative number`);
      }
    } else if (arg.startsWith("--to-ms=")) {
      args.toMs = Number(arg.slice("--to-ms=".length));
      if (!Number.isFinite(args.toMs) || args.toMs < 0) {
        throw new Error(`--to-ms must be a non-negative number`);
      }
    } else if (arg.startsWith("-")) {
      throw new Error(`unknown flag ${arg}\n\n${USAGE}`);
    } else if (!args.runDir) {
      args.runDir = resolve(REPO_ROOT, arg);
    } else if (!args.scene) {
      args.scene = arg;
    } else {
      throw new Error(`unexpected argument: ${arg}\n\n${USAGE}`);
    }
  }

  if (!args.runDir) throw new Error(`missing run directory\n\n${USAGE}`);
  if (!args.scene) throw new Error(`missing scene name\n\n${USAGE}`);
  if (args.fromMs !== null && args.toMs !== null && args.fromMs >= args.toMs) {
    throw new Error(`--from-ms must be less than --to-ms`);
  }

  return args;
}

function checkFfmpeg() {
  try {
    execFileSync("ffmpeg", ["-version"], { stdio: "pipe" });
    execFileSync("ffprobe", ["-version"], { stdio: "pipe" });
  } catch {
    throw new Error("ffmpeg and ffprobe must be installed and on PATH");
  }
}

function checkNativeClient() {
  if (!existsSync(NATIVE_CLIENT)) {
    throw new Error(
      `native client not found at ${NATIVE_CLIENT}\n` +
        "Build it with: cargo build --release -p native-client"
    );
  }
}

function runNativeClient(runDir, scene, framesDir, soundFile, options) {
  console.log(`Running native client with HDR output...`);
  const args = [
    "--run",
    runDir,
    "--scene",
    scene,
    "--out",
    framesDir,
    "--hdr",
    "--all-frames",
    "--fps",
    String(options.fps),
    "--sound",
    soundFile,
    "--window",
    "offscreen",
    "--pace",
    "0",
  ];
  if (options.fromMs !== null) {
    args.push("--from-ms", String(options.fromMs));
  }
  if (options.toMs !== null) {
    args.push("--to-ms", String(options.toMs));
  }
  if (options.noMusic) {
    args.push("--no-music");
  }
  console.log(`  ${NATIVE_CLIENT} ${args.join(" ")}`);
  const result = spawnSync(NATIVE_CLIENT, args, {
    stdio: "inherit",
    env: { ...process.env, BATTLEGAME_ROOT: REPO_ROOT },
  });
  if (result.status !== 0) {
    throw new Error(`native client failed with exit code ${result.status}`);
  }
}

function findFrames(framesDir) {
  const files = readdirSync(framesDir).filter((f) => f.endsWith(".exr")).sort();
  if (files.length === 0) {
    throw new Error(`no EXR frames found in ${framesDir}`);
  }
  // Sequential naming: frame-000001.exr, frame-000002.exr, ...
  // The native client with --all-frames uses this format.
  const first = files[0];
  const last = files[files.length - 1];
  const match = first.match(/^frame-(\d+)\.exr$/);
  if (!match) {
    // Fallback: try to detect other patterns
    const altMatch = first.match(/^(.+?)(\d+)(\.exr)$/);
    if (!altMatch) {
      throw new Error(`unexpected frame naming: ${first}`);
    }
    const [, prefix, digits, suffix] = altMatch;
    const pattern = `${prefix}%0${digits.length}d${suffix}`;
    return {
      pattern: join(framesDir, pattern),
      count: files.length,
      first: files[0],
    };
  }
  // Sequential frame-NNNNNN.exr pattern
  const startIndex = parseInt(match[1], 10);
  const lastMatch = last.match(/^frame-(\d+)\.exr$/);
  const endIndex = lastMatch ? parseInt(lastMatch[1], 10) : startIndex + files.length - 1;

  // Verify sequential: count should match expected range
  const expected = endIndex - startIndex + 1;
  if (files.length !== expected) {
    console.warn(`Warning: expected ${expected} frames (${startIndex}-${endIndex}), found ${files.length}`);
  }

  return {
    pattern: join(framesDir, "frame-%06d.exr"),
    count: files.length,
    first: files[0],
    startNumber: startIndex,
  };
}

function getFrameSize(framesDir, firstFrame) {
  const result = execFileSync(
    "ffprobe",
    [
      "-v",
      "error",
      "-select_streams",
      "v:0",
      "-show_entries",
      "stream=width,height",
      "-of",
      "json",
      join(framesDir, firstFrame),
    ],
    { encoding: "utf8" }
  );
  const probe = JSON.parse(result);
  const stream = probe?.streams?.[0];
  if (!stream?.width || !stream?.height) {
    throw new Error("could not determine frame size");
  }
  return { width: stream.width, height: stream.height };
}

function encodeHdr(framePattern, audioFile, outputFile, fps, width, height, hevc, startNumber) {
  const format = hevc ? "HEVC" : "H.264";
  console.log(`Encoding ${format} HLG...`);
  const args = hdrEncodeArgs({
    framePattern,
    audio: existsSync(audioFile) ? audioFile : null,
    output: outputFile,
    fps,
    width,
    height,
    hevc,
    startNumber,
  });
  console.log(`  ffmpeg ${args.join(" ")}`);
  execFileSync("ffmpeg", args, { stdio: "inherit" });
}

function probeAndVerify(outputFile) {
  console.log(`Verifying HDR metadata...`);
  const result = execFileSync("ffprobe", hdrProbeArgs(outputFile), {
    encoding: "utf8",
  });
  const probe = JSON.parse(result);
  const { valid, errors } = verifyHdrMetadata(probe);
  if (!valid) {
    console.error("HDR verification failed:");
    errors.forEach((e) => console.error(`  - ${e}`));
    throw new Error("HDR metadata verification failed");
  }
  console.log("  ✓ HDR metadata verified");
  return probe.streams[0];
}

function main() {
  try {
    const args = parseArgs(process.argv.slice(2));
    checkFfmpeg();
    checkNativeClient();

    mkdirSync(args.output, { recursive: true });

    const framesDir = join(args.output, "frames");
    const soundFile = join(args.output, "sound.wav");
    mkdirSync(framesDir, { recursive: true });

    runNativeClient(args.runDir, args.scene, framesDir, soundFile, {
      fps: args.fps,
      fromMs: args.fromMs,
      toMs: args.toMs,
      noMusic: args.noMusic,
    });

    const frames = findFrames(framesDir);
    const { width, height } = getFrameSize(framesDir, frames.first);
    const startNumber = frames.startNumber || 0;
    console.log(`Found ${frames.count} frames at ${width}x${height} (starting at ${startNumber})`);

    const hevcOutput = join(args.output, `${args.scene}-hdr.mp4`);
    encodeHdr(frames.pattern, soundFile, hevcOutput, args.fps, width, height, true, startNumber);
    const hevcMeta = probeAndVerify(hevcOutput);

    let h264Output = null;
    if (args.h264) {
      h264Output = join(args.output, `${args.scene}-hdr-h264.mp4`);
      encodeHdr(frames.pattern, soundFile, h264Output, args.fps, width, height, false, startNumber);
      probeAndVerify(h264Output);
    }

    if (!args.keepFrames) {
      console.log("Cleaning up intermediate frames...");
      rmSync(framesDir, { recursive: true });
    }

    const report = {
      scene: args.scene,
      runDir: args.runDir,
      fps: args.fps,
      frames: frames.count,
      width,
      height,
      timeWindow: {
        fromMs: args.fromMs,
        toMs: args.toMs,
      },
      outputs: {
        hevc: {
          file: hevcOutput,
          codec: hevcMeta.codec_name,
          profile: hevcMeta.profile,
          pixFmt: hevcMeta.pix_fmt,
          colorPrimaries: hevcMeta.color_primaries,
          colorSpace: hevcMeta.color_space,
          colorTransfer: hevcMeta.color_transfer,
          colorRange: hevcMeta.color_range,
        },
        ...(h264Output && {
          h264: { file: h264Output },
        }),
      },
    };

    const reportFile = join(args.output, `${args.scene}-hdr.report.json`);
    writeFileSync(reportFile, JSON.stringify(report, null, 2) + "\n");

    console.log("\n✓ HDR capture complete");
    console.log(`  HEVC output: ${hevcOutput}`);
    if (h264Output) {
      console.log(`  H.264 output: ${h264Output}`);
    }
    console.log(`  Report: ${reportFile}`);

    console.log("\nffprobe output (HEVC):");
    console.log(
      execFileSync("ffprobe", hdrProbeArgs(hevcOutput), { encoding: "utf8" })
    );
  } catch (err) {
    console.error(`\nError: ${err.message}`);
    process.exit(1);
  }
}

main();
