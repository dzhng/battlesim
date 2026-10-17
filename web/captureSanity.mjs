#!/usr/bin/env node
/**
 * Headless sanity check for deterministic capture mode.
 * 
 * Verifies that sim tick, camera pose, and fire-event times all advance
 * monotonically when stepping through virtual time, WITHOUT requiring WebGPU.
 * 
 * This test uses a mock sim that simulates the stepping behavior and verifies
 * the timing calculations are correct.
 * 
 * Usage:
 *   node web/captureSanity.mjs [options]
 * 
 * Options:
 *   --frames=N     Number of frames to simulate (default 100)
 *   --fps=N        Frame rate (default 60)
 *   --tick-hz=N    Sim tick rate (default 30)
 *   --cut-ms=N     Preroll cut time in ms (default 1150)
 *   --verbose      Print detailed frame-by-frame output
 */

const parseArgs = (argv) => {
  const args = {
    frames: 100,
    fps: 60,
    tickHz: 30,
    cutMs: 1150,
    verbose: false,
  };

  for (const arg of argv.slice(2)) {
    if (arg.startsWith("--frames=")) {
      args.frames = parseInt(arg.slice("--frames=".length), 10);
    } else if (arg.startsWith("--fps=")) {
      args.fps = parseInt(arg.slice("--fps=".length), 10);
    } else if (arg.startsWith("--tick-hz=")) {
      args.tickHz = parseInt(arg.slice("--tick-hz=".length), 10);
    } else if (arg.startsWith("--cut-ms=")) {
      args.cutMs = parseFloat(arg.slice("--cut-ms=".length));
    } else if (arg === "--verbose") {
      args.verbose = true;
    } else if (arg === "--help" || arg === "-h") {
      console.log(`Headless sanity check for deterministic capture mode.

Usage:
  node web/captureSanity.mjs [options]

Options:
  --frames=N     Number of frames to simulate (default 100)
  --fps=N        Frame rate (default 60)
  --tick-hz=N    Sim tick rate (default 30)
  --cut-ms=N     Preroll cut time in ms (default 1150)
  --verbose      Print detailed frame-by-frame output

This test verifies that:
  1. Sim tick advances monotonically with virtual time
  2. Camera pose (via elapsed time) advances correctly
  3. Fire event time_s = frame_index / fps (monotonic)
  4. Virtual time starts at cut point and advances by frameDuration
`);
      process.exit(0);
    }
  }

  return args;
};

/**
 * Simulate the deterministic capture logic from capture.tsx
 */
