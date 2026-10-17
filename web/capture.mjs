#!/usr/bin/env node
// Cinematic capture CLI: record the menu backdrop battle with a scripted camera.
//
// Uses CDP Page.startScreencast for page-level capture, which includes both the
// WebGL canvas AND DOM overlays (info cards). Audio is captured separately via
// the browser's Web Audio API and muxed with ffmpeg.
//
// Usage:
//   bun run --cwd web capture [options]
//   node capture.mjs [options]
//
// Options:
//   --preset=<name>    Camera preset (default, top-down, ground-level, orbit)
//   --width=<n>        Output width in pixels (default 1920)
//   --height=<n>       Output height in pixels (default 1080)
//   --fps=<n>          Frame rate (default 60)
//   --no-audio         Disable audio recording
//   --no-cards         Hide info cards
//   --webm             Output WebM instead of MP4 (skip ffmpeg transcoding)
//   --output=<path>    Output directory (default throwaway/capture/)
//   --scene=<name>     Scene to capture: menu (default) or skirmish
//
// Example:
//   bun run --cwd web capture --preset=ground-level --width=3840 --height=2160

import { mkdir, writeFile, unlink, access, stat, readFile } from "node:fs/promises";
import { resolve, basename, join } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { startServer, ensureNativeTools } from "./scene.mjs";
import { launchBrowser } from "./browser.mjs";

const HERE = new URL(".", import.meta.url);
const OUTPUT_DIR = new URL("../throwaway/capture/", HERE);
const ROOT = new URL("../", HERE);

function parseArgs(argv) {
  const args = {
    preset: "default",
    width: 1920,
    height: 1080,
    fps: 60,
    audio: true,
    cards: true,
    grade: true,
    webm: false,
    output: OUTPUT_DIR.pathname,
    scene: "menu",
    deterministic: false, // Virtual clock + explicit frame capture
    keepRaw: false, // Keep raw audio and frames for offline analysis
  };

  for (const arg of argv.slice(2)) {
    if (arg.startsWith("--preset=")) {
      args.preset = arg.slice("--preset=".length);
    } else if (arg.startsWith("--width=")) {
      args.width = parseInt(arg.slice("--width=".length), 10);
    } else if (arg.startsWith("--height=")) {
      args.height = parseInt(arg.slice("--height=".length), 10);
    } else if (arg.startsWith("--fps=")) {
      args.fps = parseInt(arg.slice("--fps=".length), 10);
    } else if (arg === "--no-audio") {
      args.audio = false;
    } else if (arg === "--no-cards") {
      args.cards = false;
    } else if (arg === "--no-grade") {
      args.grade = false;
    } else if (arg === "--webm") {
      args.webm = true;
    } else if (arg.startsWith("--output=")) {
      args.output = resolve(arg.slice("--output=".length));
    } else if (arg.startsWith("--scene=")) {
      args.scene = arg.slice("--scene=".length);
    } else if (arg === "--deterministic") {
      args.deterministic = true;
    } else if (arg === "--realtime") {
      args.deterministic = false;
    } else if (arg === "--keep-raw") {
      args.keepRaw = true;
    } else if (arg === "--help" || arg === "-h") {
      console.log(`Cinematic capture: record menu backdrop with scripted camera

Usage:
  bun run --cwd web capture [options]

Options:
  --preset=<name>    Camera preset (default, top-down, ground-level, orbit)
  --width=<n>        Output width in pixels (default 1920)
  --height=<n>       Output height in pixels (default 1080)
  --fps=<n>          Frame rate (default 60)
  --no-audio         Disable audio recording
  --no-cards         Hide info cards
  --no-grade         Disable color grade (for before/after comparisons)
  --webm             Output WebM instead of MP4 (skip ffmpeg transcoding)
  --output=<path>    Output directory (default throwaway/capture/)
  --scene=<name>     Scene to capture: menu (default) or skirmish
  --keep-raw         Keep raw audio, calibration frames, and detailed frames.json
  --deterministic    [EXPERIMENTAL] Virtual clock capture (may timeout, follow-up)

Examples:
  bun run --cwd web capture
  bun run --cwd web capture --preset=ground-level
  bun run --cwd web capture --width=3840 --height=2160 --preset=orbit
  bun run --cwd web capture --scene=skirmish
  bun run --cwd web capture --no-grade  # Without color grading
`);
      process.exit(0);
    }
  }

  return args;
}

/** Check if the mechanics validator is built. */
async function checkMechanicsBuilt() {
  const mechanicsPath = new URL("target/debug/examples/mechanics_validate", ROOT);
  const mechanicsRelease = new URL("target/release/examples/mechanics_validate", ROOT);
  if (!existsSync(mechanicsPath) && !existsSync(mechanicsRelease)) {
    return false;
  }
  return true;
}

/** Check if the wasm build exists. */
async function checkWasmBuilt() {
  const wasmPath = new URL("web/src/wasm/presentation/presentation_wasm_bg.wasm", ROOT);
  return existsSync(wasmPath);
}

/** Run a build command and wait for it. */
async function runBuild(name, command) {
  console.log(`Building ${name}...`);
  return new Promise((resolve, reject) => {
    const proc = spawn("bun", ["run", command], {
      cwd: ROOT.pathname,
      stdio: "inherit",
    });
    proc.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`${command} failed with code ${code}`));
    });
    proc.on("error", reject);
  });
}

/** Ensure prerequisite builds are done, running them if needed. */
async function ensureBuilds() {
  if (!(await checkMechanicsBuilt())) {
    await runBuild("mechanics validator", "build:mechanics");
  }
  if (!(await checkWasmBuilt())) {
    await runBuild("wasm", "build:wasm");
  }
}

/** Check if a file is an LFS pointer (text file starting with "version https://git-lfs.github.com"). */
async function isLfsPointer(filePath) {
  try {
    const content = await readFile(filePath, "utf8");
    return content.startsWith("version https://git-lfs.github.com");
  } catch {
    return false;
  }
}

/** Check for LFS pointer files in critical asset directories. */
async function checkLfsAssets() {
  const assetDirs = [new URL("assets/sounds/", ROOT), new URL("assets/models/", ROOT)];
  const pointers = [];
  for (const dir of assetDirs) {
    if (!existsSync(dir)) continue;
    try {
      const { readdirSync } = await import("node:fs");
      const files = readdirSync(dir).slice(0, 5);
      for (const file of files) {
        const filePath = new URL(file, dir);
        if (await isLfsPointer(filePath)) {
          pointers.push(filePath.pathname);
        }
      }
    } catch {
      // Directory may not exist
    }
  }
  return pointers;
}

/** Check prerequisites and build what's missing. */
async function checkPrerequisites() {
  const lfsPointers = await checkLfsAssets();
  if (lfsPointers.length > 0) {
    console.error("LFS assets are pointers (not downloaded). Run: git lfs pull");
    return false;
  }

  // Ensure mechanics and wasm are built
  await ensureBuilds();
  return true;
}

/** Check if ffmpeg is available. */
function hasFfmpeg() {
  try {
    const result = spawnSync("ffmpeg", ["-version"], { stdio: "pipe" });
    return result.status === 0;
  } catch {
    return false;
  }
}

/** Deterministic capture: virtual clock, explicit frame capture, perfect timing.
 *  Instead of real-time screencast (limited to ~50fps), this mode:
 *  1. Drives the sim/camera/render from a virtual clock advanced by exactly 1/60s per frame
 *  2. Captures each frame explicitly via CDP Page.captureScreenshot after it's presented
 *  3. Outputs frames that are unique and exactly at the target fps (no stutter)
 *  4. Records sound events for offline audio rendering */
