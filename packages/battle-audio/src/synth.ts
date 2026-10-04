// The synthesized baseline: every sound is a deterministic function of its
// name and sample rate (seeded noise, filters, envelopes), so the offline
// render is reproducible. Transients are shaped noise and sines;
// loops are built to wrap seamlessly (whole cycles, or a crossfaded seam).
// Each sound peaks at `PEAK`; the mix's levels are `presentation.audio`'s.
import { mulberry32 } from "math/random";
import { hashString } from "@packages/renderer-core/src/math";

/** Every sound's peak before the mix. */
const PEAK = 0.9;

export interface SynthSound {
  /** One array per channel (the ambience bed is stereo, the rest mono). */
  channels: Float32Array[];
  loop: boolean;
}

type Rng = ReturnType<typeof mulberry32.create>;
const TAU = Math.PI * 2;

const white = (rng: Rng) => mulberry32.sample(rng) * 2 - 1;

/** An RBJ biquad run in place over `x`. */
function biquad(
  x: Float32Array,
  sr: number,
  type: "lowpass" | "highpass" | "bandpass",
  hz: number | ((i: number) => number),
  q = 0.707,
) {
  let x1 = 0,
    x2 = 0,
    y1 = 0,
    y2 = 0;
  let b0 = 0,
    b1 = 0,
    b2 = 0,
    a1 = 0,
    a2 = 0;
  let last = -1;
  for (let i = 0; i < x.length; i++) {
    const f = typeof hz === "number" ? hz : hz(i);
    if (f !== last && (typeof hz === "number" ? i === 0 : (i & 31) === 0)) {
      last = f;
      const w = (TAU * Math.min(f, sr * 0.45)) / sr;
      const cw = Math.cos(w);
      const alpha = Math.sin(w) / (2 * q);
      const a0 = 1 + alpha;
      if (type === "lowpass") {
        b0 = (1 - cw) / 2 / a0;
        b1 = (1 - cw) / a0;
        b2 = b0;
      } else if (type === "highpass") {
        b0 = (1 + cw) / 2 / a0;
        b1 = -(1 + cw) / a0;
        b2 = b0;
      } else {
        b0 = alpha / a0;
        b1 = 0;
        b2 = -alpha / a0;
      }
      a1 = (-2 * cw) / a0;
      a2 = (1 - alpha) / a0;
    }
    const y = b0 * x[i] + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
    x2 = x1;
    x1 = x[i];
    y2 = y1;
    y1 = y;
    x[i] = y;
  }
  return x;
}

function noise(n: number, rng: Rng): Float32Array {
  const x = new Float32Array(n);
  for (let i = 0; i < n; i++) x[i] = white(rng);
  return x;
}

/** `into += gain × src × env(t)`, from sample `at`. */
function mix(
  into: Float32Array,
  src: Float32Array,
  sr: number,
  gain: number,
  env: (t: number) => number,
  at = 0,
) {
  for (let i = 0; i < src.length && i + at < into.length; i++)
    into[i + at] += gain * src[i] * env(i / sr);
}

const decay =
  (tau: number, attack = 0.0005) =>
  (t: number) =>
    Math.min(1, t / attack) * Math.exp(-t / tau);

/** A sine whose frequency follows `hz(t)`, under `env`. */
function tone(n: number, sr: number, hz: (t: number) => number, env: (t: number) => number) {
  const x = new Float32Array(n);
  let phase = 0;
  for (let i = 0; i < n; i++) {
    const t = i / sr;
    phase += (TAU * hz(t)) / sr;
    x[i] = Math.sin(phase) * env(t);
  }
  return x;
}

function toPeak(x: Float32Array, peak = PEAK): Float32Array {
  let m = 0;
  for (let i = 0; i < x.length; i++) m = Math.max(m, Math.abs(x[i]));
  if (m > 0) for (let i = 0; i < x.length; i++) x[i] *= peak / m;
  return x;
}

/** A loop of `n` samples from `make(n + fade)`: the tail is crossfaded over
 *  the head, so it wraps without a click. */
export function seamless(n: number, fade: number, make: (n: number) => Float32Array): Float32Array {
  const long = make(n + fade);
  const out = long.slice(0, n);
  for (let i = 0; i < fade; i++) {
    const a = i / fade;
    out[i] = out[i] * a + long[n + i] * (1 - a);
  }
  return out;
}