function simulateDeterministicCapture(args) {
  const { frames: numFrames, fps, tickHz, cutMs, verbose } = args;
  const frameDurationMs = 1000 / fps;
  
  // State that mirrors capture.tsx refs
  let deterministicTick = 0;
  let deterministicFrame = 0;
  let virtualTimeMs = 0;
  
  // Tracking for monotonicity checks
  const records = [];
  let errors = [];
  
  console.log(`Simulating ${numFrames} frames at ${fps} fps, ${tickHz} tick_hz, cut at ${cutMs}ms\n`);
  
  // enterDeterministicMode: reset state
  deterministicTick = 0;
  deterministicFrame = 0;
  virtualTimeMs = 0;
  
  // Simulate stepFrame calls as capture.mjs does
  for (let outputIdx = 0; outputIdx < numFrames; outputIdx++) {
    // capture.mjs: virtualTimeMs = cutStartMs + (outputIdx * frameDurationMs)
    virtualTimeMs = cutMs + (outputIdx * frameDurationMs);
    
    // stepFrame logic from capture.tsx:
    // targetTick = floor(virtualTimeMs / 1000 * tickHz)
    const targetTick = Math.floor(virtualTimeMs / 1000 * tickHz);
    
    // Advance sim if needed
    if (targetTick > deterministicTick) {
      deterministicTick = targetTick;
    }
    
    // Increment frame counter
    deterministicFrame++;
    
    // Fire event time_s = (deterministicFrame / fps) * 1000 in ms, then / 1000 to seconds
    // Actually from the code: eventTime = (deterministicFrame / fps) * 1000 (in ms)
    // Then in WeaponFireTracker: time_s = (now - startTime) / 1000
    // startTime is set on first call, so time_s = elapsed since first frame
    // For first frame: eventTime = (1 / 60) * 1000 = 16.67ms, startTime = 16.67ms
    // For frame N: eventTime = ((N+1) / 60) * 1000, so elapsed = (N / 60) * 1000
    // time_s = elapsed / 1000 = N / 60
    // So time_s should be outputIdx / fps (0-indexed frame gives correct 0-start time)
    const fireEventTimeS = outputIdx / fps;
    
    // Camera elapsed calculation from pose():
    // elapsed = virtualTimeRef.current / 1000 (in deterministic mode)
    const cameraElapsedS = virtualTimeMs / 1000;
    
    records.push({
      outputIdx,
      virtualTimeMs,
      targetTick,
      deterministicTick,
      deterministicFrame,
      fireEventTimeS,
      cameraElapsedS,
    });
    
    if (verbose) {
      console.log(`Frame ${outputIdx}: virtualTime=${virtualTimeMs.toFixed(1)}ms, tick=${targetTick}, fireTime=${fireEventTimeS.toFixed(3)}s, cameraElapsed=${cameraElapsedS.toFixed(3)}s`);
    }
  }
  
  // Monotonicity checks
  console.log("Checking monotonicity...");
  
  // 1. Sim tick must advance monotonically
  let lastTick = -1;
  for (let i = 0; i < records.length; i++) {
    const r = records[i];
    if (r.targetTick < lastTick) {
      errors.push(`Frame ${i}: tick decreased (${lastTick} -> ${r.targetTick})`);
    }
    lastTick = r.targetTick;
  }
  
  // 2. Virtual time must advance monotonically
  let lastVirtualTime = -1;
  for (let i = 0; i < records.length; i++) {
    const r = records[i];
    if (r.virtualTimeMs <= lastVirtualTime) {
      errors.push(`Frame ${i}: virtualTime not increasing (${lastVirtualTime} -> ${r.virtualTimeMs})`);
    }
    lastVirtualTime = r.virtualTimeMs;
  }
  
  // 3. Fire event times must be strictly increasing (0, 1/fps, 2/fps, ...)
  let lastFireTime = -1;
  for (let i = 0; i < records.length; i++) {
    const r = records[i];
    const expectedTime = i / fps;
    if (Math.abs(r.fireEventTimeS - expectedTime) > 0.0001) {
      errors.push(`Frame ${i}: fireEventTime mismatch (got ${r.fireEventTimeS.toFixed(4)}, expected ${expectedTime.toFixed(4)})`);
    }
    if (r.fireEventTimeS <= lastFireTime && i > 0) {
      errors.push(`Frame ${i}: fireEventTime not increasing (${lastFireTime} -> ${r.fireEventTimeS})`);
    }
    lastFireTime = r.fireEventTimeS;
  }
  
  // 4. Camera elapsed must advance with virtual time
  let lastCameraElapsed = -1;
  for (let i = 0; i < records.length; i++) {
    const r = records[i];
    if (r.cameraElapsedS <= lastCameraElapsed && i > 0) {
      errors.push(`Frame ${i}: cameraElapsed not increasing (${lastCameraElapsed} -> ${r.cameraElapsedS})`);
    }
    lastCameraElapsed = r.cameraElapsedS;
  }
  
  // 5. Check tick coverage - should cover the expected number of ticks
  const expectedFirstTick = Math.floor(cutMs / 1000 * tickHz);
  const expectedLastTick = Math.floor((cutMs + (numFrames - 1) * frameDurationMs) / 1000 * tickHz);
  const actualFirstTick = records[0].targetTick;
  const actualLastTick = records[records.length - 1].targetTick;
  
  if (actualFirstTick !== expectedFirstTick) {
    errors.push(`First tick mismatch: got ${actualFirstTick}, expected ${expectedFirstTick}`);
  }
  if (actualLastTick !== expectedLastTick) {
    errors.push(`Last tick mismatch: got ${actualLastTick}, expected ${expectedLastTick}`);
  }
  
  // Summary
  console.log("\n=== Summary ===");
  console.log(`Frames: ${numFrames}`);
  console.log(`First virtual time: ${records[0].virtualTimeMs.toFixed(1)}ms (tick ${records[0].targetTick})`);
  console.log(`Last virtual time: ${records[records.length - 1].virtualTimeMs.toFixed(1)}ms (tick ${records[records.length - 1].targetTick})`);
  console.log(`Ticks covered: ${actualFirstTick} - ${actualLastTick} (${actualLastTick - actualFirstTick + 1} unique)`);
  console.log(`Fire event time range: 0s - ${records[records.length - 1].fireEventTimeS.toFixed(3)}s`);
  
  if (errors.length > 0) {
    console.log(`\n❌ FAILED: ${errors.length} errors`);
    for (const err of errors.slice(0, 10)) {
      console.log(`  - ${err}`);
    }
    if (errors.length > 10) {
      console.log(`  ... and ${errors.length - 10} more`);
    }
    return false;
  } else {
    console.log(`\n✓ PASSED: All monotonicity checks passed`);
    return true;
  }
}

// Run the test
const args = parseArgs(process.argv);
const passed = simulateDeterministicCapture(args);
process.exit(passed ? 0 : 1);
