#!/usr/bin/env node
// Cinematic capture CLI: record the menu backdrop battle with a scripted camera.
//
// Usage:
//   pnpm --filter battle-web capture [options]
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
//   pnpm --filter battle-web capture --preset=ground-level --width=3840 --height=2160

import { mkdir, writeFile, unlink, access } from "node:fs/promises";
import { resolve, basename } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { startServer } from "./scene.mjs";
import { launchBrowser } from "./browser.mjs";

const HERE = new URL(".", import.meta.url);
const OUTPUT_DIR = new URL("../throwaway/capture/", HERE);

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
  pnpm --filter battle-web capture [options]

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
  pnpm --filter battle-web capture
  pnpm --filter battle-web capture --preset=ground-level
  pnpm --filter battle-web capture --width=3840 --height=2160 --preset=orbit
  pnpm --filter battle-web capture --scene=skirmish
`);
      process.exit(0);
    }
  }

  return args;
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

/** Transcode WebM to MP4 using ffmpeg. Returns the output path. */
async function transcodeToMp4(webmPath, fps) {
  const mp4Path = webmPath.replace(/\.webm$/, ".mp4");
  
  console.log("Transcoding to MP4 (H.264 + AAC)...");
  
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
    ffmpeg.stderr.on("data", (data) => {
      stderr += data.toString();
      const match = stderr.match(/time=(\d+:\d+:\d+\.\d+)/g);
      if (match) {
        process.stdout.write(`\r  Progress: ${match[match.length - 1]}    `);
      }
    });
    
    ffmpeg.on("close", (code) => {
      process.stdout.write("\n");
      if (code === 0) {
        resolve(mp4Path);
      } else {
        reject(new Error(`ffmpeg exited with code ${code}`));
      }
    });
    
    ffmpeg.on("error", reject);
  });
}

async function main() {
  const args = parseArgs(process.argv);
  console.log("Starting cinematic capture...");
  console.log(`  Scene: ${args.scene}`);
  console.log(`  Preset: ${args.preset}`);
  console.log(`  Resolution: ${args.width}x${args.height}`);
  console.log(`  FPS: ${args.fps}`);
  console.log(`  Audio: ${args.audio}`);
  console.log(`  Cards: ${args.cards}`);
  console.log(`  Output format: ${args.webm ? "WebM" : "MP4"}`);

  if (!args.webm && !hasFfmpeg()) {
    console.warn("Warning: ffmpeg not found, output will be WebM");
    args.webm = true;
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

    console.log("Recording started...");

    // Poll for progress
    const progressInterval = setInterval(async () => {
      try {
        const status = await page.evaluate(() => window.__capture);
        if (status?.state === "recording") {
          const pct = ((status.elapsed / status.duration) * 100).toFixed(0);
          process.stdout.write(
            `\r  Recording: ${status.elapsed.toFixed(1)}s / ${status.duration.toFixed(1)}s (${pct}%)    `,
          );
        }
      } catch {
        // Page may be navigating
      }
    }, 500);

    // Wait for capture to complete
    await page.waitForFunction(
      () => {
        const capture = window.__capture;
        return capture?.state === "complete" || capture?.state === "error";
      },
      undefined,
      { timeout: 300_000 },
    );
    clearInterval(progressInterval);
    process.stdout.write("\n");

    const result = await page.evaluate(() => window.__capture);

    if (result?.state === "error") {
      throw new Error(`Capture error: ${result.error}`);
    }

    if (result?.state !== "complete") {
      throw new Error("Capture did not complete");
    }

    console.log("Encoding complete, saving video...");

    // Download the blob
    const base64 = await page.evaluate(async () => {
      const capture = window.__capture;
      if (!capture?.blob) return null;
      const buffer = await capture.blob.arrayBuffer();
      const bytes = new Uint8Array(buffer);
      let binary = "";
      const chunk = 8192;
      for (let i = 0; i < bytes.length; i += chunk) {
        binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
      }
      return btoa(binary);
    });

    if (!base64) {
      throw new Error("Could not extract video blob");
    }

    const videoData = Buffer.from(base64, "base64");
    const webmPath = resolve(args.output, result.filename);
    await writeFile(webmPath, videoData);

    let finalPath = webmPath;
    let finalSize = videoData.length;

    // Transcode to MP4 unless --webm flag is set
    if (!args.webm) {
      try {
        const mp4Path = await transcodeToMp4(webmPath, args.fps);
        // Remove the intermediate WebM file
        await unlink(webmPath).catch(() => {});
        finalPath = mp4Path;
        const { size } = await import("node:fs").then((fs) =>
          fs.promises.stat(mp4Path),
        );
        finalSize = size;
      } catch (err) {
        console.warn(`Transcoding failed: ${err.message}`);
        console.warn("Keeping WebM output");
      }
    }

    console.log(`\nCapture complete!`);
    console.log(`  Output: ${finalPath}`);
    console.log(`  Size: ${(finalSize / 1024 / 1024).toFixed(2)} MB`);

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
