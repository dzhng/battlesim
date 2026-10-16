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
    console.log(`  Calibration burst: ${calibrationTimes.map(t => t.toFixed(0)).join(", ")}ms`);
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
  console.log(`  Watchdog: expecting completion within ${(watchdogTimeoutMs / 1000).toFixed(1)}s (duration=${captureDuration.toFixed(1)}s + 3s buffer)`);
  
  try {
    await page.waitForFunction(
      () => {
        const capture = window.__capture;
        return capture?.state === "complete" || capture?.state === "error" || capture?.state === "audio_complete";
      },
      undefined,
      { timeout: watchdogTimeoutMs },
    );
  } catch (watchdogError) {
    // Watchdog timeout: gather diagnostics and fail fast
    const elapsed = (Date.now() - watchdogStart) / 1000;
    const diag = await page.evaluate(() => {
      const capture = window.__capture;
      return {
        state: capture?.state ?? 'undefined',
        reelReadyTime: capture?.reelReadyTime ?? null,
        reelEndTime: capture?.reelEndTime ?? null,
        audioStartTime: capture?.audioStartTime ?? null,
        duration: capture?.duration ?? null,
      };
    });
    console.error(`\n  WATCHDOG TIMEOUT after ${elapsed.toFixed(1)}s!`);
    console.error(`  Diagnostics:`);
    console.error(`    - state: ${diag.state}`);
    console.error(`    - reelReadyTime: ${diag.reelReadyTime?.toFixed(1) ?? 'null'}ms`);
    console.error(`    - reelEndTime: ${diag.reelEndTime?.toFixed(1) ?? 'null'}ms`);
    console.error(`    - audioStartTime: ${diag.audioStartTime?.toFixed(1) ?? 'null'}ms`);
    console.error(`    - expected duration: ${diag.duration?.toFixed(1) ?? 'null'}s`);
    if (diag.reelReadyTime !== null && diag.reelEndTime !== null) {
      const actualReelDuration = (diag.reelEndTime - diag.reelReadyTime) / 1000;
      console.error(`    - actual reel duration: ${actualReelDuration.toFixed(2)}s`);
    }
    throw new Error(`Capture watchdog timeout: page stuck in state '${diag.state}' after ${elapsed.toFixed(1)}s`);
  }
  
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
  
  // Calculate target output duration: trim at exactly reel end
  // Use reelEndTime if available (precise), otherwise estimate from duration
  const reelEndTime = await page.evaluate(() => window.__capture?.reelEndTime);
  
  // Log reel timing diagnostic: compare expected vs actual reel end
  if (reelReadyTime !== null && captureDuration > 0) {
    const expectedReelEndMs = reelReadyTime + (captureDuration * 1000);
    if (reelEndTime !== null && reelEndTime !== undefined) {
      const actualReelDurationMs = reelEndTime - reelReadyTime;
      const expectedReelDurationMs = captureDuration * 1000;
      const driftMs = actualReelDurationMs - expectedReelDurationMs;
      console.log(`  Reel timing: ready=${reelReadyTime.toFixed(0)}ms, end=${reelEndTime.toFixed(0)}ms`);
      console.log(`    Expected duration: ${expectedReelDurationMs.toFixed(0)}ms, actual: ${actualReelDurationMs.toFixed(0)}ms, drift: ${driftMs >= 0 ? '+' : ''}${driftMs.toFixed(0)}ms`);
    } else {
      console.log(`  Reel timing: ready=${reelReadyTime.toFixed(0)}ms, end=NOT_RECORDED (expected at ${expectedReelEndMs.toFixed(0)}ms)`);
    }
  }
  
  let targetOutputDuration = null;
  if (dynamicCutTimeMs !== null && reelReadyTime !== null) {
    if (reelEndTime !== null && reelEndTime !== undefined) {
      // Precise: use the actual reel-end timestamp from the page
      const reelDurationMs = reelEndTime - reelReadyTime;
      const cutOffsetFromReelStart = (dynamicCutTimeMs - reelReadyTime) / 1000;
      targetOutputDuration = reelDurationMs / 1000 - cutOffsetFromReelStart;
      console.log(`  Target output duration: ${targetOutputDuration.toFixed(2)}s (reelEnd=${reelDurationMs.toFixed(0)}ms - cut=${(cutOffsetFromReelStart * 1000).toFixed(0)}ms)`);
    } else if (captureDuration > 0) {
      // Fallback: estimate from captureDuration minus 0.5s tail
      const TAIL_DURATION_S = 0.5;
      const cutOffsetFromReelStart = (dynamicCutTimeMs - reelReadyTime) / 1000;
      targetOutputDuration = captureDuration - cutOffsetFromReelStart - TAIL_DURATION_S;
      console.log(`  Target output duration: ${targetOutputDuration.toFixed(2)}s (estimated: reel=${captureDuration}s - cut=${cutOffsetFromReelStart.toFixed(2)}s - tail=${TAIL_DURATION_S}s)`);
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
  const actualDuration = frameTimestamps.length > 0 
    ? frameTimestamps[frameTimestamps.length - 1].timestamp_s 
    : 0;
  console.log(`  Captured ${rawFrames.length} raw frames, kept ${frameCount} (dropped ${droppedEarlyFrames} early, ${droppedTailFrames} tail) over ${actualDuration.toFixed(2)}s`);
  
  // Frame-to-frame diff check in the last 1s: detect camera jump/reset
  // Use mean-abs-diff spike rule: fail if diff > 8 AND > 3× median of surrounding frames
  // On v22 this gives 13.2 at the jump vs ≤4 elsewhere
  if (frameTimestamps.length > 0 && targetOutputDuration !== null) {
    const last1sStart = Math.max(0, actualDuration - 1.0);
    const last1sFrames = frameTimestamps.filter(f => f.timestamp_s >= last1sStart);
    if (last1sFrames.length >= 2) {
      console.log(`  Checking frame-to-frame diff in last 1s (${last1sFrames.length} frames)...`);
      
      // Calculate true mean-abs-diff using signalstats YAVG of blend=difference
      const diffs = [];
      for (let i = 1; i < last1sFrames.length; i++) {
        const prevFrame = last1sFrames[i - 1].path;
        const currFrame = last1sFrames[i].path;
        try {
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
        } catch (e) {
          // Skip on error
        }
      }
      
      if (diffs.length > 0) {
        // Spike rule: fail if diff > 8 AND > 3× median of surrounding frames
        const sortedDiffs = [...diffs].sort((a, b) => a.diff - b.diff);
        const medianDiff = sortedDiffs[Math.floor(sortedDiffs.length / 2)]?.diff ?? 0;
        const maxDiff = Math.max(...diffs.map(d => d.diff));
        
        // Check each frame against the spike rule
        const spikes = diffs.filter(d => d.diff > 8 && d.diff > medianDiff * 3);
        if (spikes.length > 0) {
          const worstSpike = spikes.reduce((a, b) => a.diff > b.diff ? a : b);
          console.error(`  ERROR: Camera jump detected at ${worstSpike.time.toFixed(2)}s`);
          console.error(`    - Mean-abs-diff: ${worstSpike.diff.toFixed(1)} (median=${medianDiff.toFixed(1)}, threshold=max(8, ${(medianDiff * 3).toFixed(1)}))`);
          throw new Error(`Camera jump detected in last 1s at ${worstSpike.time.toFixed(2)}s (diff=${worstSpike.diff.toFixed(1)} > 8 and > 3×median)`);
        } else {
          console.log(`  Frame diff check OK (max=${maxDiff.toFixed(1)}, median=${medianDiff.toFixed(1)})`);
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
      const pageLoadEpochMs = audioInfo.reelReadyTime !== null && reelReadyCdpTime !== null
        ? (reelReadyCdpTime * 1000) - audioInfo.reelReadyTime
        : null;
      const audioStartMs = audioInfo.audioStartTime ?? 0;
      
      // Helper: detect white frames in a time range by CDP timestamp
      async function detectWhiteFrames(expectedTimesMs, label) {
        if (!expectedTimesMs || expectedTimesMs.length === 0 || pageLoadEpochMs === null) return [];
        const whiteFrames = [];
        const expectedCdpTimes = expectedTimesMs.map(t => (pageLoadEpochMs + t) / 1000);
        const minCdp = Math.min(...expectedCdpTimes) - 0.1;
        const maxCdp = Math.max(...expectedCdpTimes) + 0.1;
        const searchFrames = rawFrames.filter(f => f.cdpTimestamp >= minCdp && f.cdpTimestamp <= maxCdp);
        console.log(`    ${label}: searching ${searchFrames.length} frames in CDP [${minCdp.toFixed(3)}, ${maxCdp.toFixed(3)}]`);
        
        for (const frame of searchFrames) {
          try {
            const result = await new Promise((resolve) => {
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
            const yavgMatch = result.match(/lavfi\.signalstats\.YAVG=([\d.]+)/);
            if (yavgMatch && parseFloat(yavgMatch[1]) > 250) {
              whiteFrames.push({ cdpTimestamp: frame.cdpTimestamp, yavg: parseFloat(yavgMatch[1]) });
            }
          } catch { /* continue */ }
        }
        console.log(`    ${label}: found ${whiteFrames.length} white frames`);
        return whiteFrames;
      }
      
      // Helper: detect click onsets in audio
      async function detectClicks(expectedTimesMs, label) {
        if (!expectedTimesMs || expectedTimesMs.length === 0) return [];
        const clicks = [];
        const relativeTimesMs = expectedTimesMs.map(t => t - audioStartMs);
        const searchStartMs = Math.max(0, Math.min(...relativeTimesMs) - 100);
        const searchEndMs = Math.max(...relativeTimesMs) + 100;
        const searchStartS = searchStartMs / 1000;
        const searchDurationS = (searchEndMs - searchStartMs) / 1000;
        
        console.log(`    ${label}: searching audio ${searchStartS.toFixed(3)}s - ${(searchStartS + searchDurationS).toFixed(3)}s`);
        
        try {
          const astatsResult = await new Promise((resolve, reject) => {
            const ffmpeg = spawn("ffmpeg", [
              "-ss", String(searchStartS),
              "-t", String(searchDurationS),
              "-i", rawAudioPath,
              "-af", "asetnsamples=n=240,astats=metadata=1:reset=1,ametadata=print:key=lavfi.astats.Overall.Peak_level:file=-",
              "-f", "null", "-",
            ], { stdio: ["pipe", "pipe", "pipe"] });
            let stdout = "";
            ffmpeg.stdout.on("data", (d) => stdout += d);
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
            if (peakLevel > -10 && absoluteMs - lastOnsetMs > 80) {
              clicks.push({ timeMs: absoluteMs, peakDb: peakLevel });
              lastOnsetMs = absoluteMs;
            }
          }
        } catch { /* ignore */ }
        console.log(`    ${label}: found ${clicks.length} clicks`);
        return clicks;
      }
      
      // Detect START calibration burst (5 flash/click pairs)
      console.log("  Detecting START calibration burst...");
      const startCalibTimes = audioInfo.calibrationTimes ?? [];
      const startWhiteFrames = await detectWhiteFrames(startCalibTimes, "START video");
      const startClicks = await detectClicks(startCalibTimes, "START audio");
      
      // Match white frames to expected times by CDP timestamp, compute offsets
      const startPairOffsets = [];
      for (let i = 0; i < startCalibTimes.length; i++) {
        const expectedCdp = (pageLoadEpochMs + startCalibTimes[i]) / 1000;
        const videoMatch = startWhiteFrames.find(f => Math.abs(f.cdpTimestamp - expectedCdp) < 0.05);
        const expectedRelMs = startCalibTimes[i] - audioStartMs;
        const audioMatch = startClicks.find(c => Math.abs(c.timeMs - expectedRelMs) < 50);
        if (videoMatch && audioMatch) {
          const videoMs = (videoMatch.cdpTimestamp * 1000) - pageLoadEpochMs - audioStartMs;
          const offset = videoMs - audioMatch.timeMs;
          startPairOffsets.push(offset);
        }
      }
      
      if (startPairOffsets.length > 0) {
        const sorted = [...startPairOffsets].sort((a, b) => a - b);
        startMedianOffsetMs = sorted[Math.floor(sorted.length / 2)];
        console.log(`  START calibration: ${startPairOffsets.length} pairs, offsets=[${startPairOffsets.map(o => o.toFixed(1)).join(", ")}]ms, median=${startMedianOffsetMs.toFixed(1)}ms`);
      } else {
        console.error("  ERROR: No matched START calibration pairs");
        throw new Error("START calibration failed: no matched video/audio pairs");
      }
      
      // Detect END calibration burst for drift measurement
      console.log("  Detecting END calibration burst...");
      const endCalibTimes = audioInfo.endCalibrationTimes ?? [];
      
      if (endCalibTimes.length > 0) {
        const endWhiteFrames = await detectWhiteFrames(endCalibTimes, "END video");
        const endClicks = await detectClicks(endCalibTimes, "END audio");
        
        const endPairOffsets = [];
        for (let i = 0; i < endCalibTimes.length; i++) {
          const expectedCdp = (pageLoadEpochMs + endCalibTimes[i]) / 1000;
          const videoMatch = endWhiteFrames.find(f => Math.abs(f.cdpTimestamp - expectedCdp) < 0.05);
          const expectedRelMs = endCalibTimes[i] - audioStartMs;
          const audioMatch = endClicks.find(c => Math.abs(c.timeMs - expectedRelMs) < 50);
          if (videoMatch && audioMatch) {
            const videoMs = (videoMatch.cdpTimestamp * 1000) - pageLoadEpochMs - audioStartMs;
            endPairOffsets.push(videoMs - audioMatch.timeMs);
          }
        }
        
        if (endPairOffsets.length > 0) {
          const sorted = [...endPairOffsets].sort((a, b) => a - b);
          endMedianOffsetMs = sorted[Math.floor(sorted.length / 2)];
          console.log(`  END calibration: ${endPairOffsets.length} pairs, offsets=[${endPairOffsets.map(o => o.toFixed(1)).join(", ")}]ms, median=${endMedianOffsetMs.toFixed(1)}ms`);
          
          // Compute drift ratio
          const driftMs = endMedianOffsetMs - startMedianOffsetMs;
          const startMidMs = startCalibTimes[Math.floor(startCalibTimes.length / 2)];
          const endMidMs = endCalibTimes[Math.floor(endCalibTimes.length / 2)];
          const spanMs = endMidMs - startMidMs;
          
          if (spanMs > 1000) {
            clockRatio = 1 - (driftMs / spanMs);
            console.log(`  Drift: ${driftMs.toFixed(1)}ms over ${(spanMs / 1000).toFixed(1)}s, clockRatio=${clockRatio.toFixed(6)}`);
          } else {
            console.log("  WARNING: Span too short for drift, using ratio 1.0");
          }
        } else {
          console.log("  WARNING: No matched END calibration pairs");
        }
      } else {
        console.log("  WARNING: No END calibration times");
      }
      
      // Use start median offset for sync calculation
      calibVideoMs = startMedianOffsetMs;
      calibAudioMs = 0; // Offset is already video-audio
      
      // Calculate sync offset
      const videoCutTime = dynamicCutTimeMs !== null
        ? dynamicCutTimeMs
        : (audioInfo.reelReadyTime !== null ? audioInfo.reelReadyTime + PREROLL_CUT_S * 1000 : null);
      
      const cutOffsetMs = videoCutTime !== null ? (videoCutTime - audioStartMs) : PREROLL_CUT_S * 1000;
      audioOffsetMs = cutOffsetMs - startMedianOffsetMs;
      console.log(`  Final audio offset: ${audioOffsetMs.toFixed(1)}ms (cut=${cutOffsetMs.toFixed(1)}ms - calib=${startMedianOffsetMs.toFixed(1)}ms)`);
      
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
          const ffmpegTrim = spawn("ffmpeg", [
            "-y",
            "-ss", String(trimS),
            "-i", rawAudioPath,
            "-t", String(audioTargetDuration),
            "-af", filterChain,
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
        console.log(`  Audio: trimmed ${trimS.toFixed(3)}s, duration ${audioTargetDuration.toFixed(2)}s, fade at ${fadeStartTime.toFixed(2)}s${needsDriftCorrection ? `, atempo=${clockRatio.toFixed(6)}` : ''}`);
      } else {
        const delayMs = Math.abs(audioOffsetMs);
        const delayFilter = `adelay=${Math.round(delayMs)}:all=1`;
        const filterChain = atempoFilter
          ? `${delayFilter},${atempoFilter},${limiterFilter},${fadeFilter}`
          : `${delayFilter},${limiterFilter},${fadeFilter}`;
        await new Promise((resolve, reject) => {
          const ffmpegDelay = spawn("ffmpeg", [
            "-y",
            "-i", rawAudioPath,
            "-t", String(audioTargetDuration + delayMs / 1000),
            "-af", filterChain,
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
        console.log(`  Audio: delayed ${delayMs.toFixed(1)}ms, duration ${audioTargetDuration.toFixed(2)}s${needsDriftCorrection ? `, atempo=${clockRatio.toFixed(6)}` : ''}`);
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
    start_median_offset_ms: startMedianOffsetMs,
    end_median_offset_ms: endMedianOffsetMs,
    clock_ratio: clockRatio,
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
