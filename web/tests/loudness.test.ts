// @vitest-environment node
import { expect, test } from "vitest";
import { maxMomentaryLoudness } from "@packages/battle-audio/src/loudness";

test("K-weighted metering matches the BS.1770 1 kHz reference across browser sample rates", () => {
  for (const rate of [44100, 48000, 96000]) {
    const tone = Float32Array.from(
      { length: rate },
      (_, frame) => 0.1 * Math.sin((2 * Math.PI * 1000 * frame) / rate),
    );
    // BS.1770: a full-scale mono 1 kHz sine reads -3.01; 0.1 amplitude adds -20 dB.
    expect(maxMomentaryLoudness([tone], rate)).toBeCloseTo(-23.01, 1);
  }
});

test("a short report keeps its reference when surrounded by silence or followed by a long quiet tail", () => {
  const rate = 48000;
  const report = Float32Array.from(
    { length: rate / 20 },
    (_, frame) => 0.1 * Math.sin((2 * Math.PI * 1000 * frame) / rate),
  );
  const padded = new Float32Array(rate * 2);
  padded.set(report, rate / 4);
  expect(maxMomentaryLoudness([padded], rate)).toBeCloseTo(maxMomentaryLoudness([report], rate), 6);
  expect(maxMomentaryLoudness([new Float32Array(rate)], rate)).toBe(-Infinity);
});