async function captureWithDeterministic(page, args) {
  const cdp = await page.context().newCDPSession(page);

  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  const sceneName = args.scene === "skirmish" ? "skirmish" : "menu";
  const rawVideoPath = resolve(args.output, `raw-${timestamp}.mp4`);
  const framesDir = resolve(args.output, `frames-${timestamp}`);
  const concatPath = resolve(args.output, `concat-${timestamp}.txt`);
  const finalPath = resolve(
    args.output,
    `capture-${sceneName}-${args.preset}-${timestamp}.${args.webm ? "webm" : "mp4"}`,
  );

  await mkdir(framesDir, { recursive: true });

  const frameDurationMs = 1000 / args.fps;
  const frameDurationS = 1 / args.fps;
  const frames = []; // { path, virtualTimeMs }

  // Wait for scene to be ready
  console.log("Waiting for scene to prepare...");
  const reelDuration = await page.evaluate(() => window.__capture?.duration ?? 0);
  if (!reelDuration || reelDuration <= 0) {
    throw new Error("Scene duration not available");
  }

  // Preroll cut: skip the first portion of the reel (same as realtime mode's calibration cut)
  // The reel has 2.0s preroll; we cut at ~1.15s like realtime mode does
  const PREROLL_CUT_S = 1.15;
  const cutStartS = PREROLL_CUT_S;
  const outputDuration = reelDuration - cutStartS;

  console.log(`  Full reel: ${reelDuration.toFixed(2)}s, preroll cut at ${cutStartS.toFixed(2)}s`);
  console.log(
    `  Output duration: ${outputDuration.toFixed(2)}s, ${Math.ceil(outputDuration * args.fps)} frames at ${args.fps}fps`,
  );

  // Wait for reel to be ready
  console.log("Waiting for reel-ready signal...");
  await page.waitForFunction(() => window.__capture?.reelReadyTime !== null, undefined, {
    timeout: 60000,
  });

  // Initialize deterministic mode in the page
  console.log("Initializing deterministic capture mode...");
  const hasStepFrame = await page.evaluate(() => typeof window.__capture?.stepFrame === "function");
  if (!hasStepFrame) {
    throw new Error(
      "Page does not support deterministic capture (stepFrame not available). Update the capture route.",
    );
  }

  // Configure deterministic mode - this advances the sim to cover the entire duration
  console.log("Entering deterministic mode (advancing sim)...");
  await page.evaluate(async () => {
    if (window.__capture?.enterDeterministicMode) {
      await window.__capture.enterDeterministicMode();
    }
  });
  console.log("  Sim advanced, ready for capture");

  // Note: In deterministic mode, we record audio commands for offline rendering
  // rather than capturing real-time audio (which wouldn't sync with virtual time)

  // Capture frames at exact virtual times, starting from the preroll cut point
  const totalOutputFrames = Math.ceil(outputDuration * args.fps);
  const cutStartMs = cutStartS * 1000;
  console.log(
    `Capturing ${totalOutputFrames} frames with virtual clock (starting at ${cutStartS.toFixed(2)}s)...`,
  );
  console.log(`  Note: Deterministic capture runs at ~5 fps; this will take several minutes.`);

  const captureStart = Date.now();
  for (let outputIdx = 0; outputIdx < totalOutputFrames; outputIdx++) {
    // Virtual time starts from the cut point
    const virtualTimeMs = cutStartMs + outputIdx * frameDurationMs;

    // Step the frame to this virtual time - this updates the camera pose
    await page.evaluate((t) => window.__capture?.stepFrame(t), virtualTimeMs);

    // Wait for two RAF frames to ensure the render is complete with the new pose
    await page.evaluate(
      () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))),
    );

    // Capture the frame via CDP
    const screenshotResult = await cdp.send("Page.captureScreenshot", {
      format: "jpeg",
      quality: 90,
      captureBeyondViewport: false,
    });

    // Save frame
    const framePath = join(framesDir, `frame-${String(outputIdx).padStart(6, "0")}.jpg`);
    const frameData = Buffer.from(screenshotResult.data, "base64");
    await writeFile(framePath, frameData);

    // Compute hash for uniqueness check (first 8 chars of MD5)
    const { createHash } = await import("node:crypto");
    const hash = createHash("md5").update(frameData).digest("hex").slice(0, 8);

    // Get current sim/capture state for diagnostics
    const state = await page.evaluate(() => window.__capture?.getDeterministicState?.());

    frames.push({
      path: framePath,
      virtualTimeMs,
      idx: outputIdx,
      hash,
      tick: state?.tick ?? 0,
      frame: state?.frame ?? outputIdx,
    });

    // Progress
    if (outputIdx % 30 === 0 || outputIdx === totalOutputFrames - 1) {
      const elapsed = (Date.now() - captureStart) / 1000;
      const pct = (((outputIdx + 1) / totalOutputFrames) * 100).toFixed(0);
      const captureFps = (outputIdx + 1) / elapsed;
      const eta = (elapsed / (outputIdx + 1)) * (totalOutputFrames - outputIdx - 1);
      const tickInfo = state ? ` tick=${state.tick}` : "";
      process.stdout.write(
        `\r  Capturing: frame ${outputIdx + 1}/${totalOutputFrames} (${pct}%) at ${captureFps.toFixed(1)} fps, ETA ${Math.ceil(eta)}s${tickInfo}    `,
      );
    }
  }
  const captureElapsed = (Date.now() - captureStart) / 1000;
  const actualCaptureFps = frames.length / captureElapsed;
  process.stdout.write("\n");
  console.log(
    `  Captured ${frames.length} frames in ${captureElapsed.toFixed(1)}s (${actualCaptureFps.toFixed(1)} fps capture rate)`,
  );

  // Get recorded audio commands for offline rendering
  console.log("Getting recorded audio commands...");
  const audioCommands = await page.evaluate(
    () => window.__capture?.getRecordedAudioCommands?.() ?? [],
  );
  console.log(`  Recorded ${audioCommands.length} audio commands`);

  // Frame pacing analysis: compute uniqueness from hashes
  console.log("Analyzing frame uniqueness...");
  const hashCounts = new Map();
  const duplicateRuns = [];
  let currentRunStart = -1;
  let duplicateCount = 0;

  for (let i = 0; i < frames.length; i++) {
    const frame = frames[i];
    const count = (hashCounts.get(frame.hash) ?? 0) + 1;
    hashCounts.set(frame.hash, count);

    // Track runs of duplicate frames
    if (i > 0 && frames[i].hash === frames[i - 1].hash) {
      if (currentRunStart < 0) {
        currentRunStart = i - 1;
      }
      duplicateCount++;
    } else if (currentRunStart >= 0) {
      // End of a duplicate run
      const runLength = i - currentRunStart;
      const runTimeS = currentRunStart / args.fps;
      duplicateRuns.push({
        startFrame: currentRunStart,
        endFrame: i - 1,
        length: runLength,
        timeS: runTimeS,
      });
      currentRunStart = -1;
    }
  }
  // Handle run extending to end
  if (currentRunStart >= 0) {
    const runLength = frames.length - currentRunStart;
    const runTimeS = currentRunStart / args.fps;
    duplicateRuns.push({
      startFrame: currentRunStart,
      endFrame: frames.length - 1,
      length: runLength,
      timeS: runTimeS,
    });
  }

  const uniqueHashes = hashCounts.size;
  const framePacing = {
    totalFrames: frames.length,
    outputFrames: frames.length,
    uniqueFrames: uniqueHashes,
    duplicateFrames: duplicateCount,
    droppedEarlyFrames: 0,
    droppedTailFrames: 0,
    longestGapMs: frameDurationMs,
    longestGapTimestamp: 0,
    gaps: [],
    duplicateRuns,
  };

  // Log duplicates if any
  if (duplicateCount > 0) {
    console.log(
      `  WARNING: ${duplicateCount} duplicate frames detected (${uniqueHashes} unique / ${frames.length} total)`,
    );
    for (const run of duplicateRuns.slice(0, 5)) {
      console.log(
        `    - Freeze at ${run.timeS.toFixed(2)}s: frames ${run.startFrame}-${run.endFrame} (${run.length} identical)`,
      );
    }
    if (duplicateRuns.length > 5) {
      console.log(`    - ... and ${duplicateRuns.length - 5} more runs`);
    }
  } else {
    console.log(`  Frame uniqueness: all ${frames.length} frames unique`);
  }

  // Write frames.json
  const framesJsonPath = resolve(args.output, `frames-${timestamp}.json`);
  const framesJson = {
    mode: "deterministic",
    fps: args.fps,
    frameDurationMs,
    totalFrames: frames.length,
    uniqueFrames: uniqueHashes,
    reelDuration,
    outputDuration,
    prerollCutS: cutStartS,
    frames: frames.map((f) => ({
      path: f.path,
      virtualTimeMs: f.virtualTimeMs,
      idx: f.idx,
      hash: f.hash,
      tick: f.tick,
    })),
    pacing: framePacing,
  };
  await writeFile(framesJsonPath, JSON.stringify(framesJson, null, 2));
  console.log(`  Wrote ${basename(framesJsonPath)}`);

  // Build concat demuxer file with exact frame durations
  console.log("Building frame list with exact timing...");
  let concatContent = "";
  for (const frame of frames) {
    concatContent += `file '${frame.path}'\n`;
    concatContent += `duration ${frameDurationS.toFixed(6)}\n`;
  }
  if (frames.length > 0) {
    concatContent += `file '${frames[frames.length - 1].path}'\n`;
  }
  await writeFile(concatPath, concatContent);

  // Encode video
  console.log(`Encoding video at ${args.fps} fps...`);
  await new Promise((resolve, reject) => {
    const ffmpegVideo = spawn(
      "ffmpeg",
      [
        "-y",
        "-f",
        "concat",
        "-safe",
        "0",
        "-i",
        concatPath,
        "-fps_mode",
        "cfr",
        "-r",
        String(args.fps),
        "-c:v",
        "libx264",
        "-profile:v",
        "high",
        "-preset",
        "medium",
        "-crf",
        "20",
        "-pix_fmt",
        "yuv420p",
        "-movflags",
        "+faststart",
        rawVideoPath,
      ],
      { stdio: ["ignore", "pipe", "pipe"] },
    );

    let stderr = "";
    ffmpegVideo.stderr.on("data", (data) => {
      stderr += data.toString();
      const timeMatch = stderr.match(/time=(\d+:\d+:\d+\.\d+)/g);
      if (timeMatch) {
        process.stdout.write(`\r  Encoding: ${timeMatch[timeMatch.length - 1]}    `);
      }
    });

    ffmpegVideo.on("close", (code) => {
      process.stdout.write("\n");
      if (code === 0) resolve();
      else reject(new Error(`ffmpeg video exited with code ${code}`));
    });
    ffmpegVideo.on("error", reject);
  });
  console.log("  Video encoded");

  // Render audio offline from recorded commands
  let hasAudio = false;
  const rawAudioPath = resolve(args.output, `raw-audio-${timestamp}.wav`);
  const trimmedAudioPath = resolve(args.output, `trimmed-audio-${timestamp}.aac`);

  if (args.audio && audioCommands.length > 0) {
    console.log(`Rendering audio offline from ${audioCommands.length} recorded commands...`);

    // Render audio for the full reel duration (audio commands have virtual timestamps from 0)
    const audioBase64 = await page.evaluate(async (fullDurationS) => {
      const capture = window.__capture;
      if (!capture?.renderOfflineAudio) {
        console.log(`[deterministic] renderOfflineAudio not available`);
        return null;
      }

      try {
        const blob = await capture.renderOfflineAudio(fullDurationS);
        if (!blob) {
          console.log(`[deterministic] No audio rendered`);
          return null;
        }

        // Convert blob to base64
        const buffer = await blob.arrayBuffer();
        const bytes = new Uint8Array(buffer);
        let binary = "";
        const chunk = 8192;
        for (let i = 0; i < bytes.length; i += chunk) {
          binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
        }
        return btoa(binary);
      } catch (e) {
        console.error(`[deterministic] Audio rendering failed:`, e);
        return null;
      }
    }, reelDuration);

    if (audioBase64) {
      const audioData = Buffer.from(audioBase64, "base64");
      await writeFile(rawAudioPath, audioData);
      console.log(
        `  Audio rendered: ${(audioData.length / 1024 / 1024).toFixed(2)} MB WAV (full ${reelDuration.toFixed(2)}s reel)`,
      );

      // Process audio with ffmpeg: trim from preroll cut point, add limiter and fade
      const limiterFilter = "alimiter=limit=0.6:attack=0.1:release=50:level=false";
      const fadeOutDuration = 0.3;
      const fadeStartTime = Math.max(0, outputDuration - fadeOutDuration);
      const fadeFilter = `afade=t=out:st=${fadeStartTime.toFixed(3)}:d=${fadeOutDuration}`;

      await new Promise((resolve, reject) => {
        const ffmpegTrim = spawn(
          "ffmpeg",
          [
            "-y",
            "-ss",
            String(cutStartS), // Start from preroll cut point
            "-i",
            rawAudioPath,
            "-t",
            String(outputDuration),
            "-af",
            `${limiterFilter},${fadeFilter}`,
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            trimmedAudioPath,
          ],
          { stdio: "pipe" },
        );
        ffmpegTrim.on("close", (code) => {
          if (code === 0) resolve();
          else reject(new Error(`ffmpeg audio trim exited with code ${code}`));
        });
        ffmpegTrim.on("error", reject);
      });
      console.log(
        `  Audio: trimmed from ${cutStartS.toFixed(2)}s, duration ${outputDuration.toFixed(2)}s, fade at ${fadeStartTime.toFixed(2)}s`,
      );
      hasAudio = true;
    } else {
      console.log("  Note: Audio rendering returned no data");
    }
  }

  // Mux audio and video
  console.log("Muxing audio and video...");
  const muxArgs = hasAudio
    ? [
        "-y",
        "-i",
        rawVideoPath,
        "-i",
        trimmedAudioPath,
        "-c:v",
        "copy",
        "-c:a",
        "copy",
        "-movflags",
        "+faststart",
        finalPath,
      ]
    : ["-y", "-i", rawVideoPath, "-c:v", "copy", "-an", "-movflags", "+faststart", finalPath];

  await new Promise((resolve, reject) => {
    const ffmpegMux = spawn("ffmpeg", muxArgs, { stdio: "pipe" });
    ffmpegMux.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`ffmpeg mux exited with code ${code}`));
    });
    ffmpegMux.on("error", reject);
  });

  // Clean up (unless --keep-raw)
  if (!args.keepRaw) {
    await unlink(rawVideoPath).catch(() => {});
    await unlink(concatPath).catch(() => {});
    if (hasAudio) {
      await unlink(rawAudioPath).catch(() => {});
      await unlink(trimmedAudioPath).catch(() => {});
    }
    for (const frame of frames) {
      await unlink(frame.path).catch(() => {});
    }
    await import("node:fs/promises").then((fs) => fs.rmdir(framesDir).catch(() => {}));
  } else {
    console.log(
      `  Keeping raw files: ${framesDir}, ${basename(rawVideoPath)}${hasAudio ? `, ${basename(rawAudioPath)}` : ""}`,
    );
  }

  // Write sidecar
  const fireEvents = await page.evaluate(() => window.__capture?.fireEvents ?? []);
  const followUnitId = await page.evaluate(() => window.__capture?.followUnitId ?? null);
  const sidecarPath = finalPath.replace(/\.(mp4|webm)$/, "-fire-events.json");
  const sidecar = {
    mode: "deterministic",
    follow_unit_id: followUnitId,
    fps: args.fps,
    total_frames: frames.length,
    reel_duration: reelDuration,
    output_duration: outputDuration,
    preroll_cut_s: cutStartS,
    frame_pacing: framePacing,
    audio_commands: audioCommands.length,
    has_audio: hasAudio,
    total_events: fireEvents.length,
    followed_events: fireEvents.filter((e) => e.is_followed).length,
    events: fireEvents,
  };
  await writeFile(sidecarPath, JSON.stringify(sidecar, null, 2));
  console.log(`  Fire events: ${fireEvents.length} total -> ${basename(sidecarPath)}`);

  const { size } = await stat(finalPath);
  return { path: finalPath, size, fireEventsPath: fireEvents.length > 0 ? sidecarPath : null };
}