/** A report: a crack, a bandpassed body, a low thump and a room tail. */
function report(
  sr: number,
  rng: Rng,
  o: {
    len: number;
    crack: number;
    body: number;
    bodyTau: number;
    thumpHz: number;
    thump: number;
    thumpTau: number;
    tail: number;
    tailTau: number;
    tailHz: number;
  },
) {
  const n = Math.round(o.len * sr);
  const out = new Float32Array(n);
  mix(out, biquad(noise(n, rng), sr, "highpass", 2500), sr, o.crack, decay(0.004));
  mix(out, biquad(noise(n, rng), sr, "bandpass", o.body, 0.8), sr, 2.5, decay(o.bodyTau));
  mix(
    out,
    tone(n, sr, (t) => o.thumpHz * (1 + 0.8 * Math.exp(-t / 0.01)), decay(o.thumpTau, 0.001)),
    sr,
    o.thump,
    () => 1,
  );
  mix(
    out,
    biquad(noise(n, rng), sr, "lowpass", o.tailHz),
    sr,
    o.tail,
    (t) => Math.min(1, t / 0.02) * Math.exp(-t / o.tailTau),
  );
  return toPeak(out);
}

/** A far report: low-passed, a soft attack and a rolling echo. */
function farReport(sr: number, rng: Rng, len: number, hz: number, tau: number, echo: number) {
  const n = Math.round(len * sr);
  const out = new Float32Array(n);
  const boom = biquad(noise(n, rng), sr, "lowpass", hz);
  mix(out, boom, sr, 1, (t) => Math.min(1, t / 0.008) * Math.exp(-t / tau));
  mix(
    out,
    boom,
    sr,
    echo,
    (t) => Math.min(1, t / 0.05) * Math.exp(-t / (tau * 2.5)),
    Math.round(0.35 * sr),
  );
  return toPeak(biquad(out, sr, "lowpass", hz * 1.5));
}

function explosion(sr: number, rng: Rng, len: number, far: boolean) {
  const n = Math.round(len * sr);
  const out = new Float32Array(n);
  mix(
    out,
    tone(n, sr, (t) => 55 * Math.exp(-t / 0.4) + 28, decay(0.35, 0.002)),
    sr,
    1.4,
    () => 1,
  );
  mix(
    out,
    biquad(noise(n, rng), sr, "lowpass", far ? 350 : 2200),
    sr,
    far ? 1.6 : 1.2,
    decay(far ? 0.45 : 0.25, 0.002),
  );
  mix(
    out,
    biquad(noise(n, rng), sr, "lowpass", 160),
    sr,
    2.2,
    (t) => Math.min(1, t / 0.05) * Math.exp(-t / 0.9),
  );
  if (!far) {
    mix(out, biquad(noise(n, rng), sr, "highpass", 3000), sr, 0.8, decay(0.005));
    // Debris pattering down.
    for (let k = 0; k < 40; k++) {
      const at = 0.25 + mulberry32.sample(rng) * (len - 0.6);
      const m = Math.round(0.03 * sr);
      const click = biquad(noise(m, rng), sr, "bandpass", 1200 + 2000 * mulberry32.sample(rng), 2);
      mix(out, click, sr, 0.15 * Math.exp(-at / 1.2), decay(0.008), Math.round(at * sr));
    }
  }
  return toPeak(out);
}

/** An engine at `hz` firings a second: a jittered pulse train through its
 *  exhaust (a low-pass), with mechanical noise. Whole cycles, so it loops. */
function engine(sr: number, rng: Rng, hz: number, len: number, bright: number) {
  const cycles = Math.round(hz * len);
  const n = Math.round((cycles / hz) * sr);
  const out = new Float32Array(n);
  const period = n / cycles;
  for (let c = 0; c < cycles; c++) {
    const amp = 0.75 + 0.25 * mulberry32.sample(rng);
    const at = Math.round(c * period);
    for (let i = 0; i < period * 0.9 && at + i < n; i++)
      out[at + i] += amp * Math.exp(-i / (period * 0.18)) * Math.sin((TAU * i) / (period * 0.45));
  }
  biquad(out, sr, "lowpass", 180 + 500 * bright, 1.2);
  const mech = seamless(n, Math.round(0.05 * sr), (m) =>
    biquad(noise(m, rng), sr, "bandpass", 900 + 1500 * bright, 1.5),
  );
  for (let i = 0; i < n; i++) out[i] += 0.12 * mech[i] * (0.8 + 0.2 * Math.sin((TAU * i * 3) / n));
  return toPeak(out);
}

