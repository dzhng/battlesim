// @vitest-environment node
import { beforeAll, expect, test } from "vitest";
import { audioMix as AUDIO } from "@web/audioMix";
import { vehicleLoops } from "@packages/battle-audio/src/soundFrame";
import { synthesize } from "@packages/battle-audio/src/soundBank";
import { loadPresentation } from "@web/battle/sim/presentation";
beforeAll(() => loadPresentation());

test("a reversing vehicle beeps at a steady rate and level, whatever its speed", () => {
  const row = AUDIO.vehicles.tracked_heavy;
  const beeper = (load: number) =>
    vehicleLoops(row, load, 0, true).find((l) => l.slot === "reverse");
  // An electronic back-up alarm: creeping or backing fast, the same beep.
  expect(beeper(0.1)).toEqual(beeper(1));
  expect(beeper(1)!.rate).toBe(1);
  // Standing still, or driving forward, it is silent.
  expect(beeper(0)).toBeUndefined();
  expect(vehicleLoops(row, 1, 0, false).some((l) => l.slot === "reverse")).toBe(false);
});

test("the reverse sound beeps: tone, then silence, each period", () => {
  const sampleRate = 48000;
  const sound = synthesize(AUDIO.vehicles.default.reverse!, sampleRate);
  const x = sound.channels[0];
  expect(sound.loop).toBe(true);
  // 10 ms levels: a beep's level is near the peak, the gap between beeps silent.
  const n = sampleRate / 100;
  const levels = Array.from({ length: Math.floor(x.length / n) }, (_, i) =>
    Math.sqrt(x.subarray(i * n, (i + 1) * n).reduce((sum, v) => sum + v * v, 0) / n),
  );
  const top = Math.max(...levels);
  const on = levels.filter((l) => l > top / 2).length / levels.length;
  const off = levels.filter((l) => l < top / 100).length / levels.length;
  expect(on).toBeGreaterThan(0.3);
  expect(off).toBeGreaterThan(0.3);
});
