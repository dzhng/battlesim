//! The synthesized baseline: every sound is a deterministic function of its
//! name and sample rate (seeded noise, filters, envelopes), so both clients
//! play the same samples. Transients are shaped noise and sines; loops wrap
//! seamlessly (whole cycles, or a crossfaded seam). Each sound peaks at
//! `PEAK`; the mix's levels are `audio.json`'s. Samples are stored
//! as f32 and worked in f64, and the transcendental functions are `libm`'s,
//! so the browser's WebAssembly and native agree bit for bit.
use std::f64::consts::PI;

use libm::{cos, exp, floor, pow, sin};

use crate::seed::{Key, Mulberry32};

/// Every sound's peak before the mix.
const PEAK: f64 = 0.9;
const TAU: f64 = PI * 2.0;

/// One synthesized sound: a channel per array (the bed is stereo).
pub struct Synth {
    pub channels: Vec<Vec<f32>>,
    pub looping: bool,
}

fn len(seconds: f64, sr: f64) -> usize {
    f64::round(seconds * sr) as usize
}

fn white(rng: &mut Mulberry32) -> f64 {
    rng.sample() * 2.0 - 1.0
}

#[derive(Clone, Copy)]
enum Filter {
    Lowpass,
    Highpass,
    Bandpass,
}

/// An RBJ biquad run in place over `x`, its cutoff `hz(i)` taken every 32
/// samples (fixed when `fixed`).
fn biquad_by(x: &mut [f32], sr: f64, kind: Filter, hz: impl Fn(usize) -> f64, q: f64, fixed: bool) {
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let (mut b0, mut b1, mut b2, mut a1, mut a2) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut last = -1.0;
    for (i, v) in x.iter_mut().enumerate() {
        let f = hz(i);
        if f != last && (if fixed { i == 0 } else { i & 31 == 0 }) {
            last = f;
            let w = TAU * f.min(sr * 0.45) / sr;
            let cw = cos(w);
            let alpha = sin(w) / (2.0 * q);
            let a0 = 1.0 + alpha;
            match kind {
                Filter::Lowpass => {
                    b0 = (1.0 - cw) / 2.0 / a0;
                    b1 = (1.0 - cw) / a0;
                    b2 = b0;
                }
                Filter::Highpass => {
                    b0 = (1.0 + cw) / 2.0 / a0;
                    b1 = -(1.0 + cw) / a0;
                    b2 = b0;
                }
                Filter::Bandpass => {
                    b0 = alpha / a0;
                    b1 = 0.0;
                    b2 = -alpha / a0;
                }
            }
            a1 = (-2.0 * cw) / a0;
            a2 = (1.0 - alpha) / a0;
        }
        let xi = f64::from(*v);
        let y = b0 * xi + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        x2 = x1;
        x1 = xi;
        y2 = y1;
        y1 = y;
        *v = y as f32;
    }
}

fn biquad(mut x: Vec<f32>, sr: f64, kind: Filter, hz: f64, q: f64) -> Vec<f32> {
    biquad_by(&mut x, sr, kind, |_| hz, q, true);
    x
}

const Q: f64 = 0.707;

fn noise(n: usize, rng: &mut Mulberry32) -> Vec<f32> {
    (0..n).map(|_| white(rng) as f32).collect()
}

/// `into += gain × src × env(t)`, from sample `at`.
fn mix(into: &mut [f32], src: &[f32], sr: f64, gain: f64, env: impl Fn(f64) -> f64, at: usize) {
    for (i, s) in src.iter().enumerate() {
        let Some(o) = into.get_mut(i + at) else { break };
        *o = (f64::from(*o) + gain * f64::from(*s) * env(i as f64 / sr)) as f32;
    }
}

fn decay(tau: f64, attack: f64) -> impl Fn(f64) -> f64 {
    move |t| (t / attack).min(1.0) * exp(-t / tau)
}

const fn one(_: f64) -> f64 {
    1.0
}

/// A sine whose frequency follows `hz(t)`, under `env`.
fn tone(n: usize, sr: f64, hz: impl Fn(f64) -> f64, env: impl Fn(f64) -> f64) -> Vec<f32> {
    let mut phase = 0.0;
    (0..n)
        .map(|i| {
            let t = i as f64 / sr;
            phase += TAU * hz(t) / sr;
            (sin(phase) * env(t)) as f32
        })
        .collect()
}

fn to_peak(mut x: Vec<f32>) -> Vec<f32> {
    let m = x.iter().fold(0.0f64, |m, v| m.max(f64::from(v.abs())));
    if m > 0.0 {
        let k = PEAK / m;
        for v in &mut x {
            *v = (f64::from(*v) * k) as f32;
        }
    }
    x
}

