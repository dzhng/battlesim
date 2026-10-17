// `<name>.report.json`: what each scene recording delivered, and warnings.
//
// A slow machine records choppier, never slower: the recording is wall-clock
// real time. The report says when a scene fell under 90% of the target rate.
// Missing audio is recorded here; the runner makes it fatal.

/** Average rate and worst gap from a recording's frame timestamps (seconds,
 * ascending), e.g. ffprobe's packet pts of the WebM before CFR resampling. */
export function frameStats(times) {
  if (times.length < 2) return { avgFps: 0, worstGapMs: 0 };
  let worst = 0;
  for (let i = 1; i < times.length; i++) worst = Math.max(worst, times[i] - times[i - 1]);
  return {
    avgFps: (times.length - 1) / (times.at(-1) - times[0]),
    worstGapMs: worst * 1000,
  };
}

/** `outputs` are `{scene, file, seconds, avgFps, worstGapMs, audio}`. */
export function buildReport({ capture, fps, outputs, joined = null }) {
  const warnings = [];
  for (const output of outputs) {
    // Compared in tenths so 0.9 × target has no float error at the boundary.
    if (output.avgFps * 10 < fps * 9) {
      warnings.push(
        `scene ${output.scene}: avg ${output.avgFps.toFixed(1)} fps is below 90% of ${fps}`,
      );
    }
    if (!output.audio) warnings.push(`scene ${output.scene}: no audio track`);
  }
  return { capture, outputs, joined, warnings };
}
