//! The ambient beds: stereo, and always looping.
use libm::{floor, sin};

use super::*;

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

pub(super) type Stereo = fn(f64, &mut Mulberry32) -> Vec<Vec<f32>>;

/// Every ambient bed, by name: stereo, and always looping.
pub(super) const BEDS: &[(&str, Stereo)] = &[
    ("countryside", countryside),
    ("distant_battle", distant_battle),
];

pub(super) fn bed(name: &str) -> Option<Stereo> {
    BEDS.iter().find(|(n, _)| *n == name).map(|b| b.1)
}