/// A loop of `n` samples from `make(n + fade)`: the tail is crossfaded over
/// the head, so it wraps without a click.
pub fn seamless(n: usize, fade: usize, make: impl FnOnce(usize) -> Vec<f32>) -> Vec<f32> {
    let long = make(n + fade);
    let mut out = long[..n].to_vec();
    for i in 0..fade {
        let a = i as f64 / fade as f64;
        out[i] = (f64::from(out[i]) * a + f64::from(long[n + i]) * (1.0 - a)) as f32;
    }
    out
}

struct Report {
    len: f64,
    crack: f64,
    body: f64,
    body_tau: f64,
    thump_hz: f64,
    thump: f64,
    thump_tau: f64,
    tail: f64,
    tail_tau: f64,
    tail_hz: f64,
}

/// A report: a crack, a bandpassed body, a low thump and a room tail.
fn report(sr: f64, rng: &mut Mulberry32, o: Report) -> Vec<f32> {
    let n = len(o.len, sr);
    let mut out = vec![0.0f32; n];
    let crack = biquad(noise(n, rng), sr, Filter::Highpass, 2500.0, Q);
    mix(&mut out, &crack, sr, o.crack, decay(0.004, 0.0005), 0);
    let body = biquad(noise(n, rng), sr, Filter::Bandpass, o.body, 0.8);
    mix(&mut out, &body, sr, 2.5, decay(o.body_tau, 0.0005), 0);
    let thump = tone(
        n,
        sr,
        |t| o.thump_hz * (1.0 + 0.8 * exp(-t / 0.01)),
        decay(o.thump_tau, 0.001),
    );
    mix(&mut out, &thump, sr, o.thump, one, 0);
    let tail = biquad(noise(n, rng), sr, Filter::Lowpass, o.tail_hz, Q);
    let tail_tau = o.tail_tau;
    mix(
        &mut out,
        &tail,
        sr,
        o.tail,
        |t| (t / 0.02).min(1.0) * exp(-t / tail_tau),
        0,
    );
    to_peak(out)
}

/// A far report: low-passed, a soft attack and a rolling echo.
fn far_report(
    sr: f64,
    rng: &mut Mulberry32,
    seconds: f64,
    hz: f64,
    tau: f64,
    echo: f64,
) -> Vec<f32> {
    let n = len(seconds, sr);
    let mut out = vec![0.0f32; n];
    let boom = biquad(noise(n, rng), sr, Filter::Lowpass, hz, Q);
    mix(
        &mut out,
        &boom,
        sr,
        1.0,
        |t| (t / 0.008).min(1.0) * exp(-t / tau),
        0,
    );
    mix(
        &mut out,
        &boom,
        sr,
        echo,
        |t| (t / 0.05).min(1.0) * exp(-t / (tau * 2.5)),
        len(0.35, sr),
    );
    to_peak(biquad(out, sr, Filter::Lowpass, hz * 1.5, Q))
}

fn explosion(sr: f64, rng: &mut Mulberry32, seconds: f64, far: bool) -> Vec<f32> {
    let n = len(seconds, sr);
    let mut out = vec![0.0f32; n];
    let boom = tone(n, sr, |t| 55.0 * exp(-t / 0.4) + 28.0, decay(0.35, 0.002));
    mix(&mut out, &boom, sr, 1.4, one, 0);
    let body = biquad(
        noise(n, rng),
        sr,
        Filter::Lowpass,
        if far { 350.0 } else { 2200.0 },
        Q,
    );
    mix(
        &mut out,
        &body,
        sr,
        if far { 1.6 } else { 1.2 },
        decay(if far { 0.45 } else { 0.25 }, 0.002),
        0,
    );
    let rumble = biquad(noise(n, rng), sr, Filter::Lowpass, 160.0, Q);
    mix(
        &mut out,
        &rumble,
        sr,
        2.2,
        |t| (t / 0.05).min(1.0) * exp(-t / 0.9),
        0,
    );
    if !far {
        let crack = biquad(noise(n, rng), sr, Filter::Highpass, 3000.0, Q);
        mix(&mut out, &crack, sr, 0.8, decay(0.005, 0.0005), 0);
        // Debris pattering down.
        for _ in 0..40 {
            let at = 0.25 + rng.sample() * (seconds - 0.6);
            let m = len(0.03, sr);
            let pattern = noise(m, rng);
            let click = biquad(
                pattern,
                sr,
                Filter::Bandpass,
                1200.0 + 2000.0 * rng.sample(),
                2.0,
            );
            mix(
                &mut out,
                &click,
                sr,
                0.15 * exp(-at / 1.2),
                decay(0.008, 0.0005),
                len(at, sr),
            );
        }
    }
    to_peak(out)
}