/** CDP-based page capture: captures the full page including DOM overlays.
 *  Uses Page.startScreencast for video frames with real timestamps, piped to
 *  ffmpeg via a concat demuxer for proper VFR-to-CFR conversion. Audio is
 *  captured via browser Web Audio API and muxed at the end. */
async function captureWithCDP(page, args) {
  const cdp = await page.context().newCDPSession(page);

  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  // Use scene in filename: capture-menu-preset or capture-skirmish-preset
  const sceneName = args.scene === "skirmish" ? "skirmish" : "menu";
  const rawVideoPath = resolve(args.output, `raw-${timestamp}.mp4`);
  const framesDir = resolve(args.output, `frames-${timestamp}`);
  const concatPath = resolve(args.output, `concat-${timestamp}.txt`);
  const finalPath = resolve(
    args.output,
    `capture-${sceneName}-${args.preset}-${timestamp}.${args.webm ? "webm" : "mp4"}`,
  );

  await mkdir(framesDir, { recursive: true });

  let droppedEarlyFrames = 0;
  let captureStartTime = null;
  let captureDuration = 0;
  const rawFrames = []; // { path, cdpTimestamp, writePromise }
  let frameIndex = 0;
  // The reel-ready timestamp from the page (performance.now() ms when first pose was rendered)
  let reelReadyTime = null;
  // CDP timestamp (seconds since epoch) when the reel became ready
  let reelReadyCdpTime = null;
  // Preroll cut: drop 0.5s of video/audio after ready to ensure clean start
  const PREROLL_CUT_S = 0.5;

  // Handle screencast frames with real timestamps
  // Ack immediately, write asynchronously to reduce screencast load
  cdp.on("Page.screencastFrame", async (event) => {
    // ACK IMMEDIATELY to reduce screencast backpressure
    cdp.send("Page.screencastFrameAck", { sessionId: event.sessionId }).catch(() => {});

    if (!captureStartTime) captureStartTime = Date.now();

    try {
      const frameTime = event.metadata.timestamp;

      // Check if we have the reel-ready signal yet, and convert to CDP timestamp space
      if (reelReadyCdpTime === null) {
        const readyInfo = await page.evaluate(() => {
          const reel = window.__capture?.reelReadyTime;
          if (reel === null || reel === undefined) return null;
          return {
            reelReadyTime: reel,
            performanceNow: performance.now(),
            dateNow: Date.now(),
          };
        });
        if (readyInfo !== null) {
          reelReadyTime = readyInfo.reelReadyTime;
          const pageLoadEpochMs = readyInfo.dateNow - readyInfo.performanceNow;
          reelReadyCdpTime = (pageLoadEpochMs + reelReadyTime) / 1000;
        }
      }

      // Save all frames asynchronously; track write promises for later await
      const frameData = Buffer.from(event.data, "base64");
      const idx = frameIndex++;
      const framePath = join(framesDir, `frame-${String(idx).padStart(6, "0")}.jpg`);
      const writePromise = writeFile(framePath, frameData);
      rawFrames.push({ path: framePath, cdpTimestamp: frameTime, writePromise, idx });

      // Update progress (don't block)
      const elapsed = (Date.now() - captureStartTime) / 1000;
      if (captureDuration > 0) {
        const pct = Math.min(100, (elapsed / captureDuration) * 100).toFixed(0);
        process.stdout.write(
          `\r  Recording: ${elapsed.toFixed(1)}s / ${captureDuration.toFixed(1)}s (${pct}%) [${rawFrames.length} frames]    `,
        );
      }
    } catch (err) {
      console.error("Frame error:", err.message);
    }
  });

  // Start screencast with reduced quality to lower bandwidth/CPU load
  await cdp.send("Page.startScreencast", {
    format: "jpeg",
    quality: 80, // Reduced from 95 to lower screencast load
    maxWidth: args.width,
    maxHeight: args.height,
    everyNthFrame: 1,
  });

  console.log("CDP screencast started, waiting for reel-ready signal...");

  // Wait for page to signal completion or get duration
  const duration = await page.evaluate(() => window.__capture?.duration);
  if (duration) captureDuration = duration;

  // Preroll sequence: reel-ready → audio start → wait 150ms → calibration → cut at calib + 400ms
  // This ensures the click is recorded in the audio tap

  // Step 1: Wait for reel to be ready
  console.log("  Waiting for reel-ready...");
  await page.waitForFunction(() => window.__capture?.reelReadyTime !== null, undefined, {
    timeout: 60000,
  });
  const reelReadyTimeFromPage = await page.evaluate(() => window.__capture?.reelReadyTime);
  console.log(`  Reel ready at ${reelReadyTimeFromPage?.toFixed(1) ?? "?"}ms`);

  // Step 2: Start audio recording
  console.log("  Starting audio recording...");
  await page.evaluate(() => {
    if (window.__capture?.startAudioOnly) {
      window.__capture.startAudioOnly();
    }
  });

  // Step 3: Wait for audio to actually start + 150ms settling time
  console.log("  Waiting for audio tap to start...");
  await page.waitForFunction(() => window.__capture?.audioStartTime !== null, undefined, {
    timeout: 10000,
  });
  const audioStartTimeFromPage = await page.evaluate(() => window.__capture?.audioStartTime);
  console.log(`  Audio started at ${audioStartTimeFromPage?.toFixed(1) ?? "?"}ms`);

  // Wait 150ms after audio starts for the tap to be fully active
  await new Promise((r) => setTimeout(r, 150));

  // Step 4: Trigger calibration BURST (5 flash/click pairs, 150ms apart)
  console.log("  Triggering calibration burst (5 flashes, 150ms apart)...");
  const calibrationTimes = await page.evaluate(async () => {
    if (window.__capture?.triggerCalibrationBurst) {
      return await window.__capture.triggerCalibrationBurst(5, 150);
    }
    return null;
  });

  // Step 5: Calculate video cut time = last calibration + 400ms (after flash ends)
  // This replaces the old PREROLL_CUT_S logic with calibration-based cut
  let dynamicCutTimeMs = null;
  const lastCalibTime = calibrationTimes?.[calibrationTimes.length - 1] ?? null;
  if (lastCalibTime !== null && audioStartTimeFromPage !== null) {
    dynamicCutTimeMs = lastCalibTime + 400; // 400ms after last flash
    console.log(`  Calibration burst: ${calibrationTimes.map((t) => t.toFixed(0)).join(", ")}ms`);
    console.log(`  Cut at ${dynamicCutTimeMs.toFixed(1)}ms (last flash + 400ms)`);

    // Verify ordering: audioStart < first calib < cut
    const firstCalibTime = calibrationTimes[0];
    const ordering = `audioStart(${audioStartTimeFromPage.toFixed(0)}) < calib[0](${firstCalibTime.toFixed(0)}) < cut(${dynamicCutTimeMs.toFixed(0)})`;
    if (audioStartTimeFromPage < firstCalibTime && firstCalibTime < dynamicCutTimeMs) {
      console.log(`  Ordering OK: ${ordering}`);
    } else {
      console.error(`  ERROR: Bad ordering: ${ordering}`);
      throw new Error(`Preroll ordering failed: ${ordering}`);
    }
  } else {
    console.log("  WARNING: Calibration failed, falling back to fixed preroll cut");
  }

  // Poll for completion with watchdog timeout
  // Normal capture should complete within captureDuration + extended tail (1.5s for end calib) + buffer
  const watchdogTimeoutMs = (captureDuration + 5) * 1000;
  const watchdogStart = Date.now();
  console.log(
    `  Watchdog: expecting completion within ${(watchdogTimeoutMs / 1000).toFixed(1)}s (duration=${captureDuration.toFixed(1)}s + 3s buffer)`,
  );

  try {
    await page.waitForFunction(
      () => {
        const capture = window.__capture;
        return (
          capture?.state === "complete" ||
          capture?.state === "error" ||
          capture?.state === "audio_complete"
        );
      },
      undefined,
      { timeout: watchdogTimeoutMs },
    );
  } catch {
    // Watchdog timeout: gather diagnostics and fail fast
    const elapsed = (Date.now() - watchdogStart) / 1000;
    const diag = await page.evaluate(() => {
      const capture = window.__capture;
      return {
        state: capture?.state ?? "undefined",
        reelReadyTime: capture?.reelReadyTime ?? null,
        reelEndTime: capture?.reelEndTime ?? null,
        audioStartTime: capture?.audioStartTime ?? null,
        duration: capture?.duration ?? null,
      };
    });
    console.error(`\n  WATCHDOG TIMEOUT after ${elapsed.toFixed(1)}s!`);
    console.error(`  Diagnostics:`);
    console.error(`    - state: ${diag.state}`);
    console.error(`    - reelReadyTime: ${diag.reelReadyTime?.toFixed(1) ?? "null"}ms`);
    console.error(`    - reelEndTime: ${diag.reelEndTime?.toFixed(1) ?? "null"}ms`);
    console.error(`    - audioStartTime: ${diag.audioStartTime?.toFixed(1) ?? "null"}ms`);
    console.error(`    - expected duration: ${diag.duration?.toFixed(1) ?? "null"}s`);
    if (diag.reelReadyTime !== null && diag.reelEndTime !== null) {
      const actualReelDuration = (diag.reelEndTime - diag.reelReadyTime) / 1000;
      console.error(`    - actual reel duration: ${actualReelDuration.toFixed(2)}s`);
    }
    throw new Error(
      `Capture watchdog timeout: page stuck in state '${diag.state}' after ${elapsed.toFixed(1)}s`,
    );
  }

  process.stdout.write("\n");

  // Stop screencast
  await cdp.send("Page.stopScreencast");

  // Await all pending frame writes
  console.log("  Waiting for frame writes to complete...");
  await Promise.all(rawFrames.map((f) => f.writePromise));

  // Calculate cut timestamp using dynamic calibration time or fallback to fixed preroll
  let cutCdpTime = null;
  if (dynamicCutTimeMs !== null && reelReadyCdpTime !== null && reelReadyTime !== null) {
    // Convert dynamicCutTimeMs (perf.now() space) to CDP timestamp
    const pageLoadEpochMs = reelReadyCdpTime * 1000 - reelReadyTime;
    cutCdpTime = (pageLoadEpochMs + dynamicCutTimeMs) / 1000;
    console.log(`  Video cut at CDP time ${cutCdpTime.toFixed(3)}s (dynamic)`);
  } else if (reelReadyCdpTime !== null) {
    // Fallback to fixed preroll cut
    cutCdpTime = reelReadyCdpTime + PREROLL_CUT_S;
    console.log(
      `  Video cut at CDP time ${cutCdpTime.toFixed(3)}s (fallback, preroll=${PREROLL_CUT_S}s)`,
    );
  }

  // Filter frames: drop those before cut timestamp
  let validFrames = rawFrames;
  if (cutCdpTime !== null) {
    validFrames = rawFrames.filter((f) => f.cdpTimestamp >= cutCdpTime);
    droppedEarlyFrames = rawFrames.length - validFrames.length;
  }

  // Sort by timestamp (should already be sorted, but ensure)
  validFrames.sort((a, b) => a.cdpTimestamp - b.cdpTimestamp);

  // Dedupe frames with same or very close timestamps (within 1ms)
  const dedupedFrames = [];
  let duplicateCount = 0;
  for (const frame of validFrames) {
    const last = dedupedFrames[dedupedFrames.length - 1];
    if (!last || frame.cdpTimestamp - last.cdpTimestamp > 0.001) {
      dedupedFrames.push(frame);
    } else {
      duplicateCount++;
    }
  }

  // Calculate target output duration: trim at exactly reel end
  // Use reelEndTime if available (precise), otherwise estimate from duration
  const reelEndTime = await page.evaluate(() => window.__capture?.reelEndTime);

  // Log reel timing diagnostic: compare expected vs actual reel end
  if (reelReadyTime !== null && captureDuration > 0) {
    const expectedReelEndMs = reelReadyTime + captureDuration * 1000;
    if (reelEndTime !== null && reelEndTime !== undefined) {
      const actualReelDurationMs = reelEndTime - reelReadyTime;
      const expectedReelDurationMs = captureDuration * 1000;
      const driftMs = actualReelDurationMs - expectedReelDurationMs;
      console.log(
        `  Reel timing: ready=${reelReadyTime.toFixed(0)}ms, end=${reelEndTime.toFixed(0)}ms`,
      );
      console.log(
        `    Expected duration: ${expectedReelDurationMs.toFixed(0)}ms, actual: ${actualReelDurationMs.toFixed(0)}ms, drift: ${driftMs >= 0 ? "+" : ""}${driftMs.toFixed(0)}ms`,
      );
    } else {
      console.log(
        `  Reel timing: ready=${reelReadyTime.toFixed(0)}ms, end=NOT_RECORDED (expected at ${expectedReelEndMs.toFixed(0)}ms)`,
      );
    }
  }

  let targetOutputDuration = null;
  if (dynamicCutTimeMs !== null && reelReadyTime !== null) {
    if (reelEndTime !== null && reelEndTime !== undefined) {
      // Precise: use the actual reel-end timestamp from the page
      const reelDurationMs = reelEndTime - reelReadyTime;
      const cutOffsetFromReelStart = (dynamicCutTimeMs - reelReadyTime) / 1000;
      targetOutputDuration = reelDurationMs / 1000 - cutOffsetFromReelStart;
      console.log(
        `  Target output duration: ${targetOutputDuration.toFixed(2)}s (reelEnd=${reelDurationMs.toFixed(0)}ms - cut=${(cutOffsetFromReelStart * 1000).toFixed(0)}ms)`,
      );
    } else if (captureDuration > 0) {
      // Fallback: estimate from captureDuration minus 0.5s tail
      const TAIL_DURATION_S = 0.5;
      const cutOffsetFromReelStart = (dynamicCutTimeMs - reelReadyTime) / 1000;
      targetOutputDuration = captureDuration - cutOffsetFromReelStart - TAIL_DURATION_S;
      console.log(
        `  Target output duration: ${targetOutputDuration.toFixed(2)}s (estimated: reel=${captureDuration}s - cut=${cutOffsetFromReelStart.toFixed(2)}s - tail=${TAIL_DURATION_S}s)`,
      );
    }
  }

  // Build frameTimestamps only from files that exist on disk
  const frameTimestamps = [];
  const firstValidTime = dedupedFrames[0]?.cdpTimestamp ?? 0;
  let droppedTailFrames = 0;
  for (const frame of dedupedFrames) {
    const timestamp_s = frame.cdpTimestamp - firstValidTime;
    // Trim frames past the target output duration (remove tail)
    if (targetOutputDuration !== null && timestamp_s > targetOutputDuration) {
      droppedTailFrames++;
      continue;
    }
    // Verify file exists before including
    try {
      await access(frame.path);
      frameTimestamps.push({
        path: frame.path,
        timestamp_s,
      });
    } catch {
      console.warn(`  Warning: frame file missing: ${frame.path}`);
    }
  }

  const frameCount = frameTimestamps.length;
  const actualDuration =
    frameTimestamps.length > 0 ? frameTimestamps[frameTimestamps.length - 1].timestamp_s : 0;
  console.log(
    `  Captured ${rawFrames.length} raw frames, kept ${frameCount} (dropped ${droppedEarlyFrames} early, ${droppedTailFrames} tail) over ${actualDuration.toFixed(2)}s`,
  );

  // Frame-pacing analysis: detect gaps in the OUTPUT frames (after tail trim)
  // This analyzes only frames that will be in the final video [cut, reelEnd]
  const framePacing = {
    totalFrames: validFrames.length,
    uniqueFrames: dedupedFrames.length,
    outputFrames: frameCount,
    duplicateFrames: duplicateCount,
    droppedEarlyFrames,
    droppedTailFrames,
    longestGapMs: 0,
    longestGapTimestamp: 0,
    longestGapIdx: 0,
    gaps: [], // Gaps > 25ms (missed frame threshold)
  };

  // Analyze pacing on frameTimestamps (output frames only, not tail)
  for (let i = 1; i < frameTimestamps.length; i++) {
    const prev = frameTimestamps[i - 1];
    const curr = frameTimestamps[i];
    const gapMs = (curr.timestamp_s - prev.timestamp_s) * 1000;

    if (gapMs > framePacing.longestGapMs) {
      framePacing.longestGapMs = gapMs;
      framePacing.longestGapTimestamp = prev.timestamp_s;
      framePacing.longestGapIdx = i - 1;
    }

    // Track gaps > 25ms (more than 1.5 frames at 60fps)
    if (gapMs > 25) {
      framePacing.gaps.push({
        idx: i - 1,
        timestamp_s: prev.timestamp_s,
        gapMs,
      });
    }
  }

  // Log frame-pacing report
  const avgFps = frameCount > 1 ? frameCount / actualDuration : 0;
  console.log(`  Frame pacing: ${frameCount} output frames, avg ${avgFps.toFixed(1)} fps`);
  if (framePacing.gaps.length > 0) {
    console.log(
      `    Gaps > 25ms: ${framePacing.gaps.length}, longest ${framePacing.longestGapMs.toFixed(1)}ms at ${framePacing.longestGapTimestamp.toFixed(2)}s (frame ${framePacing.longestGapIdx})`,
    );
  }

  // Write frames.json BEFORE any checks (so it's available even on failure)
  const framesJsonPath = resolve(args.output, `frames-${timestamp}.json`);
  const framesJson = {
    mode: "realtime",
    cutCdpTime,
    targetOutputDuration,
    pageLoadEpochMs:
      reelReadyCdpTime !== null && reelReadyTime !== null
        ? reelReadyCdpTime * 1000 - reelReadyTime
        : null,
    audioStartMs: await page.evaluate(() => window.__capture?.audioStartTime ?? null),
    // Output frames (after cut, before tail trim)
    frames: frameTimestamps.map((f, i) => ({
      path: f.path,
      timestamp_s: f.timestamp_s,
      idx: i,
    })),
    pacing: framePacing,
    // When --keep-raw, include all raw frames (including calibration/preroll)
    ...(args.keepRaw
      ? {
          rawFrames: rawFrames.map((f) => ({
            path: f.path,
            cdpTimestamp: f.cdpTimestamp,
            relativeTimestamp_s: f.cdpTimestamp - (rawFrames[0]?.cdpTimestamp ?? f.cdpTimestamp),
            idx: f.idx,
            isCalibration: f.cdpTimestamp < cutCdpTime,
            isOutput:
              f.cdpTimestamp >= cutCdpTime && frameTimestamps.some((ft) => ft.path === f.path),
          })),
          calibrationFrames: rawFrames
            .filter((f) => f.cdpTimestamp < cutCdpTime)
            .map((f, i) => ({
              path: f.path,
              cdpTimestamp: f.cdpTimestamp,
              relativeTimestamp_s: f.cdpTimestamp - (rawFrames[0]?.cdpTimestamp ?? f.cdpTimestamp),
              idx: i,
            })),
        }
      : {}),
  };
  await writeFile(framesJsonPath, JSON.stringify(framesJson, null, 2));
  console.log(
    `  Wrote ${basename(framesJsonPath)}${args.keepRaw ? ` (with ${rawFrames.filter((f) => f.cdpTimestamp < cutCdpTime).length} calibration frames)` : ""}`,
  );

  // FAIL if any gap > 50ms (frame starvation) in the OUTPUT range
  if (framePacing.longestGapMs > 50) {
    console.error(`  ERROR: Frame starvation detected in output range!`);
    console.error(
      `    Longest gap: ${framePacing.longestGapMs.toFixed(1)}ms at ${framePacing.longestGapTimestamp.toFixed(2)}s (frame ${framePacing.longestGapIdx})`,
    );
    throw new Error(
      `Frame starvation: ${framePacing.longestGapMs.toFixed(1)}ms gap at ${framePacing.longestGapTimestamp.toFixed(2)}s exceeds 50ms threshold`,
    );
  }

  // Frame-to-frame diff check in the last 1s: detect camera jump/reset
  // Use mean-abs-diff spike rule: fail if diff > 8 AND > 3× median of surrounding frames
  // On v22 this gives 13.2 at the jump vs ≤4 elsewhere
  if (frameTimestamps.length > 0 && targetOutputDuration !== null) {
    const last1sStart = Math.max(0, actualDuration - 1.0);
    const last1sFrames = frameTimestamps.filter((f) => f.timestamp_s >= last1sStart);
    if (last1sFrames.length >= 2) {
      console.log(`  Checking frame-to-frame diff in last 1s (${last1sFrames.length} frames)...`);

      // Calculate true mean-abs-diff using signalstats YAVG of blend=difference
      const diffs = [];
      for (let i = 1; i < last1sFrames.length; i++) {
        const prevFrame = last1sFrames[i - 1].path;
        const currFrame = last1sFrames[i].path;
        try {
          const statResult = await new Promise((resolve) => {
            const ffmpeg = spawn(
              "ffmpeg",
              [
                "-i",
                prevFrame,
                "-i",
                currFrame,
                "-filter_complex",
                "[0:v][1:v]blend=all_mode=difference[d];[d]signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-",
                "-f",
                "null",
                "-",
              ],
              { stdio: ["pipe", "pipe", "pipe"] },
            );
            let stdout = "";
            ffmpeg.stdout.on("data", (d) => (stdout += d));
            ffmpeg.on("close", () => resolve(stdout));
            ffmpeg.on("error", () => resolve(""));
          });
          const yavgMatch = statResult.match(/lavfi\.signalstats\.YAVG=([\d.]+)/);
          if (yavgMatch) {
            const meanAbsDiff = parseFloat(yavgMatch[1]);
            diffs.push({ index: i, time: last1sFrames[i].timestamp_s, diff: meanAbsDiff });
          }
        } catch {
          // Skip on error
        }
      }

      if (diffs.length > 0) {
        // Spike rule: fail if diff > 8 AND > 3× median of surrounding frames
        const sortedDiffs = [...diffs].sort((a, b) => a.diff - b.diff);
        const medianDiff = sortedDiffs[Math.floor(sortedDiffs.length / 2)]?.diff ?? 0;
        const maxDiff = Math.max(...diffs.map((d) => d.diff));

        // Check each frame against the spike rule
        const spikes = diffs.filter((d) => d.diff > 8 && d.diff > medianDiff * 3);
        if (spikes.length > 0) {
          const worstSpike = spikes.reduce((a, b) => (a.diff > b.diff ? a : b));
          console.error(`  ERROR: Camera jump detected at ${worstSpike.time.toFixed(2)}s`);
          console.error(
            `    - Mean-abs-diff: ${worstSpike.diff.toFixed(1)} (median=${medianDiff.toFixed(1)}, threshold=max(8, ${(medianDiff * 3).toFixed(1)}))`,
          );
          throw new Error(
            `Camera jump detected in last 1s at ${worstSpike.time.toFixed(2)}s (diff=${worstSpike.diff.toFixed(1)} > 8 and > 3×median)`,
          );
        } else {
          console.log(
            `  Frame diff check OK (max=${maxDiff.toFixed(1)}, median=${medianDiff.toFixed(1)})`,
          );
        }
      }
    }
  }

  // Build concat demuxer file with per-frame durations
  console.log("Building frame list with real timestamps...");
  const minDuration = 1 / 120; // Clamp to >= 1/120s to avoid negative/zero durations
  let concatContent = "";
  for (let i = 0; i < frameTimestamps.length; i++) {
    const frame = frameTimestamps[i];
    const nextFrame = frameTimestamps[i + 1];
    // Duration is time to next frame, or 1/fps for the last frame
    let duration = nextFrame ? nextFrame.timestamp_s - frame.timestamp_s : 1 / args.fps;
    // Clamp to minimum to prevent ffmpeg errors with negative/zero durations
    duration = Math.max(minDuration, duration);
    concatContent += `file '${frame.path}'\n`;
    concatContent += `duration ${duration.toFixed(6)}\n`;
  }
  // Concat demuxer needs last file repeated without duration
  if (frameTimestamps.length > 0) {
    concatContent += `file '${frameTimestamps[frameTimestamps.length - 1].path}'\n`;
  }
  await writeFile(concatPath, concatContent);

  // Encode video with proper timing using concat demuxer
  // High profile, CRF 20, faststart for web playback
  // Use -fps_mode cfr -r to force constant frame rate (VFR to CFR conversion)
  console.log(`Encoding video with real frame timing at ${args.fps} fps...`);
  await new Promise((resolve, reject) => {
    const ffmpegVideo = spawn(
      "ffmpeg",
      [
        "-y",
        "-f",
        "concat",
        "-safe",
        "0",
        "-i",
        concatPath,
        "-fps_mode",
        "cfr",
        "-r",
        String(args.fps),
        "-c:v",
        "libx264",
        "-profile:v",
        "high",
        "-preset",
        "medium",
        "-crf",
        "20",
        "-pix_fmt",
        "yuv420p",
        "-movflags",
        "+faststart",
        rawVideoPath,
      ],
      { stdio: ["ignore", "pipe", "pipe"] },
    );

    let stderr = "";
    ffmpegVideo.stderr.on("data", (data) => {
      stderr += data.toString();
      const timeMatch = stderr.match(/time=(\d+:\d+:\d+\.\d+)/g);
      if (timeMatch) {
        process.stdout.write(`\r  Encoding: ${timeMatch[timeMatch.length - 1]}    `);
      }
    });

    ffmpegVideo.on("close", (code) => {
      process.stdout.write("\n");
      if (code === 0) resolve();
      else reject(new Error(`ffmpeg video exited with code ${code}`));
    });
    ffmpegVideo.on("error", reject);
  });

  console.log("  Video encoded");

  // Get audio from browser if enabled
  let hasAudio = false;
  let audioOffsetMs = 0;
  let audioLatencyMs = 0;
  let startMedianOffsetMs = null;
  let endMedianOffsetMs = null;
  let clockRatio = 1.0;
  const rawAudioPath = resolve(args.output, `raw-audio-${timestamp}.webm`);
  const trimmedAudioPath = resolve(args.output, `trimmed-audio-${timestamp}.aac`);
  if (args.audio) {
    const audioInfo = await page.evaluate(async () => {
      const capture = window.__capture;
      if (!capture?.audioBlob) return null;
      const buffer = await capture.audioBlob.arrayBuffer();
      const bytes = new Uint8Array(buffer);
      let binary = "";
      const chunk = 8192;
      for (let i = 0; i < bytes.length; i += chunk) {
        binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
      }
      return {
        audioBase64: btoa(binary),
        audioStartTime: capture.audioStartTime ?? null,
        reelReadyTime: capture.reelReadyTime ?? null,
        reelEndTime: capture.reelEndTime ?? null,
        audioLatency: capture.audioLatency ?? 0,
        calibrationTimes: capture.calibrationTimes ?? [],
        endCalibrationTimes: capture.endCalibrationTimes ?? [],
      };
    });

    if (audioInfo?.audioBase64) {
      const audioData = Buffer.from(audioInfo.audioBase64, "base64");
      await writeFile(rawAudioPath, audioData);
      console.log("  Audio captured, detecting calibration bursts...");

      // Audio output latency from AudioContext (outputLatency + baseLatency)
      audioLatencyMs = (audioInfo.audioLatency ?? 0) * 1000;

      // Convert perf.now timestamps to CDP time using pageLoadEpochMs
      const pageLoadEpochMs =
        audioInfo.reelReadyTime !== null && reelReadyCdpTime !== null
          ? reelReadyCdpTime * 1000 - audioInfo.reelReadyTime
          : null;
      const audioStartMs = audioInfo.audioStartTime ?? 0;

      // Helper: detect white frame ONSETS (first white frame after non-white)
      // Returns array of { cdpTimestamp, yavg, onsetMs } where onsetMs is relative to audioStart
      async function detectFlashOnsets(expectedTimesMs, label) {
        if (!expectedTimesMs || expectedTimesMs.length === 0 || pageLoadEpochMs === null) return [];

        // Search range: from first trigger to last trigger + 150ms (enough for render latency)
        const expectedCdpTimes = expectedTimesMs.map((t) => (pageLoadEpochMs + t) / 1000);
        const minCdp = Math.min(...expectedCdpTimes) - 0.02; // 20ms before first
        const maxCdp = Math.max(...expectedCdpTimes) + 0.15; // 150ms after last
        const searchFrames = rawFrames
          .filter((f) => f.cdpTimestamp >= minCdp && f.cdpTimestamp <= maxCdp)
          .sort((a, b) => a.cdpTimestamp - b.cdpTimestamp);
        console.log(
          `    ${label}: searching ${searchFrames.length} frames in CDP [${minCdp.toFixed(3)}, ${maxCdp.toFixed(3)}]`,
        );

        // Analyze all frames for brightness
        const frameBrightness = [];
        for (const frame of searchFrames) {
          try {
            const result = await new Promise((resolve) => {
              const ffmpeg = spawn(
                "ffmpeg",
                [
                  "-i",
                  frame.path,
                  "-vf",
                  "signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-",
                  "-f",
                  "null",
                  "-",
                ],
                { stdio: ["pipe", "pipe", "pipe"] },
              );
              let stdout = "";
              ffmpeg.stdout.on("data", (d) => (stdout += d));
              ffmpeg.on("close", () => resolve(stdout));
              ffmpeg.on("error", () => resolve(""));
            });
            const yavgMatch = result.match(/lavfi\.signalstats\.YAVG=([\d.]+)/);
            const yavg = yavgMatch ? parseFloat(yavgMatch[1]) : 0;
            frameBrightness.push({ cdpTimestamp: frame.cdpTimestamp, yavg });
          } catch {
            frameBrightness.push({ cdpTimestamp: frame.cdpTimestamp, yavg: 0 });
          }
        }

        // Detect onsets: first frame with YAVG > 250 after a frame with YAVG < 200
        const onsets = [];
        for (let i = 1; i < frameBrightness.length; i++) {
          const prev = frameBrightness[i - 1];
          const curr = frameBrightness[i];
          if (prev.yavg < 200 && curr.yavg > 250) {
            const onsetMs = curr.cdpTimestamp * 1000 - pageLoadEpochMs - audioStartMs;
            onsets.push({ cdpTimestamp: curr.cdpTimestamp, yavg: curr.yavg, onsetMs });
          }
        }
        console.log(
          `    ${label}: found ${onsets.length} flash onsets at [${onsets.map((o) => o.onsetMs.toFixed(0)).join(", ")}]ms`,
        );
        return onsets;
      }

      // Helper: detect click onsets in audio
      // Returns array of { timeMs, peakDb } where timeMs is relative to audioStart
      async function detectClickOnsets(expectedTimesMs, label) {
        if (!expectedTimesMs || expectedTimesMs.length === 0) return [];

        // Search range: 50ms before first trigger to 150ms after last trigger
        const relativeTimesMs = expectedTimesMs.map((t) => t - audioStartMs);
        const searchStartMs = Math.max(0, Math.min(...relativeTimesMs) - 50);
        const searchEndMs = Math.max(...relativeTimesMs) + 150;
        const searchStartS = searchStartMs / 1000;
        const searchDurationS = (searchEndMs - searchStartMs) / 1000;

        console.log(
          `    ${label}: searching audio ${searchStartS.toFixed(3)}s - ${(searchStartS + searchDurationS).toFixed(3)}s`,
        );

        const clicks = [];
        try {
          const astatsResult = await new Promise((resolve, reject) => {
            const ffmpeg = spawn(
              "ffmpeg",
              [
                "-ss",
                String(searchStartS),
                "-t",
                String(searchDurationS),
                "-i",
                rawAudioPath,
                "-af",
                "asetnsamples=n=240,astats=metadata=1:reset=1,ametadata=print:key=lavfi.astats.Overall.Peak_level:file=-",
                "-f",
                "null",
                "-",
              ],
              { stdio: ["pipe", "pipe", "pipe"] },
            );
            let stdout = "";
            ffmpeg.stdout.on("data", (d) => (stdout += d));
            ffmpeg.on("close", (code) => resolve({ stdout, code }));
            ffmpeg.on("error", (e) => reject(e));
          });

          if (astatsResult.code !== 0) return [];

          const peakPattern = /pts_time:([\d.]+)\s*\nlavfi\.astats\.Overall\.Peak_level=([-\d.]+)/g;
          let peakMatch;
          let lastOnsetMs = -1000;
          while ((peakMatch = peakPattern.exec(astatsResult.stdout)) !== null) {
            const ptsTime = parseFloat(peakMatch[1]);
            const peakLevel = parseFloat(peakMatch[2]);
            const absoluteMs = (searchStartS + ptsTime) * 1000;
            // Onset detection: > -10dB and at least 80ms since last
            if (peakLevel > -10 && absoluteMs - lastOnsetMs > 80) {
              clicks.push({ timeMs: absoluteMs, peakDb: peakLevel });
              lastOnsetMs = absoluteMs;
            }
          }
        } catch {
          /* ignore */
        }
        console.log(
          `    ${label}: found ${clicks.length} click onsets at [${clicks.map((c) => c.timeMs.toFixed(0)).join(", ")}]ms`,
        );
        return clicks;
      }

      // Helper: match triggers to detected onsets
      // For each trigger, find first onset in [trigger, trigger+120ms] for video
      // or [trigger-20ms, trigger+120ms] for audio
      function matchTriggersToOnsets(triggerTimesMs, onsets, label, isVideo) {
        const matched = [];
        const usedOnsets = new Set();

        for (let i = 0; i < triggerTimesMs.length; i++) {
          const triggerRelMs = triggerTimesMs[i] - audioStartMs;
          const windowStart = isVideo ? triggerRelMs : triggerRelMs - 20;
          const windowEnd = triggerRelMs + 120;

          // Find first unused onset in window
          const onset = onsets.find((o) => {
            const t = isVideo ? o.onsetMs : o.timeMs;
            return t >= windowStart && t <= windowEnd && !usedOnsets.has(o);
          });

          if (onset) {
            usedOnsets.add(onset);
            matched.push({ triggerIdx: i, triggerMs: triggerRelMs, onset });
          }
        }
        console.log(`    ${label}: matched ${matched.length}/${triggerTimesMs.length} triggers`);
        return matched;
      }

      // Detect START calibration burst (5 flash/click pairs)
      // Use the local calibrationTimes from the page.evaluate that triggered them,
      // NOT audioInfo.calibrationTimes which reads from React state (may be empty)
      console.log("  Detecting START calibration burst...");
      const startCalibTimes = calibrationTimes ?? [];
      const startFlashOnsets = await detectFlashOnsets(startCalibTimes, "START video");
      const startClickOnsets = await detectClickOnsets(startCalibTimes, "START audio");

      const startVideoMatches = matchTriggersToOnsets(
        startCalibTimes,
        startFlashOnsets,
        "START video",
        true,
      );
      const startAudioMatches = matchTriggersToOnsets(
        startCalibTimes,
        startClickOnsets,
        "START audio",
        false,
      );

      // Pair up video and audio matches by trigger index
      const startPairOffsets = [];
      const startMatchedPairs = [];
      for (const vm of startVideoMatches) {
        const am = startAudioMatches.find((a) => a.triggerIdx === vm.triggerIdx);
        if (am) {
          const videoOnsetMs = vm.onset.onsetMs;
          const audioOnsetMs = am.onset.timeMs;
          const offset = videoOnsetMs - audioOnsetMs;
          startPairOffsets.push(offset);
          startMatchedPairs.push({
            idx: vm.triggerIdx,
            videoMs: videoOnsetMs,
            audioMs: audioOnsetMs,
            offset,
          });
        }
      }

      console.log(
        `  START matched pairs: ${startMatchedPairs.map((p) => `[${p.idx}] v=${p.videoMs.toFixed(0)} a=${p.audioMs.toFixed(0)} Δ=${p.offset.toFixed(1)}`).join(", ")}`,
      );

      if (startPairOffsets.length >= 3) {
        const sorted = [...startPairOffsets].sort((a, b) => a - b);
        startMedianOffsetMs = sorted[Math.floor(sorted.length / 2)];
        console.log(
          `  START calibration: ${startPairOffsets.length} pairs, median offset=${startMedianOffsetMs.toFixed(1)}ms`,
        );
      } else {
        console.error(`  ERROR: Only ${startPairOffsets.length} matched START pairs (need ≥3)`);
        throw new Error(
          `START calibration failed: only ${startPairOffsets.length} matched pairs (need ≥3)`,
        );
      }

      // Detect END calibration burst for drift measurement
      // Get END calibration times directly from page (triggered after reel ended)
      console.log("  Detecting END calibration burst...");
      const endCalibTimes = await page.evaluate(() => window.__capture?.endCalibrationTimes ?? []);

      if (endCalibTimes.length > 0) {
        const endFlashOnsets = await detectFlashOnsets(endCalibTimes, "END video");
        const endClickOnsets = await detectClickOnsets(endCalibTimes, "END audio");

        const endVideoMatches = matchTriggersToOnsets(
          endCalibTimes,
          endFlashOnsets,
          "END video",
          true,
        );
        const endAudioMatches = matchTriggersToOnsets(
          endCalibTimes,
          endClickOnsets,
          "END audio",
          false,
        );

        const endPairOffsets = [];
        const endMatchedPairs = [];
        for (const vm of endVideoMatches) {
          const am = endAudioMatches.find((a) => a.triggerIdx === vm.triggerIdx);
          if (am) {
            const videoOnsetMs = vm.onset.onsetMs;
            const audioOnsetMs = am.onset.timeMs;
            const offset = videoOnsetMs - audioOnsetMs;
            endPairOffsets.push(offset);
            endMatchedPairs.push({
              idx: vm.triggerIdx,
              videoMs: videoOnsetMs,
              audioMs: audioOnsetMs,
              offset,
            });
          }
        }

        console.log(
          `  END matched pairs: ${endMatchedPairs.map((p) => `[${p.idx}] v=${p.videoMs.toFixed(0)} a=${p.audioMs.toFixed(0)} Δ=${p.offset.toFixed(1)}`).join(", ")}`,
        );

        if (endPairOffsets.length >= 3) {
          const sorted = [...endPairOffsets].sort((a, b) => a - b);
          endMedianOffsetMs = sorted[Math.floor(sorted.length / 2)];
          console.log(
            `  END calibration: ${endPairOffsets.length} pairs, median offset=${endMedianOffsetMs.toFixed(1)}ms`,
          );

          // Compute drift ratio
          const driftMs = endMedianOffsetMs - startMedianOffsetMs;
          const startMidMs = startCalibTimes[Math.floor(startCalibTimes.length / 2)];
          const endMidMs = endCalibTimes[Math.floor(endCalibTimes.length / 2)];
          const spanMs = endMidMs - startMidMs;

          if (spanMs > 1000) {
            clockRatio = 1 - driftMs / spanMs;
            console.log(
              `  Drift: ${driftMs.toFixed(1)}ms over ${(spanMs / 1000).toFixed(1)}s, clockRatio=${clockRatio.toFixed(6)}`,
            );
          } else {
            console.log("  WARNING: Span too short for drift, using ratio 1.0");
          }
        } else {
          console.log(
            `  WARNING: Only ${endPairOffsets.length} matched END pairs (need ≥3 for drift)`,
          );
        }
      } else {
        console.log("  WARNING: No END calibration times");
      }

      // Calculate sync offset
      const videoCutTime =
        dynamicCutTimeMs !== null
          ? dynamicCutTimeMs
          : audioInfo.reelReadyTime !== null
            ? audioInfo.reelReadyTime + PREROLL_CUT_S * 1000
            : null;

      const cutOffsetMs =
        videoCutTime !== null ? videoCutTime - audioStartMs : PREROLL_CUT_S * 1000;
      audioOffsetMs = cutOffsetMs - startMedianOffsetMs;
      console.log(
        `  Final audio offset: ${audioOffsetMs.toFixed(1)}ms (cut=${cutOffsetMs.toFixed(1)}ms - calib=${startMedianOffsetMs.toFixed(1)}ms)`,
      );

      // Build ffmpeg filter chain with drift correction
      const limiterFilter = "alimiter=limit=0.6:attack=0.1:release=50:level=false";
      const fadeOutDuration = 0.3;
      const audioTargetDuration = targetOutputDuration ?? actualDuration;
      const fadeStartTime = Math.max(0, audioTargetDuration - fadeOutDuration);
      const fadeFilter = `afade=t=out:st=${fadeStartTime.toFixed(3)}:d=${fadeOutDuration}`;

      // Apply atempo for drift correction if ratio differs from 1.0 by more than 0.1%
      const needsDriftCorrection = Math.abs(clockRatio - 1.0) > 0.001;
      const atempoFilter = needsDriftCorrection ? `atempo=${clockRatio.toFixed(6)}` : null;

      if (audioOffsetMs >= 0) {
        const trimS = audioOffsetMs / 1000;
        const filterChain = atempoFilter
          ? `${atempoFilter},${limiterFilter},${fadeFilter}`
          : `${limiterFilter},${fadeFilter}`;
        await new Promise((resolve, reject) => {
          const ffmpegTrim = spawn(
            "ffmpeg",
            [
              "-y",
              "-ss",
              String(trimS),
              "-i",
              rawAudioPath,
              "-t",
              String(audioTargetDuration),
              "-af",
              filterChain,
              "-c:a",
              "aac",
              "-b:a",
              "192k",
              trimmedAudioPath,
            ],
            { stdio: "pipe" },
          );
          ffmpegTrim.on("close", (code) => {
            if (code === 0) resolve();
            else reject(new Error(`ffmpeg audio trim exited with code ${code}`));
          });
          ffmpegTrim.on("error", reject);
        });
        console.log(
          `  Audio: trimmed ${trimS.toFixed(3)}s, duration ${audioTargetDuration.toFixed(2)}s, fade at ${fadeStartTime.toFixed(2)}s${needsDriftCorrection ? `, atempo=${clockRatio.toFixed(6)}` : ""}`,
        );
      } else {
        const delayMs = Math.abs(audioOffsetMs);
        const delayFilter = `adelay=${Math.round(delayMs)}:all=1`;
        const filterChain = atempoFilter
          ? `${delayFilter},${atempoFilter},${limiterFilter},${fadeFilter}`
          : `${delayFilter},${limiterFilter},${fadeFilter}`;
        await new Promise((resolve, reject) => {
          const ffmpegDelay = spawn(
            "ffmpeg",
            [
              "-y",
              "-i",
              rawAudioPath,
              "-t",
              String(audioTargetDuration + delayMs / 1000),
              "-af",
              filterChain,
              "-c:a",
              "aac",
              "-b:a",
              "192k",
              trimmedAudioPath,
            ],
            { stdio: "pipe" },
          );
          ffmpegDelay.on("close", (code) => {
            if (code === 0) resolve();
            else reject(new Error(`ffmpeg audio delay exited with code ${code}`));
          });
          ffmpegDelay.on("error", reject);
        });
        console.log(
          `  Audio: delayed ${delayMs.toFixed(1)}ms, duration ${audioTargetDuration.toFixed(2)}s${needsDriftCorrection ? `, atempo=${clockRatio.toFixed(6)}` : ""}`,
        );
      }
      hasAudio = true;
    }
  }

  // Mux audio and video
  console.log("Muxing audio and video...");

  const muxArgs = hasAudio
    ? [
        "-y",
        "-i",
        rawVideoPath,
        "-i",
        trimmedAudioPath,
        "-c:v",
        "copy",
        "-c:a",
        "copy",
        "-movflags",
        "+faststart",
        finalPath,
      ]
    : ["-y", "-i", rawVideoPath, "-c:v", "copy", "-an", "-movflags", "+faststart", finalPath];

  await new Promise((resolve, reject) => {
    const ffmpegMux = spawn("ffmpeg", muxArgs, { stdio: "pipe" });
    ffmpegMux.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`ffmpeg mux exited with code ${code}`));
    });
    ffmpegMux.on("error", reject);
  });

  // Clean up intermediate files (unless --keep-raw)
  if (!args.keepRaw) {
    await unlink(rawVideoPath).catch(() => {});
    await unlink(concatPath).catch(() => {});
    if (hasAudio) {
      await unlink(rawAudioPath).catch(() => {});
      await unlink(trimmedAudioPath).catch(() => {});
    }
    // Clean up ALL frame files (including dropped ones)
    for (const frame of rawFrames) {
      await unlink(frame.path).catch(() => {});
    }
    await import("node:fs/promises").then((fs) => fs.rmdir(framesDir).catch(() => {}));
  } else {
    console.log(
      `  Keeping raw files: ${framesDir}, ${basename(rawVideoPath)}${hasAudio ? `, ${basename(rawAudioPath)}` : ""}`,
    );
  }

  // Get and write weapon fire events to sidecar JSON
  const fireEvents = await page.evaluate(() => window.__capture?.fireEvents ?? []);
  const followUnitId = await page.evaluate(() => window.__capture?.followUnitId ?? null);
  const pageReelReadyTime = await page.evaluate(() => window.__capture?.reelReadyTime ?? null);
  const pageAudioStartTime = await page.evaluate(() => window.__capture?.audioStartTime ?? null);
  const sidecarPath = finalPath.replace(/\.(mp4|webm)$/, "-fire-events.json");
  const sidecar = {
    follow_unit_id: followUnitId,
    reel_ready_time_ms: pageReelReadyTime,
    audio_start_time_ms: pageAudioStartTime,
    audio_latency_ms: audioLatencyMs,
    start_median_offset_ms: startMedianOffsetMs,
    end_median_offset_ms: endMedianOffsetMs,
    clock_ratio: clockRatio,
    audio_offset_ms: audioOffsetMs,
    video_cut_ms: dynamicCutTimeMs ?? PREROLL_CUT_S * 1000,
    dropped_early_frames: droppedEarlyFrames,
    frame_pacing: {
      output_frames: framePacing.outputFrames,
      unique_frames: framePacing.uniqueFrames,
      duplicate_frames: framePacing.duplicateFrames,
      dropped_early_frames: framePacing.droppedEarlyFrames,
      dropped_tail_frames: framePacing.droppedTailFrames,
      longest_gap_ms: framePacing.longestGapMs,
      longest_gap_timestamp_s: framePacing.longestGapTimestamp,
      gaps_over_25ms: framePacing.gaps.length,
      avg_fps: avgFps,
    },
    total_events: fireEvents.length,
    followed_events: fireEvents.filter((e) => e.is_followed).length,
    events: fireEvents,
  };
  await writeFile(sidecarPath, JSON.stringify(sidecar, null, 2));
  console.log(
    `  Fire events: ${fireEvents.length} total, ${sidecar.followed_events} from followed unit (id: ${followUnitId ?? "none"}) -> ${basename(sidecarPath)}`,
  );
  if (droppedEarlyFrames > 0) {
    console.log(
      `  Dropped ${droppedEarlyFrames} early frames (before reel-ready + ${PREROLL_CUT_S}s)`,
    );
  }
  if (sidecar.followed_events === 0) {
    console.log("  WARNING: Followed unit did not fire during capture");
  }

  const { size } = await stat(finalPath);
  return { path: finalPath, size, fireEventsPath: fireEvents.length > 0 ? sidecarPath : null };
}

