#!/usr/bin/env node
/**
 * Unit test for captureCalib.mjs using synthetic frames and audio.
 * 
 * Creates:
 * - Tiny gray/white JPEG frames simulating 5 flashes at known times
 * - Synthetic audio file with clicks at known times + a known offset
 * 
 * Asserts the recovered offset is within 2ms of the expected value.
 */

import { spawn } from "node:child_process";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { matchCalibration } from "./captureCalib.mjs";

const TEST_DIR = "/tmp/captureCalib-test";
const FRAMES_DIR = join(TEST_DIR, "frames");
const AUDIO_PATH = join(TEST_DIR, "clicks.wav");

// Test parameters
const AUDIO_START_MS = 100;
const PAGE_LOAD_EPOCH_MS = 10000;
// Trigger times in perf.now space (relative to page load)
const TRIGGER_TIMES_MS = [200, 350, 500, 650, 800];
// Known offset: video appears ~50ms after trigger (quantized to frame boundary),
// audio appears 5ms after trigger
const FRAME_INTERVAL_MS = 16.67;
// Trigger relative to audio start = 200 - 100 = 100ms
// Video appears at frame round((100+50)/16.67) = round(9) = frame 9 = 150ms
// Audio click at 5ms after trigger = 100 + 5 = 105ms relative to audio start  
// Actual video latency = 150 - 100 = 50ms, actual audio latency = 5ms
// Expected offset = 150 - 105 = 45ms
const VIDEO_LATENCY_MS = 50;
const AUDIO_LATENCY_MS = 5;
const EXPECTED_OFFSET_MS = 45; // video onset - audio onset

/**
 * Generate a tiny JPEG with specified brightness.
 * Uses ffmpeg to create a 16x16 gray or white image.
 */
async function generateFrame(outputPath, isWhite) {
  const color = isWhite ? "white" : "gray";
  return new Promise((resolve, reject) => {
    const ffmpeg = spawn("ffmpeg", [
      "-y",
      "-f", "lavfi",
      "-i", `color=c=${color}:s=16x16:d=0.04`,
      "-frames:v", "1",
      "-q:v", "2",
      outputPath,
    ], { stdio: "pipe" });
    ffmpeg.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`ffmpeg frame gen failed: ${code}`));
    });
    ffmpeg.on("error", reject);
  });
}

/**
 * Generate synthetic audio with clicks at specified times.
 * Creates a 1-second WAV file with 1ms impulses at each click time.
 */
async function generateAudio(outputPath, clickTimesMs, durationMs = 1000) {
  // Build filter: silence + impulses at each click time
  // Use aevalsrc to generate impulses
  const sampleRate = 48000;
  const durationS = durationMs / 1000;
  
  // Create a silent base and add impulses
  // Each click is a short loud burst
  let filterExpr = `aevalsrc=0:s=${sampleRate}:d=${durationS}`;
  
  // Add clicks as short sine bursts
  for (const clickMs of clickTimesMs) {
    const clickS = clickMs / 1000;
    // Use sine burst for each click: loud for 1ms
    filterExpr = `${filterExpr}[base];aevalsrc='if(between(t,${clickS},${clickS + 0.001}),0.9,0)':s=${sampleRate}:d=${durationS}[click${clickMs}];[base][click${clickMs}]amix=inputs=2:duration=longest`;
  }
  
  return new Promise((resolve, reject) => {
    const ffmpeg = spawn("ffmpeg", [
      "-y",
      "-f", "lavfi",
      "-i", filterExpr,
      "-c:a", "pcm_s16le",
      outputPath,
    ], { stdio: "pipe" });
    let stderr = "";
    ffmpeg.stderr.on("data", (d) => stderr += d);
    ffmpeg.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`ffmpeg audio gen failed: ${code}\n${stderr}`));
    });
    ffmpeg.on("error", reject);
  });
}

/**
 * Alternative: generate audio with clicks using a simpler approach.
 * Creates a WAV file with short noise bursts at specified times.
 */
async function generateAudioSimple(outputPath, clickTimesMs, durationMs = 1000) {
  const sampleRate = 48000;
  const durationS = durationMs / 1000;
  
  // Build adelay filter to place clicks
  // Start with a single short click, then adelay copies to each position
  const clickDurationMs = 2;
  
  // Create base click
  const baseFilter = `anoisesrc=d=${clickDurationMs / 1000}:a=0.9:c=pink`;
  
  // If we have multiple clicks, we need to concat them at the right times
  // Simpler: create silence, then overlay clicks
  // Actually simplest: create silence with aevalsrc, then use amerge with delayed clicks
  
  // Let's try a different approach: use anullsrc for silence, then add noise bursts
  const silenceFilter = `anullsrc=r=${sampleRate}:cl=mono,atrim=duration=${durationS}`;
  
  // For each click, create a noise burst and position it
  let inputs = `-f lavfi -i "${silenceFilter}"`;
  let filterComplex = "[0]";
  
  for (let i = 0; i < clickTimesMs.length; i++) {
    const clickMs = clickTimesMs[i];
    // Create a noise burst input
    inputs += ` -f lavfi -i "anoisesrc=d=0.002:a=0.9:c=pink"`;
    // Delay it and mix
    filterComplex += `[${i + 1}]adelay=${clickMs}|${clickMs}[d${i}];[d${i}]`;
    if (i < clickTimesMs.length - 1) {
      filterComplex += `amix=inputs=2:duration=first`;
    }
  }
  filterComplex += `amix=inputs=2:duration=first[out]`;
  
  // This is getting complex, let's use a simpler shell-based approach
  // Generate clicks one at a time and concatenate
  
  // Actually, simplest: use sine waves
  return new Promise((resolve, reject) => {
    // Generate a sine burst at each click time
    // sine=f=1000:d=0.002 gives a 1kHz tone for 2ms
    const clickExprs = clickTimesMs.map((ms, i) => {
      const startS = ms / 1000;
      return `if(between(t,${startS},${startS + 0.002}),sin(2*PI*1000*t)*0.9,0)`;
    }).join("+");
    
    const ffmpeg = spawn("ffmpeg", [
      "-y",
      "-f", "lavfi",
      "-i", `aevalsrc='${clickExprs}':s=${sampleRate}:d=${durationS}`,
      "-c:a", "pcm_s16le",
      outputPath,
    ], { stdio: "pipe" });
    let stderr = "";
    ffmpeg.stderr.on("data", (d) => stderr += d);
    ffmpeg.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`ffmpeg audio gen failed: ${code}\n${stderr}`));
    });
    ffmpeg.on("error", reject);
  });
}