/// An engine at `hz` firings a second: a jittered pulse train through its
/// exhaust (a low-pass), with mechanical noise. Whole cycles, so it loops.
fn engine(sr: f64, rng: &mut Mulberry32, hz: f64, seconds: f64, bright: f64) -> Vec<f32> {
    let cycles = f64::round(hz * seconds);
    let n = f64::round(cycles / hz * sr) as usize;
    let mut out = vec![0.0f32; n];
    let period = n as f64 / cycles;
    for c in 0..cycles as usize {
        let amp = 0.75 + 0.25 * rng.sample();
        let at = f64::round(c as f64 * period) as usize;
        let mut i = 0usize;
        while (i as f64) < period * 0.9 && at + i < n {
            let fi = i as f64;
            let v = amp * exp(-fi / (period * 0.18)) * sin(TAU * fi / (period * 0.45));
            out[at + i] = (f64::from(out[at + i]) + v) as f32;
            i += 1;
        }
    }
    let mut out = biquad(out, sr, Filter::Lowpass, 180.0 + 500.0 * bright, 1.2);
    let mech = seamless(n, len(0.05, sr), |m| {
        biquad(
            noise(m, rng),
            sr,
            Filter::Bandpass,
            900.0 + 1500.0 * bright,
            1.5,
        )
    });
    for (i, o) in out.iter_mut().enumerate() {
        let swell = 0.8 + 0.2 * sin(TAU * i as f64 * 3.0 / n as f64);
        *o = (f64::from(*o) + 0.12 * f64::from(mech[i]) * swell) as f32;
    }
    to_peak(out)
}

fn rifle(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    report(
        sr,
        rng,
        Report {
            len: 0.7,
            crack: 1.2,
            body: 1500.0,
            body_tau: 0.025,
            thump_hz: 120.0,
            thump: 0.6,
            thump_tau: 0.03,
            tail: 0.35,
            tail_tau: 0.18,
            tail_hz: 1800.0,
        },
    )
}

fn hmg(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    report(
        sr,
        rng,
        Report {
            len: 0.7,
            crack: 1.0,
            body: 800.0,
            body_tau: 0.035,
            thump_hz: 85.0,
            thump: 1.0,
            thump_tau: 0.045,
            tail: 0.45,
            tail_tau: 0.22,
            tail_hz: 1200.0,
        },
    )
}

fn thump(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(0.35, sr);
    let mut out = tone(
        n,
        sr,
        |t| 150.0 + 200.0 * exp(-t / 0.01),
        decay(0.05, 0.001),
    );
    let body = biquad(noise(n, rng), sr, Filter::Bandpass, 700.0, 1.0);
    mix(&mut out, &body, sr, 1.2, decay(0.03, 0.0005), 0);
    to_peak(out)
}

fn launch(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(1.2, sr);
    let mut out = vec![0.0f32; n];
    let mut rush = noise(n, rng);
    biquad_by(
        &mut rush,
        sr,
        Filter::Bandpass,
        |i| 2400.0 - 1600.0 * (i as f64 / sr / 0.8).min(1.0),
        1.2,
        false,
    );
    mix(
        &mut out,
        &rush,
        sr,
        2.0,
        |t| (t / 0.02).min(1.0) * exp(-t / 0.4),
        0,
    );
    let thud = tone(n, sr, |_| 90.0, decay(0.06, 0.001));
    mix(&mut out, &thud, sr, 0.8, one, 0);
    to_peak(out)
}

fn impact_ground(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(0.45, sr);
    let mut out = tone(n, sr, |t| 70.0 + 60.0 * exp(-t / 0.02), decay(0.06, 0.001));
    let dirt = biquad(noise(n, rng), sr, Filter::Lowpass, 900.0, Q);
    mix(&mut out, &dirt, sr, 1.5, decay(0.08, 0.0005), 0);
    let patter = biquad(noise(n, rng), sr, Filter::Bandpass, 2500.0, 1.0);
    mix(
        &mut out,
        &patter,
        sr,
        0.4,
        |t| {
            if t < 0.05 {
                0.0
            } else {
                exp(-(t - 0.05) / 0.08) * (0.5 + 0.5 * sin(t * 190.0))
            }
        },
        0,
    );
    to_peak(out)
}

fn impact_hull(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(0.7, sr);
    let mut out = vec![0.0f32; n];
    for (hz, a, tau) in [
        (523.0, 1.0, 0.25),
        (1187.0, 0.7, 0.18),
        (1873.0, 0.5, 0.12),
        (2711.0, 0.4, 0.08),
        (3907.0, 0.25, 0.05),
    ] {
        let ring = tone(n, sr, |_| hz, decay(tau, 0.0005));
        mix(&mut out, &ring, sr, a, one, 0);
    }
    let crack = biquad(noise(n, rng), sr, Filter::Highpass, 2000.0, Q);
    mix(&mut out, &crack, sr, 1.2, decay(0.006, 0.0005), 0);
    to_peak(out)
}

