// The synthesized baselines are Rust's (`presentation::sound::synth`): the
// bank takes their samples through the presentation module (`soundBank.ts`).
// This keeps their names, generated from Rust, and the browser mixer's own
// distance reverb.
import { mulberry32 } from "math/random";
import { hashString } from "@packages/renderer-core/src/math";

export { SOUNDS } from "./synthSounds.gen";

const TAU = Math.PI * 2;

/** An RBJ low-pass run in place over `x`, its cutoff `hz(i)` taken every 32 samples. */
function lowpass(x: Float32Array, sr: number, hz: (i: number) => number) {
  let x1 = 0,
    x2 = 0,
    y1 = 0,
    y2 = 0;
  let b0 = 0,
    b1 = 0,
    a1 = 0,
    a2 = 0;
  for (let i = 0; i < x.length; i++) {
    if ((i & 31) === 0) {
      const w = (TAU * Math.min(hz(i), sr * 0.45)) / sr;
      const cw = Math.cos(w);
      const alpha = Math.sin(w) / (2 * 0.707);
      const a0 = 1 + alpha;
      b0 = (1 - cw) / 2 / a0;
      b1 = (1 - cw) / a0;
      a1 = (-2 * cw) / a0;
      a2 = (1 - alpha) / a0;
    }
    const y = b0 * x[i] + b1 * x1 + b0 * x2 - a1 * y1 - a2 * y2;
    x2 = x1;
    x1 = x[i];
    y2 = y1;
    y1 = y;
    x[i] = y;
  }
}

/** The distance reverb's impulse response, `seconds` long: an outdoor tail
 *  (no early room), a short gap, then stereo-decorrelated noise decaying
 *  60 dB over its length and darkening as it goes (the far air). Unit
 *  energy per channel, so the send level alone sets how wet a voice is. */
export function reverbImpulse(seconds: number, sampleRate: number): Float32Array[] {
  const n = Math.max(1, Math.round(seconds * sampleRate));
  const gap = Math.round(0.02 * sampleRate);
  const tau = seconds / 6.9;
  return [0, 1].map((ch) => {
    const rng = mulberry32.create(hashString(`reverb${ch}`));
    const x = new Float32Array(n);
    for (let i = gap; i < n; i++)
      x[i] = (mulberry32.sample(rng) * 2 - 1) * Math.exp(-(i - gap) / sampleRate / tau);
    lowpass(x, sampleRate, (i) => 6000 * Math.exp((-2 * i) / n) + 800);
    let e = 0;
    for (let i = 0; i < n; i++) e += x[i] * x[i];
    const k = 1 / Math.sqrt(e || 1);
    for (let i = 0; i < n; i++) x[i] *= k;
    return x;
  });
}
