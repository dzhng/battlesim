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
import { spawn, spawnSync, execSync } from "node:child_process";
import { existsSync, createWriteStream } from "node:fs";
import { startServer } from "./scene.mjs";
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
  const wasmPath = new URL("web/dist/presentation_wasm_bg.wasm", ROOT);
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
  if (!await checkMechanicsBuilt()) {
    await runBuild("mechanics validator", "build:mechanics");
  }
  if (!await checkWasmBuilt()) {
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
  const assetDirs = [
    new URL("assets/sounds/", ROOT),
    new URL("assets/models/", ROOT),
  ];
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

/** Transcode WebM to MP4 using ffmpeg. Returns the output path and dropped frame count. */
async function transcodeToMp4(webmPath, fps) {
  const mp4Path = webmPath.replace(/\.webm$/, ".mp4");
  
  console.log("Transcoding to MP4 (H.264 + AAC)...");
  console.log("  Note: Real-time capture may have variable frame rate; ffmpeg will normalize to constant " + fps + " fps.");
  
  return new Promise((resolve, reject) => {
    const ffmpeg = spawn("ffmpeg", [
      "-y",
      "-i", webmPath,
      "-c:v", "libx264",
      "-preset", "medium",
      "-crf", "18",
      "-r", String(fps),
      "-c:a", "aac",
      "-b:a", "192k",
      "-movflags", "+faststart",
      mp4Path,
    ], { stdio: ["ignore", "pipe", "pipe"] });
    
    let stderr = "";
    let droppedFrames = 0;
    ffmpeg.stderr.on("data", (data) => {
      stderr += data.toString();
      const timeMatch = stderr.match(/time=(\d+:\d+:\d+\.\d+)/g);
      if (timeMatch) {
        process.stdout.write(`\r  Progress: ${timeMatch[timeMatch.length - 1]}    `);
      }
      // Count dropped/duplicated frames from ffmpeg warnings
      const dropMatch = data.toString().match(/dropping frame|dropping.*frames|duplicating frame|dup=/gi);
      if (dropMatch) {
        droppedFrames += dropMatch.length;
      }
    });
    
    ffmpeg.on("close", (code) => {
      process.stdout.write("\n");
      if (droppedFrames > 0) {
        console.log(`  Frame adjustment: ${droppedFrames} frames dropped/duplicated during re-timing`);
      }
      if (code === 0) {
        resolve({ path: mp4Path, droppedFrames });
      } else {
        reject(new Error(`ffmpeg exited with code ${code}`));
      }
    });
    
    ffmpeg.on("error", reject);
  });
}

/** CDP-based page capture: captures the full page including DOM overlays.
 *  Uses Page.startScreencast for video frames with real timestamps, piped to
 *  ffmpeg via a concat demuxer for proper VFR-to-CFR conversion. Audio is
 *  captured via browser Web Audio API and muxed at the end. */
async function captureWithCDP(page, args, server) {
  const cdp = await page.context().newCDPSession(page);
  
  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  // Use scene in filename: capture-menu-preset or capture-skirmish-preset
  const sceneName = args.scene === "skirmish" ? "skirmish" : "menu";
  const rawVideoPath = resolve(args.output, `raw-${timestamp}.mp4`);
  const audioPath = resolve(args.output, `audio-${timestamp}.webm`);
  const framesDir = resolve(args.output, `frames-${timestamp}`);
  const concatPath = resolve(args.output, `concat-${timestamp}.txt`);
  const finalPath = resolve(args.output, `capture-${sceneName}-${args.preset}-${timestamp}.${args.webm ? "webm" : "mp4"}`);
  
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
  cdp.on("Page.screencastFrame", async (event) => {
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
      
      // Save all frames; track write promises for later await
      const frameData = Buffer.from(event.data, "base64");
      const idx = frameIndex++;
      const framePath = join(framesDir, `frame-${String(idx).padStart(6, "0")}.jpg`);
      const writePromise = writeFile(framePath, frameData);
      rawFrames.push({ path: framePath, cdpTimestamp: frameTime, writePromise });
      
      // Acknowledge frame to receive next one (don't wait for write)
      await cdp.send("Page.screencastFrameAck", { sessionId: event.sessionId });
      
      // Update progress
      const elapsed = (Date.now() - captureStartTime) / 1000;
      if (captureDuration > 0) {
        const pct = Math.min(100, (elapsed / captureDuration) * 100).toFixed(0);
        process.stdout.write(`\r  Recording: ${elapsed.toFixed(1)}s / ${captureDuration.toFixed(1)}s (${pct}%) [${rawFrames.length} frames]    `);
      }
    } catch (err) {
      console.error("Frame error:", err.message);
    }
  });
  
  // Start screencast
  await cdp.send("Page.startScreencast", {
    format: "jpeg",
    quality: 95,
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
  await page.waitForFunction(() => window.__capture?.reelReadyTime !== null, undefined, { timeout: 60000 });
  const reelReadyTimeFromPage = await page.evaluate(() => window.__capture?.reelReadyTime);
  console.log(`  Reel ready at ${reelReadyTimeFromPage?.toFixed(1) ?? '?'}ms`);
  
  // Step 2: Start audio recording
  console.log("  Starting audio recording...");
  await page.evaluate(() => {
    if (window.__capture?.startAudioOnly) {
      window.__capture.startAudioOnly();
    }
  });
  
  // Step 3: Wait for audio to actually start + 150ms settling time
  console.log("  Waiting for audio tap to start...");
  await page.waitForFunction(() => window.__capture?.audioStartTime !== null, undefined, { timeout: 10000 });
  const audioStartTimeFromPage = await page.evaluate(() => window.__capture?.audioStartTime);
  console.log(`  Audio started at ${audioStartTimeFromPage?.toFixed(1) ?? '?'}ms`);
  
  // Wait 150ms after audio starts for the tap to be fully active
  await new Promise(r => setTimeout(r, 150));
  
  // Step 4: Trigger calibration (white flash + click)
  console.log("  Triggering calibration...");
  const calibrationTime = await page.evaluate(() => {
    if (window.__capture?.triggerCalibration) {
      return window.__capture.triggerCalibration();
    }
    return null;
  });
  
  // Step 5: Calculate video cut time = calibrationTime + 400ms (after flash ends)
  // This replaces the old PREROLL_CUT_S logic with calibration-based cut
  let dynamicCutTimeMs = null;
  if (calibrationTime !== null && audioStartTimeFromPage !== null) {
    dynamicCutTimeMs = calibrationTime + 400; // 400ms after flash
    console.log(`  Calibration at ${calibrationTime.toFixed(1)}ms, cut at ${dynamicCutTimeMs.toFixed(1)}ms`);
    
    // Verify ordering: audioStart < calib < cut
    const ordering = `audioStart(${audioStartTimeFromPage.toFixed(0)}) < calib(${calibrationTime.toFixed(0)}) < cut(${dynamicCutTimeMs.toFixed(0)})`;
    if (audioStartTimeFromPage < calibrationTime && calibrationTime < dynamicCutTimeMs) {
      console.log(`  Ordering OK: ${ordering}`);
    } else {
      console.error(`  ERROR: Bad ordering: ${ordering}`);
      throw new Error(`Preroll ordering failed: ${ordering}`);
    }
  } else {
    console.log("  WARNING: Calibration failed, falling back to fixed preroll cut");
  }
  
  // Poll for completion
  await page.waitForFunction(
    () => {
      const capture = window.__capture;
      return capture?.state === "complete" || capture?.state === "error" || capture?.state === "audio_complete";
    },
    undefined,
    { timeout: 300_000 },
  );
  
  process.stdout.write("\n");
  
  // Stop screencast
  await cdp.send("Page.stopScreencast");
  
  // Await all pending frame writes
  console.log("  Waiting for frame writes to complete...");
  await Promise.all(rawFrames.map(f => f.writePromise));
  
  // Calculate cut timestamp using dynamic calibration time or fallback to fixed preroll
  let cutCdpTime = null;
  if (dynamicCutTimeMs !== null && reelReadyCdpTime !== null && reelReadyTime !== null) {
    // Convert dynamicCutTimeMs (perf.now() space) to CDP timestamp
    const pageLoadEpochMs = (reelReadyCdpTime * 1000) - reelReadyTime;
    cutCdpTime = (pageLoadEpochMs + dynamicCutTimeMs) / 1000;
    console.log(`  Video cut at CDP time ${cutCdpTime.toFixed(3)}s (dynamic)`);
  } else if (reelReadyCdpTime !== null) {
    // Fallback to fixed preroll cut
    cutCdpTime = reelReadyCdpTime + PREROLL_CUT_S;
    console.log(`  Video cut at CDP time ${cutCdpTime.toFixed(3)}s (fallback, preroll=${PREROLL_CUT_S}s)`);
  }
  
  // Filter frames: drop those before cut timestamp
  let validFrames = rawFrames;
  if (cutCdpTime !== null) {
    validFrames = rawFrames.filter(f => f.cdpTimestamp >= cutCdpTime);
    droppedEarlyFrames = rawFrames.length - validFrames.length;
  }
  
  // Sort by timestamp (should already be sorted, but ensure)
  validFrames.sort((a, b) => a.cdpTimestamp - b.cdpTimestamp);
  
  // Dedupe frames with same or very close timestamps (within 1ms)
  const dedupedFrames = [];
  for (const frame of validFrames) {
    const last = dedupedFrames[dedupedFrames.length - 1];
    if (!last || frame.cdpTimestamp - last.cdpTimestamp > 0.001) {
      dedupedFrames.push(frame);
    }
    // Don't delete duplicate files yet - we'll clean up at the end
  }
  
  // Calculate target output duration: trim at exactly reel end, no tail
  // The recording runs for captureDuration + 0.5s tail, so subtract 0.5s
  // Target = (reelDuration - 0.5s tail) - (cutTime - reelReadyTime)
  const TAIL_DURATION_S = 0.5;
  let targetOutputDuration = null;
  if (dynamicCutTimeMs !== null && reelReadyTime !== null && captureDuration > 0) {
    const cutOffsetFromReelStart = (dynamicCutTimeMs - reelReadyTime) / 1000;
    // Subtract tail duration to get the real reel end (before camera reset)
    targetOutputDuration = captureDuration - cutOffsetFromReelStart - TAIL_DURATION_S;
    console.log(`  Target output duration: ${targetOutputDuration.toFixed(2)}s (reel=${captureDuration}s - cut=${cutOffsetFromReelStart.toFixed(2)}s - tail=${TAIL_DURATION_S}s)`);
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
  const actualDuration = frameTimestamps.length > 0 
    ? frameTimestamps[frameTimestamps.length - 1].timestamp_s 
    : 0;
  console.log(`  Captured ${rawFrames.length} raw frames, kept ${frameCount} (dropped ${droppedEarlyFrames} early, ${droppedTailFrames} tail) over ${actualDuration.toFixed(2)}s`);
  
  // Frame-to-frame diff check in the last 1s: detect camera jump/reset
  // Use mean-abs-diff spike rule: fail if any frame diff > 3× median of surrounding 30 frames AND > 8
  if (frameTimestamps.length > 0 && targetOutputDuration !== null) {
    const last1sStart = Math.max(0, actualDuration - 1.0);
    const last1sFrames = frameTimestamps.filter(f => f.timestamp_s >= last1sStart);
    if (last1sFrames.length >= 2) {
      console.log(`  Checking frame-to-frame diff in last 1s (${last1sFrames.length} frames)...`);
      
      // Calculate mean-abs-diff for each consecutive pair using ssim filter (gives diff)
      const diffs = [];
      for (let i = 1; i < last1sFrames.length; i++) {
        const prevFrame = last1sFrames[i - 1].path;
        const currFrame = last1sFrames[i].path;
        try {
          // Use blend filter to compute absolute difference, then compute mean
          const diffResult = await new Promise((resolve, reject) => {
            const ffmpeg = spawn("ffmpeg", [
              "-i", prevFrame,
              "-i", currFrame,
              "-lavfi", "[0:v][1:v]blend=all_mode=difference,blackframe=amount=0:threshold=1",
              "-f", "null", "-",
            ], { stdio: ["pipe", "pipe", "pipe"] });
            let stderr = "";
            ffmpeg.stderr.on("data", (d) => stderr += d);
            ffmpeg.on("close", () => resolve(stderr));
            ffmpeg.on("error", () => resolve(""));
          });
          // Parse blackframe output: "pblack:XX" where higher = more similar (more black in diff)
          // If no blackframe match, use signalstats for mean
          const pblackMatch = diffResult.match(/pblack:\s*(\d+)/);
          if (pblackMatch) {
            // pblack is % of black pixels, so diff = 100 - pblack (higher = more different)
            const meanAbsDiff = 100 - parseInt(pblackMatch[1]);
            diffs.push({ index: i, time: last1sFrames[i].timestamp_s, diff: meanAbsDiff });
          } else {
            // Fallback: use signalstats YAVG diff
            const statResult = await new Promise((resolve, reject) => {
              const ffmpeg = spawn("ffmpeg", [
                "-i", prevFrame,
                "-i", currFrame,
                "-filter_complex", "[0:v][1:v]blend=all_mode=difference[d];[d]signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-",
                "-f", "null", "-",
              ], { stdio: ["pipe", "pipe", "pipe"] });
              let stdout = "";
              ffmpeg.stdout.on("data", (d) => stdout += d);
              ffmpeg.on("close", () => resolve(stdout));
              ffmpeg.on("error", () => resolve(""));
            });
            const yavgMatch = statResult.match(/lavfi\.signalstats\.YAVG=([\d.]+)/);
            if (yavgMatch) {
              const meanAbsDiff = parseFloat(yavgMatch[1]);
              diffs.push({ index: i, time: last1sFrames[i].timestamp_s, diff: meanAbsDiff });
            }
          }
        } catch (e) {
          // Skip on error
        }
      }
      
      if (diffs.length > 0) {
        // Find spikes: diff > 3× median of surrounding 30 frames AND > 8
        const sortedDiffs = [...diffs].sort((a, b) => a.diff - b.diff);
        const medianDiff = sortedDiffs[Math.floor(sortedDiffs.length / 2)]?.diff ?? 0;
        const maxDiff = Math.max(...diffs.map(d => d.diff));
        const spikeThreshold = Math.max(8, medianDiff * 3);
        
        const spikes = diffs.filter(d => d.diff > spikeThreshold);
        if (spikes.length > 0) {
          const worstSpike = spikes.reduce((a, b) => a.diff > b.diff ? a : b);
          console.error(`  ERROR: Camera jump detected at ${worstSpike.time.toFixed(2)}s`);
          console.error(`    - Mean-abs-diff: ${worstSpike.diff.toFixed(1)} (median=${medianDiff.toFixed(1)}, threshold=${spikeThreshold.toFixed(1)})`);
          throw new Error(`Camera jump detected in last 1s at ${worstSpike.time.toFixed(2)}s (diff=${worstSpike.diff.toFixed(1)} > ${spikeThreshold.toFixed(1)})`);
        } else {
          console.log(`  Frame diff check OK (max=${maxDiff.toFixed(1)}, median=${medianDiff.toFixed(1)}, threshold=${spikeThreshold.toFixed(1)})`);
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
    let duration = nextFrame 
      ? (nextFrame.timestamp_s - frame.timestamp_s) 
      : (1 / args.fps);
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
    const ffmpegVideo = spawn("ffmpeg", [
      "-y",
      "-f", "concat",
      "-safe", "0",
      "-i", concatPath,
      "-fps_mode", "cfr",
      "-r", String(args.fps),
      "-c:v", "libx264",
      "-profile:v", "high",
      "-preset", "medium",
      "-crf", "20",
      "-pix_fmt", "yuv420p",
      "-movflags", "+faststart",
      rawVideoPath,
    ], { stdio: ["ignore", "pipe", "pipe"] });
    
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
  let calibVideoMs = null;
  let calibAudioMs = null;
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
        audioLatency: capture.audioLatency ?? 0,
        calibrationTime: capture.calibrationTime ?? null,
      };
    });
    
    if (audioInfo?.audioBase64) {
      const audioData = Buffer.from(audioInfo.audioBase64, "base64");
      await writeFile(rawAudioPath, audioData);
      console.log("  Audio captured, detecting calibration signals...");
      
      // Audio output latency from AudioContext (outputLatency + baseLatency)
      audioLatencyMs = (audioInfo.audioLatency ?? 0) * 1000;
      
      // Expected click time relative to audio start
      const expectedClickMs = audioInfo.calibrationTime !== null && audioInfo.audioStartTime !== null
        ? audioInfo.calibrationTime - audioInfo.audioStartTime
        : null;
      
      // Detect calibration white frame by scanning ALL captured frames (including pre-cut)
      // The flash happens before the cut point, so we must scan the full frame set
      console.log("  Detecting calibration white frame in frames...");
      
      // Search frames around the expected calibration time
      const expectedCalibCdpTime = calibrationTime !== null && reelReadyCdpTime !== null && reelReadyTime !== null
        ? (((reelReadyCdpTime * 1000) - reelReadyTime + calibrationTime) / 1000)
        : null;
      
      // Filter frames to search: within ±500ms of expected calibration time
      const searchFrames = expectedCalibCdpTime !== null
        ? rawFrames.filter(f => Math.abs(f.cdpTimestamp - expectedCalibCdpTime) < 0.5)
        : rawFrames.slice(0, Math.min(100, rawFrames.length)); // fallback: first 100 frames
      
      console.log(`    Searching ${searchFrames.length} frames near expected calib time ${expectedCalibCdpTime?.toFixed(3) ?? '?'}s`);
      
      // Analyze each frame's brightness using ffmpeg signalstats
      let whiteFrame = null;
      for (const frame of searchFrames) {
        try {
          const result = await new Promise((resolve, reject) => {
            const ffmpeg = spawn("ffmpeg", [
              "-i", frame.path,
              "-vf", "signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-",
              "-f", "null", "-",
            ], { stdio: ["pipe", "pipe", "pipe"] });
            let stdout = "";
            ffmpeg.stdout.on("data", (d) => stdout += d);
            ffmpeg.on("close", () => resolve(stdout));
            ffmpeg.on("error", () => resolve(""));
          });
          
          // Parse YAVG from metadata output
          const yavgMatch = result.match(/lavfi\.signalstats\.YAVG=([\d.]+)/);
          if (yavgMatch) {
            const yavg = parseFloat(yavgMatch[1]);
            if (yavg > 250) {
              whiteFrame = { cdpTimestamp: frame.cdpTimestamp, yavg };
              console.log(`    White frame found: cdpTime=${frame.cdpTimestamp.toFixed(3)}s, YAVG=${yavg.toFixed(0)}`);
              break;
            }
          }
        } catch (e) {
          // Continue searching
        }
      }
      
      if (whiteFrame !== null) {
        // Convert white frame CDP timestamp to ms relative to audio start
        const pageLoadEpochMs = audioInfo.reelReadyTime !== null && reelReadyCdpTime !== null
          ? (reelReadyCdpTime * 1000) - audioInfo.reelReadyTime
          : null;
        if (pageLoadEpochMs !== null && audioInfo.audioStartTime !== null) {
          const framePerfNow = (whiteFrame.cdpTimestamp * 1000) - pageLoadEpochMs;
          calibVideoMs = framePerfNow - audioInfo.audioStartTime;
          console.log(`  Calibration white frame at ${calibVideoMs.toFixed(1)}ms relative to audio start (YAVG=${whiteFrame.yavg.toFixed(0)})`);
        }
      } else {
        console.log("  WARNING: No white frame (YAVG > 250) found in frames");
      }
      
      // Detect calibration click in audio by finding strongest transient
      // Search within ±300ms of expected click time
      console.log("  Detecting calibration click in audio...");
      if (expectedClickMs !== null) {
        try {
          // Use astats with 5ms windows (240 samples at 48kHz) to find click onset
          const searchStartS = Math.max(0, (expectedClickMs - 300) / 1000);
          const searchDurationS = 0.6; // 600ms window
          
          // Put -ss/-t before -i for input seeking (more efficient)
          // asetnsamples=n=240 gives 5ms windows at 48kHz
          const astatsResult = await new Promise((resolve, reject) => {
            const ffmpeg = spawn("ffmpeg", [
              "-ss", String(searchStartS),
              "-t", String(searchDurationS),
              "-i", rawAudioPath,
              "-af", "asetnsamples=n=240,astats=metadata=1:reset=1,ametadata=print:key=lavfi.astats.Overall.Peak_level:file=-",
              "-f", "null", "-",
            ], { stdio: ["pipe", "pipe", "pipe"] });
            let stdout = "";
            let stderr = "";
            ffmpeg.stdout.on("data", (d) => stdout += d);
            ffmpeg.stderr.on("data", (d) => stderr += d);
            ffmpeg.on("close", (code) => resolve({ stdout, stderr, code }));
            ffmpeg.on("error", (e) => reject(e));
          });
          
          // Check for ffmpeg errors
          if (astatsResult.code !== 0) {
            console.error(`  ERROR: ffmpeg astats exited with code ${astatsResult.code}`);
            console.error(`  stderr: ${astatsResult.stderr.slice(-500)}`);
            throw new Error(`ffmpeg astats failed with code ${astatsResult.code}`);
          }
          
          // Parse peak levels and find the FIRST window > -10dB (onset detection)
          // Format: frame:N pts:P pts_time:T\nlavfi.astats.Overall.Peak_level=V
          const peakPattern = /pts_time:([\d.]+)\s*\nlavfi\.astats\.Overall\.Peak_level=([-\d.]+)/g;
          let peakMatch;
          let onsetTime = null;
          let onsetPeak = -100;
          while ((peakMatch = peakPattern.exec(astatsResult.stdout)) !== null) {
            const ptsTime = parseFloat(peakMatch[1]);
            const peakLevel = parseFloat(peakMatch[2]);
            // Find FIRST window that exceeds -10dB (onset, not max)
            if (peakLevel > -10 && onsetTime === null) {
              // Add searchStartS to get absolute time in audio file
              onsetTime = searchStartS + ptsTime;
              onsetPeak = peakLevel;
              break; // Stop at first loud segment
            }
          }
          
          if (onsetTime !== null) {
            calibAudioMs = onsetTime * 1000;
            console.log(`  Calibration click DETECTED at audio ${calibAudioMs.toFixed(1)}ms (onset peak=${onsetPeak.toFixed(1)}dB)`);
          } else {
            // No onset found - this is a failure, not a fallback
            console.error(`  ERROR: No click onset (> -10dB) found in search window ${searchStartS.toFixed(3)}s - ${(searchStartS + searchDurationS).toFixed(3)}s`);
            console.error(`    Expected click at ${expectedClickMs.toFixed(0)}ms relative to audio start`);
            console.error(`    Parsed ${(astatsResult.stdout.match(/Peak_level/g) || []).length} peak values from astats`);
            throw new Error(`Click detection failed: no onset > -10dB found near expected time ${expectedClickMs.toFixed(0)}ms`);
          }
        } catch (e) {
          if (e.message.includes("Click detection failed") || e.message.includes("ffmpeg astats failed")) throw e;
          console.error(`  ERROR: Audio click detection failed: ${e.message}`);
          throw new Error(`Click detection error: ${e.message}`);
        }
      } else {
        console.error("  ERROR: Cannot detect click - missing calibrationTime or audioStartTime");
        throw new Error("Click detection failed: missing calibrationTime or audioStartTime");
      }
      
      // Calculate sync offset from calibration or fall back to timing-based calculation
      // Use dynamic cut time (calibrationTime + 400ms) if available, else fallback
      const videoCutTime = dynamicCutTimeMs !== null
        ? dynamicCutTimeMs
        : (audioInfo.reelReadyTime !== null ? audioInfo.reelReadyTime + PREROLL_CUT_S * 1000 : null);
      const audioStartTime = audioInfo.audioStartTime;
      
      if (calibVideoMs !== null && calibAudioMs !== null) {
        // Calibration measures A/V drift: calibOffsetMs = calibVideoMs - calibAudioMs
        // If positive: video event appears later, meaning audio is early (ahead)
        // If negative: audio event appears later, meaning audio is late (behind)
        // Since audio is early when calibOffsetMs > 0, we need LESS trim, so SUBTRACT it
        const calibOffsetMs = calibVideoMs - calibAudioMs;
        // The base offset is how much to trim to align with the video cut point
        const cutOffsetMs = videoCutTime !== null && audioStartTime !== null
          ? (videoCutTime - audioStartTime)
          : (dynamicCutTimeMs !== null ? dynamicCutTimeMs - (audioStartTime ?? 0) : PREROLL_CUT_S * 1000);
        // Subtract calibOffsetMs because calibration already accounts for pipeline latency
        audioOffsetMs = cutOffsetMs - calibOffsetMs;
        console.log(`  Calibration offset: ${calibOffsetMs.toFixed(1)}ms (video=${calibVideoMs.toFixed(1)}ms, audio=${calibAudioMs.toFixed(1)}ms)`);
        console.log(`  Final audio offset: ${audioOffsetMs.toFixed(1)}ms (cut=${cutOffsetMs.toFixed(1)}ms - calib=${calibOffsetMs.toFixed(1)}ms)`);
        
        // Plausibility check: calibration offset should be within ±300ms
        // (The total offset legitimately includes the ~600ms cut offset)
        if (Math.abs(calibOffsetMs) > 300) {
          console.error(`  ERROR: Calibration offset ${calibOffsetMs.toFixed(1)}ms is implausible (|calib| > 300ms)`);
          console.error(`    - calibVideoMs: ${calibVideoMs.toFixed(1)}ms`);
          console.error(`    - calibAudioMs: ${calibAudioMs.toFixed(1)}ms`);
          console.error(`    - cutOffsetMs: ${cutOffsetMs.toFixed(1)}ms`);
          console.error(`    - videoCutTime: ${videoCutTime?.toFixed(0) ?? 'null'}ms`);
          console.error(`    - audioStartTime: ${audioStartTime?.toFixed(0) ?? 'null'}ms`);
          throw new Error(`Implausible calibration offset: ${calibOffsetMs.toFixed(1)}ms (expected |calib| < 300ms)`);
        }
      } else {
        // Calibration detection failed - this is an error condition
        const videoDetected = calibVideoMs !== null;
        const audioDetected = calibAudioMs !== null;
        console.error(`  ERROR: Calibration detection failed!`);
        console.error(`    - White frame detected: ${videoDetected} (${calibVideoMs !== null ? calibVideoMs.toFixed(1) + 'ms' : 'not found'})`);
        console.error(`    - Click detected: ${audioDetected} (${calibAudioMs !== null ? calibAudioMs.toFixed(1) + 'ms' : 'not found'})`);
        throw new Error(`Calibration failed: video=${videoDetected}, audio=${audioDetected}. Cannot determine A/V sync offset.`);
      }
      
      // Build ffmpeg filter chain:
      // - alimiter at limit=0.6 (~-4.4 dBFS) prevents AAC from exceeding -0.5 dBFS after decode
      // - afade for 0.3s fade-out at the end (matching video trim point)
      const limiterFilter = "alimiter=limit=0.6:attack=0.1:release=50:level=false";
      
      // Calculate audio duration to match video (target output duration)
      // Add 0.3s fade-out ending at the trim point
      const fadeOutDuration = 0.3;
      const audioTargetDuration = targetOutputDuration ?? actualDuration;
      const fadeStartTime = Math.max(0, audioTargetDuration - fadeOutDuration);
      const fadeFilter = `afade=t=out:st=${fadeStartTime.toFixed(3)}:d=${fadeOutDuration}`;
      
      if (audioOffsetMs >= 0) {
        // Audio started early: trim from the beginning, and trim at the end
        const trimS = audioOffsetMs / 1000;
        await new Promise((resolve, reject) => {
          const ffmpegTrim = spawn("ffmpeg", [
            "-y",
            "-i", rawAudioPath,
            "-ss", String(trimS),
            "-t", String(audioTargetDuration),
            "-af", `${limiterFilter},${fadeFilter}`,
            "-c:a", "aac",
            "-b:a", "192k",
            trimmedAudioPath,
          ], { stdio: "pipe" });
          ffmpegTrim.on("close", (code) => {
            if (code === 0) resolve();
            else reject(new Error(`ffmpeg audio trim exited with code ${code}`));
          });
          ffmpegTrim.on("error", reject);
        });
        console.log(`  Audio: trimmed ${trimS.toFixed(3)}s from start, duration ${audioTargetDuration.toFixed(2)}s, fade-out at ${fadeStartTime.toFixed(2)}s`);
      } else {
        // Audio started late: delay (pad) the beginning, and trim at the end
        const delayMs = Math.abs(audioOffsetMs);
        // adelay adds silence to the start; use "all=1" to apply to all channels
        const delayFilter = `adelay=${Math.round(delayMs)}:all=1`;
        await new Promise((resolve, reject) => {
          const ffmpegDelay = spawn("ffmpeg", [
            "-y",
            "-i", rawAudioPath,
            "-t", String(audioTargetDuration + delayMs / 1000), // Account for delay
            "-af", `${delayFilter},${limiterFilter},${fadeFilter}`,
            "-c:a", "aac",
            "-b:a", "192k",
            trimmedAudioPath,
          ], { stdio: "pipe" });
          ffmpegDelay.on("close", (code) => {
            if (code === 0) resolve();
            else reject(new Error(`ffmpeg audio delay exited with code ${code}`));
          });
          ffmpegDelay.on("error", reject);
        });
        console.log(`  Audio: delayed ${delayMs.toFixed(1)}ms, duration ${audioTargetDuration.toFixed(2)}s, fade-out at ${fadeStartTime.toFixed(2)}s`);
      }
      hasAudio = true;
    }
  }
  
  // Mux audio and video
  console.log("Muxing audio and video...");
  
  const muxArgs = hasAudio
    ? [
        "-y",
        "-i", rawVideoPath,
        "-i", trimmedAudioPath,
        "-c:v", "copy",
        "-c:a", "copy",
        "-movflags", "+faststart",
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
  
  // Clean up intermediate files
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
  await import("node:fs/promises").then(fs => fs.rmdir(framesDir).catch(() => {}));
  
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
    calib_video_ms: calibVideoMs,
    calib_audio_ms: calibAudioMs,
    audio_offset_ms: audioOffsetMs,
    video_cut_ms: dynamicCutTimeMs ?? (PREROLL_CUT_S * 1000),
    dropped_early_frames: droppedEarlyFrames,
    total_events: fireEvents.length,
    followed_events: fireEvents.filter(e => e.is_followed).length,
    events: fireEvents,
  };
  await writeFile(sidecarPath, JSON.stringify(sidecar, null, 2));
  console.log(`  Fire events: ${fireEvents.length} total, ${sidecar.followed_events} from followed unit (id: ${followUnitId ?? 'none'}) -> ${basename(sidecarPath)}`);
  if (droppedEarlyFrames > 0) {
    console.log(`  Dropped ${droppedEarlyFrames} early frames (before reel-ready + ${PREROLL_CUT_S}s)`);
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
  if (!await checkPrerequisites()) {
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
  console.log(`  Capture mode: CDP page-level (includes DOM overlays)`);

  if (!hasFfmpeg()) {
    console.error("Error: ffmpeg is required for page-level capture");
    process.exit(1);
  }

  await mkdir(args.output, { recursive: true });

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
      "capture-mode": "page",
    });
    if (args.scene === "skirmish") {
      params.set("capture-scene", "skirmish");
    }

    const route = args.scene === "skirmish" ? "/labs/capture-skirmish" : "/labs/capture";
    const url = `${server.url}${route}?${params}`;
    console.log(`Opening ${url}`);

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
    try {
      await page.waitForFunction(
        () => window.__lab?.ready || window.__lab?.error,
        undefined,
        { timeout: 120_000 },
      );
    } catch (waitError) {
      // Check if we have a network error that explains the timeout
      if (networkError) {
        throw new Error(networkError);
      }
      throw waitError;
    }

    // Check for network error after wait
    if (networkError) {
      throw new Error(networkError);
    }

    const labError = await page.evaluate(() => window.__lab?.error);
    if (labError) {
      throw new Error(`Lab error: ${labError}`);
    }

    // Use CDP page-level capture
    const result = await captureWithCDP(page, args, server);

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