fn impact_prop(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(0.35, sr);
    let mut out = tone(n, sr, |_| 220.0, decay(0.04, 0.0005));
    let knock = biquad(noise(n, rng), sr, Filter::Bandpass, 1600.0, 1.2);
    mix(&mut out, &knock, sr, 2.0, decay(0.035, 0.0005), 0);
    let rattle = biquad(noise(n, rng), sr, Filter::Lowpass, 3000.0, Q);
    mix(
        &mut out,
        &rattle,
        sr,
        0.3,
        |t| {
            if t < 0.04 {
                0.0
            } else {
                exp(-(t - 0.04) / 0.1)
            }
        },
        0,
    );
    to_peak(out)
}

/// Noise through `kind` at `hz`, under `env`.
fn shaped(
    n: usize,
    sr: f64,
    rng: &mut Mulberry32,
    kind: Filter,
    hz: f64,
    q: f64,
    env: impl Fn(f64) -> f64,
) -> Vec<f32> {
    let mut out = biquad(noise(n, rng), sr, kind, hz, q);
    for (i, v) in out.iter_mut().enumerate() {
        *v = (f64::from(*v) * env(i as f64 / sr)) as f32;
    }
    out
}

fn impact_soldier(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(0.18, sr);
    to_peak(shaped(
        n,
        sr,
        rng,
        Filter::Lowpass,
        450.0,
        Q,
        decay(0.035, 0.002),
    ))
}

fn ricochet(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(0.65, sr);
    let start = 2600.0 + 1200.0 * rng.sample();
    let mut out = tone(
        n,
        sr,
        |t| start * exp(-t / 0.5) * (1.0 + 0.03 * sin(TAU * 38.0 * t)),
        |t| (t / 0.004).min(1.0) * exp(-t / 0.22),
    );
    let crack = biquad(noise(n, rng), sr, Filter::Highpass, 3000.0, Q);
    mix(&mut out, &crack, sr, 0.6, decay(0.004, 0.0005), 0);
    to_peak(out)
}

fn footstep(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(0.14, sr);
    let mut out = shaped(
        n,
        sr,
        rng,
        Filter::Bandpass,
        1400.0,
        0.9,
        decay(0.02, 0.002),
    );
    let thud = tone(n, sr, |_| 95.0, decay(0.025, 0.002));
    mix(&mut out, &thud, sr, 0.5, one, 0);
    to_peak(out)
}

fn motor(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(1.5, sr);
    to_peak(seamless(n, len(0.1, sr), |m| {
        let mut x = biquad(noise(m, rng), sr, Filter::Bandpass, 1100.0, 0.6);
        let low = biquad(noise(m, rng), sr, Filter::Lowpass, 220.0, Q);
        for i in 0..m {
            let flutter = 0.85 + 0.15 * white(rng);
            x[i] = (f64::from(x[i]) * flutter + 1.5 * f64::from(low[i])) as f32;
        }
        x
    }))
}

/// A helicopter's rotor: rushing air slapped by each blade's pass, 17 a
/// second at rate 1 (whole passes, so it loops), whatever its speed.
fn rotor(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let passes = 34.0;
    let n = f64::round(passes / 17.0 * sr) as usize;
    let air = seamless(n, len(0.05, sr), |m| {
        biquad(noise(m, rng), sr, Filter::Lowpass, 500.0, Q)
    });
    let out = (0..n)
        .map(|i| {
            let phase = (i as f64 * passes / n as f64) % 1.0;
            (f64::from(air[i]) * (0.3 + exp(-phase / 0.15))) as f32
        })
        .collect();
    to_peak(out)
}

/// Track links striking the sprockets, 10 a second at rate 1, and a squeal.
fn tracks(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(2.0, sr);
    let mut out = vec![0.0f32; n];
    for k in 0..20 {
        let at = f64::round(k as f64 / 20.0 * n as f64 + (rng.sample() - 0.5) * 0.01 * sr) as i64;
        let m = len(0.06, sr);
        let pattern = noise(m, rng);
        let clank = biquad(
            pattern,
            sr,
            Filter::Bandpass,
            700.0 + 900.0 * rng.sample(),
            3.0,
        );
        for (i, c) in clank.iter().enumerate() {
            let o = (at + i as i64 + n as i64) as usize % n;
            out[o] = (f64::from(out[o]) + f64::from(*c) * exp(-(i as f64) / sr / 0.012)) as f32;
        }
    }
    let rumble = seamless(n, len(0.05, sr), |m| {
        biquad(noise(m, rng), sr, Filter::Lowpass, 140.0, Q)
    });
    for (o, r) in out.iter_mut().zip(&rumble) {
        *o = (f64::from(*o) + 0.8 * f64::from(*r)) as f32;
    }
    to_peak(out)
}

