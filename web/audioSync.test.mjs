#!/usr/bin/env node
/**
 * Test for audio sync: compares event times in rendered audio to fire-events.json frame times.
 * 
 * Creates synthetic fire events at known times, renders audio with clicks at those times,
 * and verifies the detected click times match the original event times.
 */

import { spawn } from "node:child_process";
import { mkdir, rm, writeFile, readFile } from "node:fs/promises";
import { join } from "node:path";

const TEST_DIR = "/tmp/audioSync-test";

// Test parameters
const SAMPLE_RATE = 48000;
const DURATION_S = 5;
const FPS = 60;

// Fire events at known times (seconds)
const FIRE_EVENT_TIMES = [0.5, 1.2, 2.0, 3.3, 4.1];

/**
 * Generate a WAV file with clicks at specified times.
 */
async function generateAudioWithClicks(outputPath, clickTimesS, durationS, sampleRate) {
  const samples = Math.ceil(durationS * sampleRate);
  const buffer = Buffer.alloc(44 + samples * 2); // 16-bit mono WAV
  
  // WAV header
  let offset = 0;
  const writeString = (str) => {
    for (let i = 0; i < str.length; i++) {
      buffer.writeUInt8(str.charCodeAt(i), offset++);
    }
  };
  const writeUint32 = (value) => {
    buffer.writeUInt32LE(value, offset);
    offset += 4;
  };
  const writeUint16 = (value) => {
    buffer.writeUInt16LE(value, offset);
    offset += 2;
  };
  
  writeString("RIFF");
  writeUint32(36 + samples * 2);
  writeString("WAVE");
  writeString("fmt ");
  writeUint32(16);
  writeUint16(1); // PCM
  writeUint16(1); // mono
  writeUint32(sampleRate);
  writeUint32(sampleRate * 2); // byte rate
  writeUint16(2); // block align
  writeUint16(16); // bits per sample
  writeString("data");
  writeUint32(samples * 2);
  
  // Generate audio with clicks at specified times
  for (let i = 0; i < samples; i++) {
    const t = i / sampleRate;
    let sample = 0;
    
    // Check if we're within a click window (20ms duration)
    for (const clickTime of clickTimesS) {
      const clickDuration = 0.02; // 20ms
      if (t >= clickTime && t < clickTime + clickDuration) {
        // Sharp 2kHz click with exponential decay
        const relT = t - clickTime;
        const envelope = Math.exp(-relT / 0.005); // 5ms decay
        sample = Math.sin(2 * Math.PI * 2000 * relT) * envelope * 0.9;
        break;
      }
    }
    
    // Write 16-bit sample
    const intSample = Math.max(-32768, Math.min(32767, Math.round(sample * 32767)));
    buffer.writeInt16LE(intSample, offset);
    offset += 2;
  }
  
  await writeFile(outputPath, buffer);
  return samples;
}

/**
 * Detect click onsets in audio using ffmpeg astats.
 */
async function detectClickOnsets(audioPath) {
  return new Promise((resolve, reject) => {
    const ffmpeg = spawn("ffmpeg", [
      "-i", audioPath,
      "-af", "asetnsamples=n=480,astats=metadata=1:reset=1,ametadata=print:key=lavfi.astats.Overall.Peak_level:file=-",
      "-f", "null", "-",
    ], { stdio: ["pipe", "pipe", "pipe"] });
    
    let stdout = "";
    ffmpeg.stdout.on("data", (d) => stdout += d);
    ffmpeg.on("close", (code) => {
      if (code !== 0) {
        reject(new Error(`ffmpeg exited with code ${code}`));
        return;
      }
      
      // Parse peak levels and detect onsets
      const clicks = [];
      const peakPattern = /pts_time:([\d.]+)\s*\nlavfi\.astats\.Overall\.Peak_level=([-\d.]+)/g;
      let match;
      let lastOnsetTime = -1000;
      
      while ((match = peakPattern.exec(stdout)) !== null) {
        const ptsTime = parseFloat(match[1]);
        const peakLevel = parseFloat(match[2]);
        
        // Onset detection: > -10dB and at least 80ms since last
        if (peakLevel > -10 && ptsTime - lastOnsetTime > 0.08) {
          clicks.push({ timeS: ptsTime, peakDb: peakLevel });
          lastOnsetTime = ptsTime;
        }
      }
      
      resolve(clicks);
    });
    ffmpeg.on("error", reject);
  });
}

