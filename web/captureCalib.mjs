#!/usr/bin/env node
/**
 * Calibration analysis module for A/V sync measurement.
 * 
 * Detects flash onsets in video frames and click onsets in audio,
 * matches them to trigger times, and computes the median A/V offset.
 * 
 * CLI usage:
 *   node web/captureCalib.mjs --frames <dir> --audio <file> --triggers <json> --audio-start <ms>
 * 
 * Module API:
 *   import { detectFlashOnsets, detectClickOnsets, matchCalibration } from './captureCalib.mjs';
 */

import { spawn } from "node:child_process";
import { readdir, readFile, writeFile, mkdir, rm } from "node:fs/promises";
import { join, resolve, basename } from "node:path";

/**
 * Analyze frame brightness using ffmpeg signalstats.
 * @param {string} framePath - Path to JPEG frame
 * @returns {Promise<number>} YAVG brightness (0-255)
 */
export async function getFrameBrightness(framePath) {
  return new Promise((resolve) => {
    const ffmpeg = spawn("ffmpeg", [
      "-i", framePath,
      "-vf", "signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-",
      "-f", "null", "-",
    ], { stdio: ["pipe", "pipe", "pipe"] });
    let stdout = "";
    ffmpeg.stdout.on("data", (d) => stdout += d);
    ffmpeg.on("close", () => {
      const match = stdout.match(/lavfi\.signalstats\.YAVG=([\d.]+)/);
      resolve(match ? parseFloat(match[1]) : 0);
    });
    ffmpeg.on("error", () => resolve(0));
  });
}

/**
 * Detect flash onsets from a directory of frames.
 * A flash onset is the first white frame (YAVG > 250) after a non-white frame (YAVG < 200).
 * 
 * @param {string} framesDir - Directory containing frame-NNNNNN.jpg files
 * @param {number[]} triggerTimesMs - Trigger times in ms (perf.now space)
 * @param {number} pageLoadEpochMs - Page load epoch for CDP timestamp conversion
 * @param {number} audioStartMs - Audio start time in ms (perf.now space)
 * @param {object[]} [framesJson] - Optional frames.json data with real CDP timestamps
 * @returns {Promise<{onsetMs: number, cdpTimestamp: number, yavg: number}[]>}
 */
export async function detectFlashOnsets(framesDir, triggerTimesMs, pageLoadEpochMs, audioStartMs, framesJson = null) {
  if (!triggerTimesMs || triggerTimesMs.length === 0) return [];
  
  // If we have frames.json with real timestamps, use those
  let frameList;
  if (framesJson?.frames?.length > 0) {
    frameList = framesJson.frames.map(f => ({
      path: f.path,
      cdpTimestamp: f.cdpTimestamp,
      frameIdx: f.idx,
    }));
  } else {
    // Fall back to reading directory and estimating timestamps
    const files = await readdir(framesDir);
    const frameFiles = files
      .filter(f => f.startsWith("frame-") && f.endsWith(".jpg"))
      .sort();
    
    if (frameFiles.length === 0) return [];
    
    // Base CDP timestamp: frames are captured starting from audio start time
    const baseCdpTimestamp = (pageLoadEpochMs + audioStartMs) / 1000;
    
    frameList = frameFiles.map((file, i) => {
      const idxMatch = file.match(/frame-(\d+)/);
      const frameIdx = idxMatch ? parseInt(idxMatch[1], 10) : i;
      return {
        path: join(framesDir, file),
        cdpTimestamp: baseCdpTimestamp + (frameIdx * 0.01667),
        frameIdx,
      };
    });
  }
  
  // Analyze brightness of all frames in order
  const frameBrightness = [];
  for (const frame of frameList) {
    const yavg = await getFrameBrightness(frame.path);
    frameBrightness.push({ ...frame, yavg });
  }
  
  // Detect onsets: first frame with YAVG > 250 after a frame with YAVG < 200
  const onsets = [];
  for (let i = 1; i < frameBrightness.length; i++) {
    const prev = frameBrightness[i - 1];
    const curr = frameBrightness[i];
    if (prev.yavg < 200 && curr.yavg > 250) {
      const onsetMs = (curr.cdpTimestamp * 1000) - pageLoadEpochMs - audioStartMs;
      onsets.push({ onsetMs, cdpTimestamp: curr.cdpTimestamp, yavg: curr.yavg, frameIdx: curr.frameIdx });
    }
  }
  
  return onsets;
}