fn wheels(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(2.0, sr);
    to_peak(seamless(n, len(0.1, sr), |m| {
        let mut x = biquad(noise(m, rng), sr, Filter::Lowpass, 260.0, Q);
        let grit = biquad(noise(m, rng), sr, Filter::Bandpass, 2500.0, 1.0);
        for i in 0..m {
            let stone = if rng.sample() < 0.02 { 4.0 } else { 1.0 };
            x[i] = (2.0 * f64::from(x[i]) + 0.08 * f64::from(grit[i]) * stone) as f32;
        }
        x
    }))
}

/// A servo's whine (whole cycles in the loop) over gear noise.
fn turret(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(1.0, sr);
    let mut out = vec![0.0f32; n];
    for (hz, a) in [(180.0, 1.0), (360.0, 0.5), (540.0, 0.25)] {
        for (i, o) in out.iter_mut().enumerate() {
            let i = i as f64;
            let v = a * sin(TAU * hz * i / sr) * (0.8 + 0.2 * sin(TAU * 6.0 * i / sr));
            *o = (f64::from(*o) + v) as f32;
        }
    }
    let gear = seamless(n, len(0.05, sr), |m| {
        biquad(noise(m, rng), sr, Filter::Bandpass, 1200.0, 2.0)
    });
    for (o, g) in out.iter_mut().zip(&gear) {
        *o = (f64::from(*o) + 0.5 * f64::from(*g)) as f32;
    }
    to_peak(out)
}

/// A back-up alarm: a bright 1.1 kHz tone, half a second on, half off, its
/// edges ramped over 5 ms so it beeps rather than clicks.
fn reverse_beep(sr: f64, _: &mut Mulberry32) -> Vec<f32> {
    let n = len(1.0, sr);
    let mut out = vec![0.0f32; n];
    let (beep, edge) = (0.5, 0.005);
    for (i, o) in out.iter_mut().enumerate() {
        let t = i as f64 / sr;
        if t >= beep {
            break;
        }
        let env = (t / edge).min((beep - t) / edge).min(1.0);
        let v = env
            * (sin(TAU * 1100.0 * t) + 0.25 * sin(TAU * 2200.0 * t) + 0.3 * sin(TAU * 3300.0 * t));
        *o = v as f32;
    }
    to_peak(out)
}

/// Crackle over a low roar (the smoke's rumble), wrapping at the seam.
fn fire(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(4.0, sr);
    let mut out = seamless(n, len(0.2, sr), |m| {
        let mut x = biquad(noise(m, rng), sr, Filter::Lowpass, 180.0, Q);
        let hiss = biquad(noise(m, rng), sr, Filter::Bandpass, 3000.0, 0.7);
        for i in 0..m {
            let swell = 0.8 + 0.2 * sin(TAU * 0.5 * i as f64 / sr);
            x[i] = (3.0 * f64::from(x[i]) * swell + 0.1 * f64::from(hiss[i])) as f32;
        }
        x
    });
    for _ in 0..90 {
        let at = floor(rng.sample() * n as f64) as usize;
        let m = len(0.012, sr);
        let pattern = noise(m, rng);
        let pop = biquad(
            pattern,
            sr,
            Filter::Bandpass,
            1500.0 + 3500.0 * rng.sample(),
            2.0,
        );
        let a = 0.3 + 1.4 * pow(rng.sample(), 3.0);
        for (i, p) in pop.iter().enumerate() {
            let o = (at + i) % n;
            out[o] = (f64::from(out[o]) + a * f64::from(*p) * exp(-(i as f64) / sr / 0.003)) as f32;
        }
    }
    to_peak(out)
}

fn cue_gunfire(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(1.3, sr);
    let mut out = vec![0.0f32; n];
    let pop = far_report(sr, rng, 0.5, 700.0, 0.06, 0.3);
    let mut at = 0.0;
    let mut k = 0.0;
    // The burst's length is drawn again before each pop, as it always was.
    while k < 3.0 + floor(rng.sample() * 3.0) {
        let gain = 0.6 + 0.4 * rng.sample();
        mix(&mut out, &pop, sr, gain, one, len(at, sr));
        at += 0.09 + 0.18 * rng.sample();
        k += 1.0;
    }
    to_peak(out)
}

fn cue_engine(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let mut e = engine(sr, rng, 30.0, 1.2, 0.1);
    let n = e.len() as f64;
    for (i, v) in e.iter_mut().enumerate() {
        *v = (f64::from(*v) * pow(sin(PI * i as f64 / n), 2.0)) as f32;
    }
    to_peak(biquad(e, sr, Filter::Lowpass, 260.0, Q))
}