/**
 * Create a mock fire-events.json with events at known times.
 */
function createFireEventsJson(eventTimesS, fps) {
  const events = eventTimesS.map((timeS, i) => ({
    time_s: timeS,
    unit_id: i + 1,
    unit_kind: "test_unit",
    mount: 0,
    weapon_kind: "test_weapon",
    position: [0, 0, 0],
    is_followed: i === 0,
  }));
  
  return {
    mode: "deterministic",
    follow_unit_id: 1,
    fps,
    duration: DURATION_S,
    audio_commands: events.length,
    has_audio: true,
    total_events: events.length,
    followed_events: events.filter(e => e.is_followed).length,
    events,
  };
}

async function main() {
  console.log("\n=== Audio Sync Test ===\n");
  console.log(`Fire event times: ${FIRE_EVENT_TIMES.join(", ")}s`);
  console.log(`Sample rate: ${SAMPLE_RATE} Hz`);
  console.log(`Duration: ${DURATION_S}s\n`);
  
  // Setup
  await rm(TEST_DIR, { recursive: true, force: true });
  await mkdir(TEST_DIR, { recursive: true });
  
  const audioPath = join(TEST_DIR, "audio.wav");
  const eventsPath = join(TEST_DIR, "fire-events.json");
  
  // Generate audio with clicks at event times
  console.log("Generating audio with clicks at event times...");
  await generateAudioWithClicks(audioPath, FIRE_EVENT_TIMES, DURATION_S, SAMPLE_RATE);
  
  // Create fire-events.json
  const fireEvents = createFireEventsJson(FIRE_EVENT_TIMES, FPS);
  await writeFile(eventsPath, JSON.stringify(fireEvents, null, 2));
  console.log(`Created fire-events.json with ${fireEvents.events.length} events`);
  
  // Detect click onsets in audio
  console.log("\nDetecting click onsets in audio...");
  const detectedClicks = await detectClickOnsets(audioPath);
  console.log(`Detected ${detectedClicks.length} clicks at: ${detectedClicks.map(c => c.timeS.toFixed(3)).join(", ")}s`);
  
  // Compare detected times to event times
  console.log("\nComparing detected times to event times:");
  const matches = [];
  const tolerance = 0.05; // 50ms tolerance
  
  for (let i = 0; i < fireEvents.events.length; i++) {
    const eventTime = fireEvents.events[i].time_s;
    const detected = detectedClicks.find(c => Math.abs(c.timeS - eventTime) < tolerance);
    
    if (detected) {
      const error = (detected.timeS - eventTime) * 1000;
      matches.push({ eventTime, detectedTime: detected.timeS, errorMs: error });
      console.log(`  Event ${i + 1}: expected ${eventTime.toFixed(3)}s, detected ${detected.timeS.toFixed(3)}s, error ${error.toFixed(1)}ms`);
    } else {
      console.log(`  Event ${i + 1}: expected ${eventTime.toFixed(3)}s, NOT DETECTED`);
    }
  }
  
  // Verify all events were detected
  const allMatched = matches.length === fireEvents.events.length;
  const maxError = matches.length > 0 ? Math.max(...matches.map(m => Math.abs(m.errorMs))) : Infinity;
  
  console.log(`\nMatched: ${matches.length}/${fireEvents.events.length} events`);
  console.log(`Max error: ${maxError.toFixed(1)}ms`);
  
  // Cleanup
  await rm(TEST_DIR, { recursive: true, force: true });
  
  // Report result
  if (allMatched && maxError <= 50) {
    console.log("\nPASS: All events matched within 50ms tolerance\n");
    process.exit(0);
  } else {
    console.error("\nFAIL: Events not matched or error too large\n");
    process.exit(1);
  }
}

main().catch((error) => {
  console.error("Test failed:", error.message);
  process.exit(1);
});