/**
 * Detect click onsets in an audio file.
 * A click onset is the first sample window with peak > -10dB, with 80ms minimum gap.
 * 
 * @param {string} audioPath - Path to audio file (webm, wav, etc.)
 * @param {number[]} triggerTimesMs - Trigger times in ms (perf.now space)
 * @param {number} audioStartMs - Audio start time in ms (perf.now space)
 * @returns {Promise<{timeMs: number, peakDb: number}[]>}
 */
export async function detectClickOnsets(audioPath, triggerTimesMs, audioStartMs) {
  if (!triggerTimesMs || triggerTimesMs.length === 0) return [];
  
  // Search range: 50ms before first trigger to 150ms after last trigger (relative to audio start)
  const relativeTimesMs = triggerTimesMs.map(t => t - audioStartMs);
  const searchStartMs = Math.max(0, Math.min(...relativeTimesMs) - 50);
  const searchEndMs = Math.max(...relativeTimesMs) + 150;
  const searchStartS = searchStartMs / 1000;
  const searchDurationS = (searchEndMs - searchStartMs) / 1000;
  
  const clicks = [];
  
  try {
    const astatsResult = await new Promise((resolve, reject) => {
      const ffmpeg = spawn("ffmpeg", [
        "-ss", String(searchStartS),
        "-t", String(searchDurationS),
        "-i", audioPath,
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
      // Onset detection: > -10dB and at least 80ms since last
      if (peakLevel > -10 && absoluteMs - lastOnsetMs > 80) {
        clicks.push({ timeMs: absoluteMs, peakDb: peakLevel });
        lastOnsetMs = absoluteMs;
      }
    }
  } catch { /* ignore */ }
  
  return clicks;
}

/**
 * Match triggers to detected onsets.
 * For video: find first onset in [trigger, trigger+120ms]
 * For audio: find first onset in [trigger-20ms, trigger+120ms]
 * 
 * @param {number[]} triggerTimesMs - Trigger times (perf.now space)
 * @param {{onsetMs?: number, timeMs?: number}[]} onsets - Detected onsets
 * @param {number} audioStartMs - Audio start time
 * @param {boolean} isVideo - True for video (stricter window)
 * @returns {{triggerIdx: number, triggerMs: number, onsetMs: number}[]}
 */
export function matchTriggersToOnsets(triggerTimesMs, onsets, audioStartMs, isVideo) {
  const matched = [];
  const usedOnsets = new Set();
  
  for (let i = 0; i < triggerTimesMs.length; i++) {
    const triggerRelMs = triggerTimesMs[i] - audioStartMs;
    const windowStart = isVideo ? triggerRelMs : triggerRelMs - 20;
    const windowEnd = triggerRelMs + 120;
    
    // Find first unused onset in window
    const onset = onsets.find(o => {
      const t = isVideo ? o.onsetMs : o.timeMs;
      return t >= windowStart && t <= windowEnd && !usedOnsets.has(o);
    });
    
    if (onset) {
      usedOnsets.add(onset);
      const onsetMs = isVideo ? onset.onsetMs : onset.timeMs;
      matched.push({ triggerIdx: i, triggerMs: triggerRelMs, onsetMs });
    }
  }
  
  return matched;
}

/**
 * Run full calibration analysis: detect onsets, match, compute median offset.
 * 
 * @param {string} framesDir - Directory with frame JPEGs
 * @param {string} audioPath - Path to audio file
 * @param {number[]} triggerTimesMs - Trigger times (perf.now space)
 * @param {number} audioStartMs - Audio start time (perf.now space)
 * @param {number} pageLoadEpochMs - Page load epoch for CDP conversion
 * @param {object} [framesJson] - Optional frames.json data with real CDP timestamps
 * @returns {Promise<{medianOffsetMs: number, pairs: {idx: number, videoMs: number, audioMs: number, offset: number}[], flashOnsets: number[], clickOnsets: number[]}>}
 */
export async function matchCalibration(framesDir, audioPath, triggerTimesMs, audioStartMs, pageLoadEpochMs, framesJson = null) {
  // Detect onsets (use real timestamps from framesJson if available)
  const flashOnsets = await detectFlashOnsets(framesDir, triggerTimesMs, pageLoadEpochMs, audioStartMs, framesJson);
  const clickOnsets = await detectClickOnsets(audioPath, triggerTimesMs, audioStartMs);
  
  // Match triggers to onsets
  const videoMatches = matchTriggersToOnsets(triggerTimesMs, flashOnsets, audioStartMs, true);
  const audioMatches = matchTriggersToOnsets(triggerTimesMs, clickOnsets, audioStartMs, false);
  
  // Pair up video and audio matches by trigger index
  const pairs = [];
  for (const vm of videoMatches) {
    const am = audioMatches.find(a => a.triggerIdx === vm.triggerIdx);
    if (am) {
      const offset = vm.onsetMs - am.onsetMs;
      pairs.push({ idx: vm.triggerIdx, videoMs: vm.onsetMs, audioMs: am.onsetMs, offset });
    }
  }
  
  // Compute median offset
  let medianOffsetMs = null;
  if (pairs.length >= 3) {
    const sorted = pairs.map(p => p.offset).sort((a, b) => a - b);
    medianOffsetMs = sorted[Math.floor(sorted.length / 2)];
  }
  
  return {
    medianOffsetMs,
    pairs,
    flashOnsets: flashOnsets.map(o => o.onsetMs),
    clickOnsets: clickOnsets.map(c => c.timeMs),
  };
}

// CLI entry point
async function main() {
  const args = process.argv.slice(2);
  
  let framesDir = null;
  let audioPath = null;
  let triggersJson = null;
  let audioStartMs = 0;
  let pageLoadEpochMs = 0;
  let framesJsonPath = null;
  
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--frames" && args[i + 1]) framesDir = resolve(args[++i]);
    else if (args[i] === "--audio" && args[i + 1]) audioPath = resolve(args[++i]);
    else if (args[i] === "--triggers" && args[i + 1]) triggersJson = args[++i];
    else if (args[i] === "--audio-start" && args[i + 1]) audioStartMs = parseFloat(args[++i]);
    else if (args[i] === "--page-load-epoch" && args[i + 1]) pageLoadEpochMs = parseFloat(args[++i]);
    else if (args[i] === "--frames-json" && args[i + 1]) framesJsonPath = resolve(args[++i]);
    else if (args[i] === "--help" || args[i] === "-h") {
      console.log(`Calibration analysis for A/V sync measurement.

Usage:
  node web/captureCalib.mjs --frames <dir> --audio <file> --triggers <json> --audio-start <ms>

Options:
  --frames <dir>           Directory containing frame-NNNNNN.jpg files
  --audio <file>           Audio file (webm, wav, etc.)
  --triggers <json>        JSON array of trigger times in ms, e.g. "[100,250,400]"
  --audio-start <ms>       Audio start time in ms (default 0)
  --page-load-epoch <ms>   Page load epoch for CDP conversion (default 0)
  --frames-json <file>     frames.json with real CDP timestamps (overrides --audio-start, --page-load-epoch)

Output:
  JSON with medianOffsetMs, pairs, flashOnsets, clickOnsets
`);
      process.exit(0);
    }
  }
  
  if (!framesDir || !audioPath || !triggersJson) {
    console.error("Missing required arguments. Use --help for usage.");
    process.exit(1);
  }
  
  const triggerTimesMs = JSON.parse(triggersJson);
  
  // Load frames.json if provided (contains real CDP timestamps)
  let framesJson = null;
  if (framesJsonPath) {
    try {
      const content = await readFile(framesJsonPath, "utf8");
      framesJson = JSON.parse(content);
      // Use values from frames.json if not explicitly provided
      if (framesJson.audioStartMs !== null && audioStartMs === 0) {
        audioStartMs = framesJson.audioStartMs;
      }
      if (framesJson.pageLoadEpochMs !== null && pageLoadEpochMs === 0) {
        pageLoadEpochMs = framesJson.pageLoadEpochMs;
      }
      console.error(`Loaded ${framesJson.frames?.length ?? 0} frames from ${framesJsonPath}`);
    } catch (err) {
      console.error(`Warning: Could not load frames.json: ${err.message}`);
    }
  }
  
  const result = await matchCalibration(framesDir, audioPath, triggerTimesMs, audioStartMs, pageLoadEpochMs, framesJson);
  
  console.log(JSON.stringify(result, null, 2));
}

// Run CLI if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(err => {
    console.error(err.message);
    process.exit(1);
  });
}