fn cue_footsteps(sr: f64, rng: &mut Mulberry32) -> Vec<f32> {
    let n = len(1.2, sr);
    let mut out = vec![0.0f32; n];
    let m = len(0.1, sr);
    for k in 0..4 {
        let step = biquad(noise(m, rng), sr, Filter::Lowpass, 700.0, Q);
        mix(
            &mut out,
            &step,
            sr,
            1.0,
            decay(0.02, 0.003),
            len(0.1 + k as f64 * 0.27, sr),
        );
    }
    to_peak(out)
}

/// The summer bed, stereo: gusting wind, leaves in the gusts, birdsong, and
/// crickets. Gusts and chirps are placed on the loop, so it wraps.
fn countryside(sr: f64, rng: &mut Mulberry32) -> Vec<Vec<f32>> {
    let seconds = 12.0;
    let n = len(seconds, sr);
    let nf = n as f64;
    let gust = |i: f64, ph: f64| {
        0.55 + 0.3 * sin(TAU * 2.0 * i / nf + ph) + 0.15 * sin(TAU * 5.0 * i / nf + ph * 2.0)
    };
    let mut channels = Vec::new();
    for c in 0..2 {
        let ph = c as f64 * 0.7;
        let wind = seamless(n, len(0.3, sr), |m| {
            biquad(noise(m, rng), sr, Filter::Lowpass, 380.0, Q)
        });
        let leaves = seamless(n, len(0.3, sr), |m| {
            biquad(noise(m, rng), sr, Filter::Highpass, 3500.0, Q)
        });
        let out: Vec<f32> = (0..n)
            .map(|i| {
                let g = gust(i as f64, ph);
                (2.2 * f64::from(wind[i]) * g + 0.05 * f64::from(leaves[i]) * g * g) as f32
            })
            .collect();
        channels.push(out);
    }
    // Birds: phrases of chirps, each in one ear more than the other.
    for _ in 0..9 {
        let at = rng.sample() * (seconds - 1.5);
        let pan = rng.sample();
        let base = 2600.0 + 2400.0 * rng.sample();
        let notes = 2 + floor(rng.sample() * 5.0) as usize;
        for k in 0..notes {
            let d = 0.05 + 0.1 * rng.sample();
            let m = len(d, sr);
            let up = rng.sample() < 0.5;
            let chirp = tone(
                m,
                sr,
                |t| {
                    base * if up {
                        1.0 + 0.4 * t / d
                    } else {
                        1.4 - 0.4 * t / d
                    }
                },
                |t| pow(sin(PI * t / d), 2.0),
            );
            let start = len(at + k as f64 * (d + 0.04 + 0.06 * rng.sample()), sr);
            mix(
                &mut channels[0],
                &chirp,
                sr,
                0.06 * (1.0 - pan * 0.7),
                one,
                start,
            );
            mix(
                &mut channels[1],
                &chirp,
                sr,
                0.06 * (0.3 + pan * 0.7),
                one,
                start,
            );
        }
    }
    // Crickets: a 4.6 kHz carrier pulsed 30 times a second, in trills.
    let [left, right] = &mut channels[..] else {
        unreachable!("two channels")
    };
    for (i, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
        let t = i as f64 / sr;
        let trill = if sin(TAU * 3.0 * i as f64 / nf) > -0.2 {
            1.0
        } else {
            0.0
        };
        let pulse = pow(sin(TAU * 30.0 * t).max(0.0), 4.0);
        let s = 0.012 * trill * pulse * sin(TAU * 4600.0 * t);
        *l = (f64::from(*l) + s) as f32;
        *r = (f64::from(*r) + s * 0.7) as f32;
    }
    let m = channels
        .iter()
        .flatten()
        .fold(0.0f64, |m, v| m.max(f64::from(v.abs())));
    for v in channels.iter_mut().flatten() {
        *v = (f64::from(*v) * (PEAK / m)) as f32;
    }
    channels
}

