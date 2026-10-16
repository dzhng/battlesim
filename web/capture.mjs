#!/usr/bin/env node
// Cinematic capture CLI: record the menu backdrop battle with a scripted camera.
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
//   --output=<path>    Output directory (default throwaway/capture/)
//
// Example:
//   bun run --cwd web capture --preset=ground-level --width=3840 --height=2160

import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
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
    output: OUTPUT_DIR.pathname,
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
    } else if (arg.startsWith("--output=")) {
      args.output = resolve(arg.slice("--output=".length));
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
  --output=<path>    Output directory (default throwaway/capture/)

Examples:
  bun run --cwd web capture
  bun run --cwd web capture --preset=ground-level
  bun run --cwd web capture --width=3840 --height=2160 --preset=orbit
`);
      process.exit(0);
    }
  }

  return args;
}

async function main() {
  const args = parseArgs(process.argv);
  console.log("Starting cinematic capture...");
  console.log(`  Preset: ${args.preset}`);
  console.log(`  Resolution: ${args.width}x${args.height}`);
  console.log(`  FPS: ${args.fps}`);
  console.log(`  Audio: ${args.audio}`);
  console.log(`  Cards: ${args.cards}`);

  await mkdir(args.output, { recursive: true });

  const server = await startServer(false);
  console.log(`Server running at ${server.url}`);

  const browser = await launchBrowser({ args: ["--mute-audio"] });

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

    const url = `${server.url}/labs/capture?${params}`;
    console.log(`Opening ${url}`);
    await page.goto(url);

    // Wait for lab to be ready
    console.log("Waiting for scene to prepare...");
    await page.waitForFunction(
      () => window.__lab?.ready || window.__lab?.error,
      undefined,
      { timeout: 60_000 },
    );

    const labError = await page.evaluate(() => window.__lab?.error);
    if (labError) {
      throw new Error(`Lab error: ${labError}`);
    }

    console.log("Recording started...");

    // Wait for capture to complete
    await page.waitForFunction(
      () => {
        const capture = window.__capture;
        if (capture?.state === "recording") {
          const pct = ((capture.elapsed / capture.duration) * 100).toFixed(0);
          console.log?.(`Recording: ${capture.elapsed.toFixed(1)}s / ${capture.duration.toFixed(1)}s (${pct}%)`);
        }
        return capture?.state === "complete" || capture?.state === "error";
      },
      undefined,
      { timeout: 300_000, polling: 1000 },
    );

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
    const outputPath = resolve(args.output, result.filename);
    await writeFile(outputPath, videoData);

    console.log(`\nCapture complete!`);
    console.log(`  Output: ${outputPath}`);
    console.log(`  Size: ${(videoData.length / 1024 / 1024).toFixed(2)} MB`);

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
