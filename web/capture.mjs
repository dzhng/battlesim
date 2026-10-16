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
  
  let frameCount = 0;
  let captureStartTime = null;
  let captureDuration = 0;
  const frameTimestamps = []; // { path, timestamp_s }
  let firstFrameTime = null;
  
  // Handle screencast frames with real timestamps
  cdp.on("Page.screencastFrame", async (event) => {
    if (!captureStartTime) captureStartTime = Date.now();
    
    try {
      const frameData = Buffer.from(event.data, "base64");
      const framePath = join(framesDir, `frame-${String(frameCount).padStart(6, "0")}.jpg`);
      await writeFile(framePath, frameData);
      
      // Use CDP timestamp (seconds since epoch) for accurate timing
      const frameTime = event.metadata.timestamp;
      if (firstFrameTime === null) firstFrameTime = frameTime;
      const relativeTime = frameTime - firstFrameTime;
      
      frameTimestamps.push({ path: framePath, timestamp_s: relativeTime });
      frameCount++;
      
      // Acknowledge frame to receive next one
      await cdp.send("Page.screencastFrameAck", { sessionId: event.sessionId });
      
      // Update progress
      const elapsed = (Date.now() - captureStartTime) / 1000;
      if (captureDuration > 0) {
        const pct = Math.min(100, (elapsed / captureDuration) * 100).toFixed(0);
        process.stdout.write(`\r  Recording: ${elapsed.toFixed(1)}s / ${captureDuration.toFixed(1)}s (${pct}%) [${frameCount} frames]    `);
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
  
  console.log("CDP screencast started, recording page...");
  
  // Tell the page to start (it will run the battle and capture audio only)
  await page.evaluate(() => {
    if (window.__capture?.startAudioOnly) {
      window.__capture.startAudioOnly();
    }
  });
  
  // Wait for page to signal completion or get duration
  const duration = await page.evaluate(() => window.__capture?.duration);
  if (duration) captureDuration = duration;
  
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
  
  const actualDuration = frameTimestamps.length > 0 
    ? frameTimestamps[frameTimestamps.length - 1].timestamp_s 
    : 0;
  console.log(`  Captured ${frameCount} frames over ${actualDuration.toFixed(2)}s`);
  
  // Build concat demuxer file with per-frame durations
  console.log("Building frame list with real timestamps...");
  let concatContent = "";
  for (let i = 0; i < frameTimestamps.length; i++) {
    const frame = frameTimestamps[i];
    const nextFrame = frameTimestamps[i + 1];
    // Duration is time to next frame, or 1/fps for the last frame
    const duration = nextFrame 
      ? (nextFrame.timestamp_s - frame.timestamp_s) 
      : (1 / args.fps);
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
  if (args.audio) {
    const audioBase64 = await page.evaluate(async () => {
      const capture = window.__capture;
      if (!capture?.audioBlob) return null;
      const buffer = await capture.audioBlob.arrayBuffer();
      const bytes = new Uint8Array(buffer);
      let binary = "";
      const chunk = 8192;
      for (let i = 0; i < bytes.length; i += chunk) {
        binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
      }
      return btoa(binary);
    });
    
    if (audioBase64) {
      const audioData = Buffer.from(audioBase64, "base64");
      await writeFile(audioPath, audioData);
      hasAudio = true;
      console.log("  Audio captured");
    }
  }
  
  // Mux audio and video - use audio duration as primary, pad video if needed
  console.log("Muxing audio and video...");
  
  // High profile, CRF 20, faststart for final output
  const muxArgs = hasAudio
    ? [
        "-y",
        "-i", rawVideoPath,
        "-i", audioPath,
        "-c:v", "copy",
        "-c:a", "aac",
        "-b:a", "192k",
        "-movflags", "+faststart",
        // Use shortest=0 (disabled) to keep full audio; video loops last frame if shorter
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
  if (hasAudio) await unlink(audioPath).catch(() => {});
  // Clean up frames directory
  for (const frame of frameTimestamps) {
    await unlink(frame.path).catch(() => {});
  }
  await import("node:fs/promises").then(fs => fs.rmdir(framesDir).catch(() => {}));
  
  // Get and write weapon fire events to sidecar JSON
  const fireEvents = await page.evaluate(() => window.__capture?.fireEvents ?? []);
  const followUnitId = await page.evaluate(() => window.__capture?.followUnitId ?? null);
  const sidecarPath = finalPath.replace(/\.(mp4|webm)$/, "-fire-events.json");
  const sidecar = {
    follow_unit_id: followUnitId,
    total_events: fireEvents.length,
    followed_events: fireEvents.filter(e => e.is_followed).length,
    events: fireEvents,
  };
  await writeFile(sidecarPath, JSON.stringify(sidecar, null, 2));
  console.log(`  Fire events: ${fireEvents.length} total, ${sidecar.followed_events} from followed unit (id: ${followUnitId ?? 'none'}) -> ${basename(sidecarPath)}`);
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