/// Distant battle bed, stereo: sparse distinct events with large dynamic swings.
/// Far-off booms (artillery), MG bursts (short rattles), rifle pops; minimal wash.
/// Events are loud and separated, with 10+ dB swings between events and silence.
fn distant_battle(sr: f64, rng: &mut Mulberry32) -> Vec<Vec<f32>> {
    let seconds = 20.0;
    let n = len(seconds, sr);

    // Start with near-silence (very faint sub-bass texture, barely audible)
    let mut channels = Vec::new();
    for c in 0..2 {
        let ph = c as f64 * 0.5;
        let wash = seamless(n, len(0.8, sr), |m| {
            biquad(noise(m, rng), sr, Filter::Lowpass, 40.0, 0.4)
        });
        let out: Vec<f32> = (0..n)
            .map(|i| {
                let swell = 0.3 + 0.2 * sin(TAU * 0.08 * i as f64 / n as f64 + ph);
                (f64::from(wash[i]) * 0.03 * swell) as f32
            })
            .collect();
        channels.push(out);
    }

    // Distinct far-off booms (artillery/tank): 3-4 per loop, LOUD with long tails
    let boom_count = 3 + floor(rng.sample() * 2.0) as usize;
    for k in 0..boom_count {
        // Space booms apart with gaps (at least 4s apart)
        let at = 1.5 + (k as f64 / boom_count as f64) * (seconds - 5.0) + rng.sample() * 2.0;
        if at >= seconds - 2.5 {
            continue;
        }
        let pan = 0.15 + 0.7 * rng.sample();
        let dur = 1.4 + 0.6 * rng.sample();
        let freq = 26.0 + 14.0 * rng.sample();
        let m = len(dur, sr);
        let boom: Vec<f32> = (0..m)
            .map(|i| {
                let t = i as f64 / sr;
                let attack = (t * 80.0).min(1.0);
                let decay = exp(-t * 2.0 / dur);
                let env = attack * decay;
                let tone = sin(TAU * freq * t) * 0.8
                    + sin(TAU * freq * 1.5 * t) * 0.15
                    + sin(TAU * freq * 2.2 * t) * 0.05;
                (env * tone) as f32
            })
            .collect();
        let boom = biquad(boom, sr, Filter::Lowpass, 80.0, 0.35);
        let start = len(at, sr);
        // High gain for booms to create large dynamic swings
        let gain = 0.9 + 0.1 * rng.sample();
        mix(
            &mut channels[0],
            &boom,
            sr,
            gain * (1.0 - pan * 0.35),
            one,
            start,
        );
        mix(
            &mut channels[1],
            &boom,
            sr,
            gain * (0.65 + pan * 0.35),
            one,
            start,
        );
    }

    // MG bursts: 2-3 short rattles, well-separated
    let burst_count = 2 + floor(rng.sample() * 2.0) as usize;
    for k in 0..burst_count {
        // Space bursts apart
        let at = 0.5 + (k as f64 / burst_count as f64) * (seconds - 3.0) + rng.sample() * 1.5;
        if at >= seconds - 1.5 {
            continue;
        }
        let pan = rng.sample();
        let shots = 3 + floor(rng.sample() * 4.0) as usize;
        let interval = 0.07 + 0.03 * rng.sample();
        for j in 0..shots {
            let shot_at = at + j as f64 * interval;
            if shot_at >= seconds - 0.3 {
                break;
            }
            let dur = 0.05 + 0.02 * rng.sample();
            let freq = 380.0 + 120.0 * rng.sample();
            let m = len(dur, sr);
            let mut pop = noise(m, rng);
            biquad_by(&mut pop, sr, Filter::Bandpass, |_| freq, 1.8, true);
            for (i, v) in pop.iter_mut().enumerate() {
                let t = i as f64 / sr;
                let env = exp(-t * 10.0 / dur);
                *v = (*v as f64 * env * 0.35) as f32;
            }
            let start = len(shot_at, sr);
            let gain = 0.6 + 0.25 * rng.sample();
            mix(
                &mut channels[0],
                &pop,
                sr,
                gain * (1.0 - pan * 0.5),
                one,
                start,
            );
            mix(
                &mut channels[1],
                &pop,
                sr,
                gain * (0.5 + pan * 0.5),
                one,
                start,
            );
        }
    }

    // Single rifle pops: 3-5 isolated shots, spaced out
    let pop_count = 3 + floor(rng.sample() * 3.0) as usize;
    for k in 0..pop_count {
        // Space pops with gaps
        let at = 0.3 + (k as f64 / pop_count as f64) * (seconds - 2.0) + rng.sample() * 1.0;
        if at >= seconds - 0.5 {
            continue;
        }
        let pan = rng.sample();
        let dur = 0.035 + 0.015 * rng.sample();
        let freq = 550.0 + 180.0 * rng.sample();
        let m = len(dur, sr);
        let mut pop = noise(m, rng);
        biquad_by(&mut pop, sr, Filter::Bandpass, |_| freq, 2.2, true);
        for (i, v) in pop.iter_mut().enumerate() {
            let t = i as f64 / sr;
            let env = exp(-t * 14.0 / dur);
            *v = (*v as f64 * env * 0.25) as f32;
        }
        let start = len(at, sr);
        let gain = 0.4 + 0.2 * rng.sample();
        mix(
            &mut channels[0],
            &pop,
            sr,
            gain * (1.0 - pan * 0.6),
            one,
            start,
        );
        mix(
            &mut channels[1],
            &pop,
            sr,
            gain * (0.4 + pan * 0.6),
            one,
            start,
        );
    }

    // Normalize to peak, but since wash is very quiet and events are loud,
    // the dynamic range between events and silence will be large (10+ dB)
    let m = channels
        .iter()
        .flatten()
        .fold(0.0f64, |m, v| m.max(f64::from(v.abs())));
    for v in channels.iter_mut().flatten() {
        *v = (f64::from(*v) * (PEAK / m)) as f32;
    }
    channels
}