type Maker = ((sr: number, rng: Rng) => SynthSound) & { loop: boolean };
const once = (f: (sr: number, rng: Rng) => Float32Array): Maker =>
  Object.assign(
    (sr: number, rng: Rng) => ({
      channels: [f(sr, rng)],
      loop: false,
    }),
    { loop: false },
  );
const looped = (f: (sr: number, rng: Rng) => Float32Array): Maker =>
  Object.assign(
    (sr: number, rng: Rng) => ({
      channels: [f(sr, rng)],
      loop: true,
    }),
    { loop: true },
  );

/** Every sound the bank makes, by name. */
export const SOUNDS: Record<string, Maker> = {
  rifle: once((sr, rng) =>
    report(sr, rng, {
      len: 0.7,
      crack: 1.2,
      body: 1500,
      bodyTau: 0.025,
      thumpHz: 120,
      thump: 0.6,
      thumpTau: 0.03,
      tail: 0.35,
      tailTau: 0.18,
      tailHz: 1800,
    }),
  ),
  rifle_far: once((sr, rng) => farReport(sr, rng, 1.2, 900, 0.09, 0.35)),
  hmg: once((sr, rng) =>
    report(sr, rng, {
      len: 0.7,
      crack: 1.0,
      body: 800,
      bodyTau: 0.035,
      thumpHz: 85,
      thump: 1.0,
      thumpTau: 0.045,
      tail: 0.45,
      tailTau: 0.22,
      tailHz: 1200,
    }),
  ),
  hmg_far: once((sr, rng) => farReport(sr, rng, 1.3, 600, 0.11, 0.4)),
  cannon: once((sr, rng) => explosion(sr, rng, 2.4, false)),
  cannon_far: once((sr, rng) => explosion(sr, rng, 2.8, true)),
  thump: once((sr, rng) => {
    const n = Math.round(0.35 * sr);
    const out = tone(n, sr, (t) => 150 + 200 * Math.exp(-t / 0.01), decay(0.05, 0.001));
    mix(out, biquad(noise(n, rng), sr, "bandpass", 700, 1), sr, 1.2, decay(0.03));
    return toPeak(out);
  }),
  launch: once((sr, rng) => {
    const n = Math.round(1.2 * sr);
    const out = new Float32Array(n);
    mix(
      out,
      biquad(noise(n, rng), sr, "bandpass", (i) => 2400 - 1600 * Math.min(1, i / sr / 0.8), 1.2),
      sr,
      2,
      (t) => Math.min(1, t / 0.02) * Math.exp(-t / 0.4),
    );
    mix(
      out,
      tone(n, sr, () => 90, decay(0.06, 0.001)),
      sr,
      0.8,
      () => 1,
    );
    return toPeak(out);
  }),
  impact_ground: once((sr, rng) => {
    const n = Math.round(0.45 * sr);
    const out = tone(n, sr, (t) => 70 + 60 * Math.exp(-t / 0.02), decay(0.06, 0.001));
    mix(out, biquad(noise(n, rng), sr, "lowpass", 900), sr, 1.5, decay(0.08));
    mix(out, biquad(noise(n, rng), sr, "bandpass", 2500, 1), sr, 0.4, (t) =>
      t < 0.05 ? 0 : Math.exp(-(t - 0.05) / 0.08) * (0.5 + 0.5 * Math.sin(t * 190)),
    );
    return toPeak(out);
  }),
  impact_hull: once((sr, rng) => {
    const n = Math.round(0.7 * sr);
    const out = new Float32Array(n);
    for (const [hz, a, tau] of [
      [523, 1, 0.25],
      [1187, 0.7, 0.18],
      [1873, 0.5, 0.12],
      [2711, 0.4, 0.08],
      [3907, 0.25, 0.05],
    ])
      mix(
        out,
        tone(n, sr, () => hz, decay(tau, 0.0005)),
        sr,
        a,
        () => 1,
      );
    mix(out, biquad(noise(n, rng), sr, "highpass", 2000), sr, 1.2, decay(0.006));
    return toPeak(out);
  }),
  impact_prop: once((sr, rng) => {
    const n = Math.round(0.35 * sr);
    const out = tone(n, sr, () => 220, decay(0.04, 0.0005));
    mix(out, biquad(noise(n, rng), sr, "bandpass", 1600, 1.2), sr, 2, decay(0.035));
    mix(out, biquad(noise(n, rng), sr, "lowpass", 3000), sr, 0.3, (t) =>
      t < 0.04 ? 0 : Math.exp(-(t - 0.04) / 0.1),
    );
    return toPeak(out);
  }),
  impact_soldier: once((sr, rng) => {
    const n = Math.round(0.18 * sr);
    const out = biquad(noise(n, rng), sr, "lowpass", 450);
    for (let i = 0; i < n; i++) out[i] *= decay(0.035, 0.002)(i / sr);
    return toPeak(out);
  }),
  ricochet: once((sr, rng) => {
    const n = Math.round(0.65 * sr);
    const start = 2600 + 1200 * mulberry32.sample(rng);
    const out = tone(
      n,
      sr,
      (t) => start * Math.exp(-t / 0.5) * (1 + 0.03 * Math.sin(TAU * 38 * t)),
      (t) => Math.min(1, t / 0.004) * Math.exp(-t / 0.22),
    );
    mix(out, biquad(noise(n, rng), sr, "highpass", 3000), sr, 0.6, decay(0.004));
    return toPeak(out);
  }),
  explosion: once((sr, rng) => explosion(sr, rng, 3.2, false)),
  explosion_far: once((sr, rng) => explosion(sr, rng, 3.4, true)),
  footstep: once((sr, rng) => {
    const n = Math.round(0.14 * sr);
    const out = biquad(noise(n, rng), sr, "bandpass", 1400, 0.9);
    for (let i = 0; i < n; i++) out[i] *= decay(0.02, 0.002)(i / sr);
    mix(
      out,
      tone(n, sr, () => 95, decay(0.025, 0.002)),
      sr,
      0.5,
      () => 1,
    );
    return toPeak(out);
  }),
  motor: looped((sr, rng) => {
    const n = Math.round(1.5 * sr);
    return toPeak(
      seamless(n, Math.round(0.1 * sr), (m) => {
        const x = biquad(noise(m, rng), sr, "bandpass", 1100, 0.6);
        const low = biquad(noise(m, rng), sr, "lowpass", 220);
        for (let i = 0; i < m; i++) x[i] = x[i] * (0.85 + 0.15 * white(rng)) + 1.5 * low[i];
        return x;
      }),
    );
  }),
  engine_diesel: looped((sr, rng) => engine(sr, rng, 28, 2, 0.2)),
  engine_small: looped((sr, rng) => engine(sr, rng, 42, 2, 0.6)),
  tracks: looped((sr, rng) => {
    // Track links striking the sprockets, 10 a second at rate 1, and a squeal.
    const n = Math.round(2 * sr);
    const out = new Float32Array(n);
    for (let k = 0; k < 20; k++) {
      const at = Math.round((k / 20) * n + (mulberry32.sample(rng) - 0.5) * 0.01 * sr);
      const m = Math.round(0.06 * sr);
      const clank = biquad(noise(m, rng), sr, "bandpass", 700 + 900 * mulberry32.sample(rng), 3);
      for (let i = 0; i < m; i++) out[(at + i + n) % n] += clank[i] * Math.exp(-i / sr / 0.012);
    }
    const rumble = seamless(n, Math.round(0.05 * sr), (m) =>
      biquad(noise(m, rng), sr, "lowpass", 140),
    );
    for (let i = 0; i < n; i++) out[i] += 0.8 * rumble[i];
    return toPeak(out);
  }),
  wheels: looped((sr, rng) => {
    const n = Math.round(2 * sr);
    return toPeak(
      seamless(n, Math.round(0.1 * sr), (m) => {
        const x = biquad(noise(m, rng), sr, "lowpass", 260);
        const grit = biquad(noise(m, rng), sr, "bandpass", 2500, 1);
        for (let i = 0; i < m; i++)
          x[i] = 2 * x[i] + 0.08 * grit[i] * (mulberry32.sample(rng) < 0.02 ? 4 : 1);
        return x;
      }),
    );
  }),
  turret: looped((sr, rng) => {
    // A servo's whine (whole cycles in the loop) over gear noise.
    const n = Math.round(1 * sr);
    const out = new Float32Array(n);
    for (const [hz, a] of [
      [180, 1],
      [360, 0.5],
      [540, 0.25],
    ])
      for (let i = 0; i < n; i++)
        out[i] += a * Math.sin((TAU * hz * i) / sr) * (0.8 + 0.2 * Math.sin((TAU * 6 * i) / sr));
    const gear = seamless(n, Math.round(0.05 * sr), (m) =>
      biquad(noise(m, rng), sr, "bandpass", 1200, 2),
    );
    for (let i = 0; i < n; i++) out[i] += 0.5 * gear[i];
    return toPeak(out);
  }),
  reverse_whine: looped((sr) => {
    const n = Math.round(1 * sr);
    const out = new Float32Array(n);
    for (let i = 0; i < n; i++) {
      const t = i / sr;
      out[i] =
        (Math.sin(TAU * 720 * t) + 0.4 * Math.sin(TAU * 1440 * t)) *
        (0.7 + 0.3 * Math.sin(TAU * 4 * t));
    }
    return toPeak(out);
  }),
  fire: looped((sr, rng) => {
    // Crackle over a low roar (the smoke's rumble), wrapping at the seam.
    const n = Math.round(4 * sr);
    const out = seamless(n, Math.round(0.2 * sr), (m) => {
      const x = biquad(noise(m, rng), sr, "lowpass", 180);
      const hiss = biquad(noise(m, rng), sr, "bandpass", 3000, 0.7);
      for (let i = 0; i < m; i++)
        x[i] = 3 * x[i] * (0.8 + 0.2 * Math.sin((TAU * 0.5 * i) / sr)) + 0.1 * hiss[i];
      return x;
    });
    for (let k = 0; k < 90; k++) {
      const at = Math.floor(mulberry32.sample(rng) * n);
      const m = Math.round(0.012 * sr);
      const pop = biquad(noise(m, rng), sr, "bandpass", 1500 + 3500 * mulberry32.sample(rng), 2);
      const a = 0.3 + 1.4 * mulberry32.sample(rng) ** 3;
      for (let i = 0; i < m; i++) out[(at + i) % n] += a * pop[i] * Math.exp(-i / sr / 0.003);
    }
    return toPeak(out);
  }),
  countryside: Object.assign(
    (sr: number, rng: Rng) => ({ channels: countryside(sr, rng), loop: true }),
    { loop: true },
  ),
  cue_gunfire: once((sr, rng) => {
    const n = Math.round(1.3 * sr);
    const out = new Float32Array(n);
    const pop = farReport(sr, rng, 0.5, 700, 0.06, 0.3);
    let at = 0;
    for (let k = 0; k < 3 + Math.floor(mulberry32.sample(rng) * 3); k++) {
      mix(out, pop, sr, 0.6 + 0.4 * mulberry32.sample(rng), () => 1, Math.round(at * sr));
      at += 0.09 + 0.18 * mulberry32.sample(rng);
    }
    return toPeak(out);
  }),
  cue_engine: once((sr, rng) => {
    const e = engine(sr, rng, 30, 1.2, 0.1);
    for (let i = 0; i < e.length; i++) e[i] *= Math.sin((Math.PI * i) / e.length) ** 2;
    return toPeak(biquad(e, sr, "lowpass", 260));
  }),
  cue_footsteps: once((sr, rng) => {
    const n = Math.round(1.2 * sr);
    const out = new Float32Array(n);
    const m = Math.round(0.1 * sr);
    for (let k = 0; k < 4; k++) {
      const step = biquad(noise(m, rng), sr, "lowpass", 700);
      mix(out, step, sr, 1, decay(0.02, 0.003), Math.round((0.1 + k * 0.27) * sr));
    }
    return toPeak(out);
  }),
  cue_voices: once((sr, rng) => {
    // A murmur: formant-like bands under a syllable rhythm.
    const n = Math.round(1.2 * sr);
    const a = biquad(noise(n, rng), sr, "bandpass", 500, 3);
    const b = biquad(noise(n, rng), sr, "bandpass", 1300, 3);
    const out = new Float32Array(n);
    for (let i = 0; i < n; i++) {
      const t = i / sr;
      const syll = Math.max(0, Math.sin(TAU * 4.3 * t + 2 * Math.sin(TAU * 1.1 * t)));
      out[i] = (a[i] + 0.6 * b[i]) * syll * Math.sin((Math.PI * i) / n);
    }
    return toPeak(biquad(out, sr, "lowpass", 1500));
  }),
};