async function main() {
  const args = parseArgs(process.argv);

  // Check prerequisites before starting
  if (!(await checkPrerequisites())) {
    process.exit(1);
  }

  console.log("Starting cinematic capture...");
  console.log(`  Scene: ${args.scene}`);
  console.log(`  Preset: ${args.preset}`);
  console.log(`  Resolution: ${args.width}x${args.height}`);
  console.log(`  FPS: ${args.fps}`);
  console.log(`  Audio: ${args.audio}`);
  console.log(`  Cards: ${args.cards}`);
  console.log(`  Output format: ${args.webm ? "WebM" : "MP4"}`);
  console.log(
    `  Capture mode: ${args.deterministic ? "deterministic (virtual clock)" : "real-time (CDP screencast)"}`,
  );

  if (!hasFfmpeg()) {
    console.error("Error: ffmpeg is required for page-level capture");
    process.exit(1);
  }

  await mkdir(args.output, { recursive: true });

  // Build native tools (mechanics validator, map workbench) before starting
  // A stale tool would read today's schema with yesterday's code and fail
  console.log("Building native tools...");
  await ensureNativeTools();

  const server = await startServer(false);
  console.log(`Server running at ${server.url}`);

  const browser = await launchBrowser({ args: [] });

  try {
    const context = await browser.newContext({
      viewport: { width: args.width, height: args.height },
      deviceScaleFactor: 1,
    });
    const page = await context.newPage();

    const params = new URLSearchParams({
      capture: args.preset,
      "capture-width": String(args.width),
      "capture-height": String(args.height),
      "capture-fps": String(args.fps),
      "capture-audio": String(args.audio),
      "capture-cards": String(args.cards),
      "capture-grade": String(args.grade),
      "capture-mode": args.deterministic ? "deterministic" : "page",
    });
    if (args.scene === "skirmish") {
      params.set("capture-scene", "skirmish");
    }

    const route = args.scene === "skirmish" ? "/labs/capture-skirmish" : "/labs/capture";
    const url = `${server.url}${route}?${params}`;
    console.log(`Opening ${url}`);

    // Forward page console messages to stderr for debugging
    page.on("console", (msg) => {
      const type = msg.type();
      const text = msg.text();
      // Skip noisy messages
      if (text.includes("DevTools") || text.includes("[HMR]")) return;
      if (type === "error" || type === "warning") {
        console.error(`[page ${type}] ${text}`);
      }
    });

    // Forward page errors to stderr
    page.on("pageerror", (error) => {
      console.error(`[page error] ${error.message}`);
    });

    // Monitor network errors, especially /__mechanics validation failures
    let networkError = null;
    page.on("response", async (response) => {
      if (response.status() >= 400) {
        const reqUrl = response.url();
        if (reqUrl.includes("__mechanics")) {
          try {
            const body = await response.text();
            networkError = `/__mechanics validation failed (${response.status()}): ${body}`;
          } catch {
            networkError = `/__mechanics validation failed with status ${response.status()}`;
          }
        }
      }
    });

    await page.goto(url);

    // Check for network error immediately
    if (networkError) {
      throw new Error(networkError);
    }

    // Wait for lab to be ready
    console.log("Waiting for scene to prepare...");
    // Poll for state periodically to report progress
    const pollStart = Date.now();
    let lastState = null;
    const pollInterval = setInterval(async () => {
      try {
        const state = await page.evaluate(() => ({
          hasLab: !!window.__lab,
          ready: window.__lab?.ready ?? false,
          error: window.__lab?.error ?? null,
          hasCapture: !!window.__capture,
          captureState: window.__capture?.state ?? null,
          captureDuration: window.__capture?.duration ?? null,
        }));
        if (JSON.stringify(state) !== JSON.stringify(lastState)) {
          const elapsed = ((Date.now() - pollStart) / 1000).toFixed(1);
          console.log(`  [${elapsed}s] lab=${state.hasLab}, ready=${state.ready}, error=${state.error}, capture=${state.captureState}`);
          lastState = state;
        }
      } catch {
        // Page might be navigating
      }
    }, 5000);
    
    try {
      await page.waitForFunction(() => window.__lab?.ready || window.__lab?.error, undefined, {
        timeout: 120_000,
      });
    } catch (waitError) {
      clearInterval(pollInterval);
      // Gather diagnostic info on timeout
      const diag = await page.evaluate(() => ({
        hasLab: !!window.__lab,
        ready: window.__lab?.ready ?? false,
        error: window.__lab?.error ?? null,
        hasCapture: !!window.__capture,
        captureState: window.__capture?.state ?? null,
      })).catch(() => ({ error: "could not evaluate" }));
      console.error("  Timeout diagnostics:", JSON.stringify(diag));
      // Check if we have a network error that explains the timeout
      if (networkError) {
        throw new Error(networkError);
      }
      throw waitError;
    }
    clearInterval(pollInterval);

    // Check for network error after wait
    if (networkError) {
      throw new Error(networkError);
    }

    const labError = await page.evaluate(() => window.__lab?.error);
    if (labError) {
      throw new Error(`Lab error: ${labError}`);
    }

    // Choose capture mode
    const result = args.deterministic
      ? await captureWithDeterministic(page, args)
      : await captureWithCDP(page, args);

    console.log(`\nCapture complete!`);
    console.log(`  Output: ${result.path}`);
    console.log(`  Size: ${(result.size / 1024 / 1024).toFixed(2)} MB`);

    await context.close();
  } finally {
    await browser.close();
    await server.close();
  }
}

main().catch((error) => {
  console.error("Capture failed:", error.message);
  process.exit(1);
});