type Mono = fn(f64, &mut Mulberry32) -> Vec<f32>;

/// Every mono sound, by name, and whether it loops.
const SOUNDS: &[(&str, Mono, bool)] = &[
    ("rifle", rifle, false),
    (
        "rifle_far",
        |sr, rng| far_report(sr, rng, 1.2, 900.0, 0.09, 0.35),
        false,
    ),
    ("hmg", hmg, false),
    (
        "hmg_far",
        |sr, rng| far_report(sr, rng, 1.3, 600.0, 0.11, 0.4),
        false,
    ),
    ("cannon", |sr, rng| explosion(sr, rng, 2.4, false), false),
    ("cannon_far", |sr, rng| explosion(sr, rng, 2.8, true), false),
    ("thump", thump, false),
    ("launch", launch, false),
    ("impact_ground", impact_ground, false),
    ("impact_hull", impact_hull, false),
    ("impact_prop", impact_prop, false),
    ("impact_soldier", impact_soldier, false),
    ("ricochet", ricochet, false),
    ("explosion", |sr, rng| explosion(sr, rng, 3.2, false), false),
    (
        "explosion_far",
        |sr, rng| explosion(sr, rng, 3.4, true),
        false,
    ),
    ("footstep", footstep, false),
    ("motor", motor, true),
    (
        "engine_diesel",
        |sr, rng| engine(sr, rng, 28.0, 2.0, 0.2),
        true,
    ),
    (
        "engine_small",
        |sr, rng| engine(sr, rng, 42.0, 2.0, 0.6),
        true,
    ),
    ("rotor", rotor, true),
    ("tracks", tracks, true),
    ("wheels", wheels, true),
    ("turret", turret, true),
    ("reverse_beep", reverse_beep, true),
    ("fire", fire, true),
    ("cue_gunfire", cue_gunfire, false),
    ("cue_engine", cue_engine, false),
    ("cue_footsteps", cue_footsteps, false),
];

type Stereo = fn(f64, &mut Mulberry32) -> Vec<Vec<f32>>;

/// Every ambient bed, by name: stereo, and always looping.
const BEDS: &[(&str, Stereo)] = &[
    ("countryside", countryside),
    ("distant_battle", distant_battle),
];

fn bed(name: &str) -> Option<Stereo> {
    BEDS.iter().find(|(n, _)| *n == name).map(|b| b.1)
}

/// Whether `name` is a synthesized sound.
pub fn exists(name: &str) -> bool {
    bed(name).is_some() || SOUNDS.iter().any(|(n, ..)| *n == name)
}

/// Every synthesized sound's name.
pub fn names() -> impl Iterator<Item = &'static str> {
    SOUNDS
        .iter()
        .map(|(n, ..)| *n)
        .chain(BEDS.iter().map(|b| b.0))
}

/// Whether synthesized sound `name` loops; None for an unknown name.
pub fn loops(name: &str) -> Option<bool> {
    if bed(name).is_some() {
        return Some(true);
    }
    SOUNDS.iter().find(|(n, ..)| *n == name).map(|s| s.2)
}

/// `name` at `sample_rate`: the same samples every time.
pub fn synthesize(name: &str, sample_rate: f64) -> Result<Synth, String> {
    let mut rng = Key::new().text(name).rng();
    if let Some(make) = bed(name) {
        return Ok(Synth {
            channels: make(sample_rate, &mut rng),
            looping: true,
        });
    }
    let (_, make, looping) = SOUNDS
        .iter()
        .find(|(n, ..)| *n == name)
        .ok_or_else(|| format!("no sound named {name}"))?;
    Ok(Synth {
        channels: vec![make(sample_rate, &mut rng)],
        looping: *looping,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The browser's table of synthesized sounds (`synthSounds.gen.ts`):
    /// the workbench and the catalog validate names and loops against it.
    #[test]
    fn the_browsers_sound_names_are_current() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../packages/battle-audio/src/synthSounds.gen.ts"
        );
        let mut text = String::from(
            "// Generated by `presentation::sound::synth` (`UPDATE_SOUNDS=1 cargo test -p presentation synth`).\n\
             /** Every synthesized sound, by name, and whether it loops. */\n\
             export const SOUNDS: Record<string, { loop: boolean }> = {\n",
        );
        for name in names() {
            let looping = loops(name).expect("a sound");
            text.push_str(&format!("  {name}: {{ loop: {looping} }},\n"));
        }
        text.push_str("};\n");
        if std::env::var_os("UPDATE_SOUNDS").is_some() {
            std::fs::write(path, &text).unwrap();
        }
        let current = std::fs::read_to_string(path).unwrap_or_default();
        assert!(
            current == text,
            "synthSounds.gen.ts is stale: rerun with UPDATE_SOUNDS=1"
        );
    }
}
