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
  --webm             Output WebM instead of MP4 (skip ffmpeg transcoding)
  --output=<path>    Output directory (default throwaway/capture/)
  --scene=<name>     Scene to capture: menu (default) or skirmish

Examples:
  bun run --cwd web capture
  bun run --cwd web capture --preset=ground-level
  bun run --cwd web capture --width=3840 --height=2160 --preset=orbit
  bun run --cwd web capture --scene=skirmish
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

/** Check prerequisites and print what's missing. */
async function checkPrerequisites() {
  const issues = [];

  if (!await checkMechanicsBuilt()) {
    issues.push("Mechanics validator not built. Run: bun run build:mechanics");
  }

  const lfsPointers = await checkLfsAssets();
  if (lfsPointers.length > 0) {
    issues.push(`LFS assets are pointers (not downloaded). Run: git lfs pull`);
  }

  if (issues.length > 0) {
    console.error("Missing prerequisites:");
    for (const issue of issues) {
      console.error(`  - ${issue}`);
    }
    console.error("");
    return false;
  }
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
 *  Uses Page.startScreencast for video frames piped to ffmpeg, and
 *  browser-side audio capture for audio, then muxes them together. */
async function captureWithCDP(page, args, server) {
  const cdp = await page.context().newCDPSession(page);
  
  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  const rawVideoPath = resolve(args.output, `raw-${timestamp}.mp4`);
  const audioPath = resolve(args.output, `audio-${timestamp}.webm`);
  const finalPath = resolve(args.output, `capture-${args.preset}-${timestamp}.${args.webm ? "webm" : "mp4"}`);
  
  // Start ffmpeg to receive frames
  const ffmpegVideo = spawn("ffmpeg", [
    "-y",
    "-f", "image2pipe",
    "-framerate", String(args.fps),
    "-i", "-",
    "-c:v", "libx264",
    "-preset", "ultrafast",
    "-crf", "18",
    "-pix_fmt", "yuv420p",
    rawVideoPath,
  ], { stdio: ["pipe", "pipe", "pipe"] });
  
  let frameCount = 0;
  let captureStartTime = null;
  let captureDuration = 0;
  
  // Handle screencast frames
  cdp.on("Page.screencastFrame", async (event) => {
    if (!captureStartTime) captureStartTime = Date.now();
    
    try {
      const frameData = Buffer.from(event.data, "base64");
      ffmpegVideo.stdin.write(frameData);
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
    quality: 90,
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
  console.log(`  Captured ${frameCount} frames`);
  
  // Stop screencast
  await cdp.send("Page.stopScreencast");
  
  // Close ffmpeg video input
  ffmpegVideo.stdin.end();
  await new Promise((resolve, reject) => {
    ffmpegVideo.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`ffmpeg video exited with code ${code}`));
    });
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
  
  // Mux audio and video
  console.log("Muxing audio and video...");
  
  const muxArgs = hasAudio
    ? ["-y", "-i", rawVideoPath, "-i", audioPath, "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest", finalPath]
    : ["-y", "-i", rawVideoPath, "-c:v", "copy", "-an", finalPath];
  
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
  if (hasAudio) await unlink(audioPath).catch(() => {});
  
  const { size } = await stat(finalPath);
  return { path: finalPath, size };
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
      "capture-mode": "page",
    });
    if (args.scene === "skirmish") {
      params.set("capture-scene", "skirmish");
    }

    const route = args.scene === "skirmish" ? "/labs/capture-skirmish" : "/labs/capture";
    const url = `${server.url}${route}?${params}`;
    console.log(`Opening ${url}`);
    await page.goto(url);

    // Wait for lab to be ready
    console.log("Waiting for scene to prepare...");
    await page.waitForFunction(
      () => window.__lab?.ready || window.__lab?.error,
      undefined,
      { timeout: 120_000 },
    );

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