/** The summer bed, stereo: gusting wind, leaves in the gusts, birdsong, and
 *  crickets. Gusts and chirps are placed on the loop, so it wraps. */
function countryside(sr: number, rng: Rng): Float32Array[] {
  const len = 12;
  const n = Math.round(len * sr);
  const gust = (i: number, ph: number) =>
    0.55 + 0.3 * Math.sin((TAU * 2 * i) / n + ph) + 0.15 * Math.sin((TAU * 5 * i) / n + ph * 2);
  const channels: Float32Array[] = [];
  for (let c = 0; c < 2; c++) {
    const ph = c * 0.7;
    const wind = seamless(n, Math.round(0.3 * sr), (m) =>
      biquad(noise(m, rng), sr, "lowpass", 380),
    );
    const leaves = seamless(n, Math.round(0.3 * sr), (m) =>
      biquad(noise(m, rng), sr, "highpass", 3500),
    );
    const out = new Float32Array(n);
    for (let i = 0; i < n; i++) {
      const g = gust(i, ph);
      out[i] = 2.2 * wind[i] * g + 0.05 * leaves[i] * g * g;
    }
    channels.push(out);
  }
  // Birds: phrases of chirps, each in one ear more than the other.
  for (let p = 0; p < 9; p++) {
    const at = mulberry32.sample(rng) * (len - 1.5);
    const pan = mulberry32.sample(rng);
    const base = 2600 + 2400 * mulberry32.sample(rng);
    const notes = 2 + Math.floor(mulberry32.sample(rng) * 5);
    for (let k = 0; k < notes; k++) {
      const d = 0.05 + 0.1 * mulberry32.sample(rng);
      const m = Math.round(d * sr);
      const up = mulberry32.sample(rng) < 0.5;
      const chirp = tone(
        m,
        sr,
        (t) => base * (up ? 1 + (0.4 * t) / d : 1.4 - (0.4 * t) / d),
        (t) => Math.sin((Math.PI * t) / d) ** 2,
      );
      const start = Math.round((at + k * (d + 0.04 + 0.06 * mulberry32.sample(rng))) * sr);
      mix(channels[0], chirp, sr, 0.06 * (1 - pan * 0.7), () => 1, start);
      mix(channels[1], chirp, sr, 0.06 * (0.3 + pan * 0.7), () => 1, start);
    }
  }
  // Crickets: a 4.6 kHz carrier pulsed 30 times a second, in trills.
  for (let i = 0; i < n; i++) {
    const t = i / sr;
    const trill = Math.sin((TAU * 3 * i) / n) > -0.2 ? 1 : 0;
    const pulse = Math.max(0, Math.sin(TAU * 30 * t)) ** 4;
    const s = 0.012 * trill * pulse * Math.sin(TAU * 4600 * t);
    channels[0][i] += s;
    channels[1][i] += s * 0.7;
  }
  let m = 0;
  for (const ch of channels) for (let i = 0; i < n; i++) m = Math.max(m, Math.abs(ch[i]));
  for (const ch of channels) for (let i = 0; i < n; i++) ch[i] *= PEAK / m;
  return channels;
}

/** Synthesise `name` at `sampleRate`: the same samples every time. */
export function synthesize(name: string, sampleRate: number): SynthSound {
  const make = SOUNDS[name];
  if (!make) throw new Error(`no sound named ${name}`);
  return make(sampleRate, mulberry32.create(hashString(name)));
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
    for (let i = gap; i < n; i++) x[i] = white(rng) * Math.exp(-(i - gap) / sampleRate / tau);
    biquad(x, sampleRate, "lowpass", (i) => 6000 * Math.exp((-2 * i) / n) + 800);
    let e = 0;
    for (let i = 0; i < n; i++) e += x[i] * x[i];
    const k = 1 / Math.sqrt(e || 1);
    for (let i = 0; i < n; i++) x[i] *= k;
    return x;
  });
}