async function setup() {
  // Clean up and create test directories
  await rm(TEST_DIR, { recursive: true, force: true });
  await mkdir(FRAMES_DIR, { recursive: true });
  
  // Generate frames: mostly gray, with white frames at flash times
  // Frame rate ~60fps means ~16.67ms per frame
  const frameIntervalMs = 16.67;
  const totalDurationMs = 1000;
  const numFrames = Math.ceil(totalDurationMs / frameIntervalMs);
  
  // Calculate which frames should be white (flash onsets)
  // Video appears VIDEO_LATENCY_MS after trigger
  const flashTimesMs = TRIGGER_TIMES_MS.map(t => t - AUDIO_START_MS + VIDEO_LATENCY_MS);
  const whiteFrameIndices = new Set();
  for (const flashMs of flashTimesMs) {
    // Use round to avoid floating-point issues (150/16.67 = 8.999... should be 9)
    const frameIdx = Math.round(flashMs / frameIntervalMs);
    whiteFrameIndices.add(frameIdx);
    whiteFrameIndices.add(frameIdx + 1); // Flash lasts 2 frames
  }
  
  console.log(`Generating ${numFrames} frames...`);
  console.log(`White frame indices: ${[...whiteFrameIndices].sort((a, b) => a - b).join(", ")}`);
  
  for (let i = 0; i < numFrames; i++) {
    const framePath = join(FRAMES_DIR, `frame-${String(i).padStart(6, "0")}.jpg`);
    const isWhite = whiteFrameIndices.has(i);
    await generateFrame(framePath, isWhite);
  }
  
  // Generate audio with clicks at AUDIO_LATENCY_MS after each trigger
  const clickTimesMs = TRIGGER_TIMES_MS.map(t => t - AUDIO_START_MS + AUDIO_LATENCY_MS);
  console.log(`Generating audio with clicks at: ${clickTimesMs.join(", ")}ms`);
  await generateAudioSimple(AUDIO_PATH, clickTimesMs);
}

async function runTest() {
  console.log("\n=== Calibration Test ===\n");
  console.log(`Trigger times: ${TRIGGER_TIMES_MS.join(", ")}ms`);
  console.log(`Audio start: ${AUDIO_START_MS}ms`);
  console.log(`Expected video latency: ${VIDEO_LATENCY_MS}ms`);
  console.log(`Expected audio latency: ${AUDIO_LATENCY_MS}ms`);
  console.log(`Expected offset (video - audio): ${EXPECTED_OFFSET_MS}ms`);
  console.log();
  
  await setup();
  
  console.log("\nRunning calibration analysis...\n");
  
  const result = await matchCalibration(
    FRAMES_DIR,
    AUDIO_PATH,
    TRIGGER_TIMES_MS,
    AUDIO_START_MS,
    PAGE_LOAD_EPOCH_MS
  );
  
  console.log("Flash onsets:", result.flashOnsets.map(o => o.toFixed(1)).join(", "), "ms");
  console.log("Click onsets:", result.clickOnsets.map(o => o.toFixed(1)).join(", "), "ms");
  console.log("Matched pairs:", result.pairs.length);
  for (const p of result.pairs) {
    console.log(`  [${p.idx}] video=${p.videoMs.toFixed(1)}ms, audio=${p.audioMs.toFixed(1)}ms, offset=${p.offset.toFixed(1)}ms`);
  }
  console.log(`Median offset: ${result.medianOffsetMs?.toFixed(1) ?? "null"}ms`);
  
  // Assert
  if (result.pairs.length < 3) {
    console.error(`\nFAIL: Only ${result.pairs.length} pairs matched (need ≥3)`);
    process.exit(1);
  }
  
  if (result.medianOffsetMs === null) {
    console.error("\nFAIL: No median offset computed");
    process.exit(1);
  }
  
  const error = Math.abs(result.medianOffsetMs - EXPECTED_OFFSET_MS);
  if (error > 2) {
    console.error(`\nFAIL: Offset error ${error.toFixed(1)}ms exceeds 2ms tolerance`);
    console.error(`  Expected: ${EXPECTED_OFFSET_MS}ms, Got: ${result.medianOffsetMs.toFixed(1)}ms`);
    process.exit(1);
  }
  
  console.log(`\nPASS: Offset error ${error.toFixed(1)}ms within 2ms tolerance`);
  
  // Cleanup
  await rm(TEST_DIR, { recursive: true, force: true });
}

runTest().catch(err => {
  console.error("Test error:", err);
  process.exit(1);
});
