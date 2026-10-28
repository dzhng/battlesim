//! The prepared sounds a battle plays, composed the same way by every
//! player: each recipe's alternatives (a recording, each with its
//! synthesized support under every shot of its burst, or the synthesis
//! alone), and each buffer's source calibration, the gain that brings its
//! loudest moment to the library's reference before any authored level.
use std::collections::HashMap;

use super::synth::{self, seamless, Synth};
use super::tables::{Cadence, Recipe, SoundTables};

/// Library reference, before recipe, weapon, distance and bus gains (LUFS).
pub const SOURCE_LOUDNESS_DB: f64 = -23.0;

/// One prepared alternative.
pub struct Buffer {
    pub channels: Vec<Vec<f32>>,
    pub sample_rate: f64,
    /// Its source calibration times the recipe's authored gain.
    pub normalization: f64,
}

impl Buffer {
    /// Seconds at rate 1.
    pub fn duration(&self) -> f64 {
        self.channels[0].len() as f64 / self.sample_rate
    }
}

type Biquad = [f64; 5];

/// BS.1770 K-weighting, as libebur128 parameterizes it by sample rate.
fn weighting(sample_rate: f64) -> [Biquad; 2] {
    use std::f64::consts::PI;
    let mut k = libm::tan(PI * 1681.974450955533 / sample_rate);
    let mut q = 0.7071752369554196;
    let high = libm::pow(10f64, 3.999843853973347 / 20.0);
    let middle = libm::pow(high, 0.4996667741545416);
    let mut d = 1.0 + k / q + k * k;
    let shelf = [
        (high + middle * k / q + k * k) / d,
        2.0 * (k * k - high) / d,
        (high - middle * k / q + k * k) / d,
        2.0 * (k * k - 1.0) / d,
        (1.0 - k / q + k * k) / d,
    ];
    k = libm::tan(PI * 38.13547087602444 / sample_rate);
    q = 0.5003270373238773;
    d = 1.0 + k / q + k * k;
    [
        shelf,
        [
            1.0,
            -2.0,
            1.0,
            2.0 * (k * k - 1.0) / d,
            (1.0 - k / q + k * k) / d,
        ],
    ]
}

/// The loudest K-weighted 100 ms window, in 25 ms hops. Short reports are
/// zero-padded, so silence or a long quiet tail cannot dilute them. A
/// short-effect reference, not an integrated programme loudness.
pub fn max_momentary_loudness(channels: &[Vec<f32>], sample_rate: f64) -> f64 {
    let window = (f64::round(sample_rate * 0.1) as usize).max(1);
    let hop = (f64::round(window as f64 / 4.0) as usize).max(1);
    let frames = channels[0].len() + window;
    let mut energy = vec![0.0f64; frames];
    let [shelf, high] = weighting(sample_rate);
    for samples in channels {
        let (mut s1, mut s2, mut h1, mut h2) = (0.0, 0.0, 0.0, 0.0);
        for (frame, e) in energy.iter_mut().enumerate() {
            let input = samples.get(frame).map_or(0.0, |v| f64::from(*v));
            let shelved = shelf[0] * input + s1;
            s1 = shelf[1] * input - shelf[3] * shelved + s2;
            s2 = shelf[2] * input - shelf[4] * shelved;
            let weighted = high[0] * shelved + h1;
            h1 = high[1] * shelved - high[3] * weighted + h2;
            h2 = high[2] * shelved - high[4] * weighted;
            *e += weighted * weighted;
        }
    }
    let (mut sum, mut maximum) = (0.0f64, 0.0f64);
    for frame in 0..frames {
        sum += energy[frame];
        if frame >= window {
            sum -= energy[frame - window];
        }
        if frame + 1 >= window && (frame + 1 - window).is_multiple_of(hop) {
            maximum = maximum.max(sum / window as f64);
        }
    }
    if maximum > 0.0 {
        -0.691 + 10.0 * libm::log10(maximum)
    } else {
        f64::NEG_INFINITY
    }
}

/// The gain that brings `channels` to the library's reference, times the
/// authored gain; silence (or an authored zero) stays as it is.
pub fn source_normalization(channels: &[Vec<f32>], sample_rate: f64, authored_gain: f64) -> f64 {
    let loudness = max_momentary_loudness(channels, sample_rate);
    if loudness < -90.0 || authored_gain == 0.0 {
        1.0
    } else {
        libm::pow(10f64, (SOURCE_LOUDNESS_DB - loudness) / 20.0) * authored_gain
    }
}

/// Every prepared sound of a battle, at one sample rate.
pub struct Bank {
    pub sample_rate: f64,
    synths: HashMap<String, Synth>,
    pub sounds: HashMap<String, Vec<Buffer>>,
}

impl Bank {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            synths: HashMap::new(),
            sounds: HashMap::new(),
        }
    }

    fn synth(&mut self, name: &str) -> Result<&Synth, String> {
        if !self.synths.contains_key(name) {
            let s = synth::synthesize(name, self.sample_rate)?;
            self.synths.insert(name.to_owned(), s);
        }
        Ok(&self.synths[name])
    }

    /// Prepares every sound the battle plays, reading each recording through
    /// `clip` (its channels at this bank's rate).
    pub fn prepare(
        &mut self,
        tables: &SoundTables,
        mut clip: impl FnMut(&str) -> Result<Vec<Vec<f32>>, String>,
    ) -> Result<(), String> {
        let mut recordings: HashMap<String, Vec<Vec<f32>>> = HashMap::new();
        for name in tables.battle_sounds() {
            let recipe = &tables.catalog.sounds[&name];
            for id in &recipe.clips {
                if !recordings.contains_key(id) {
                    recordings.insert(id.clone(), clip(id)?);
                }
            }
            let cores: Vec<Core> = recipe
                .clips
                .iter()
                .map(|id| (&recordings[id][..], tables.catalog.clips[id].cadence()))
                .collect();
            let buffers = self.recipe(recipe, &cores)?;
            self.sounds.insert(name, buffers);
        }
        Ok(())
    }

    /// `recipe`'s alternatives from its recordings `cores` (at this bank's
    /// rate) and its synthesized support.
    pub fn recipe(&mut self, recipe: &Recipe, cores: &[Core]) -> Result<Vec<Buffer>, String> {
        let rate = self.sample_rate;
        let support = match &recipe.synth {
            Some(s) => Some(&self.synth(s)?.channels),
            None => None,
        };
        Ok(compose(recipe, cores, support.map(|s| &s[..]), rate))
    }

    /// Each prepared sound's alternatives' lengths at rate 1, seconds.
    pub fn durations(&self) -> HashMap<String, Vec<f64>> {
        self.sounds
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().map(Buffer::duration).collect()))
            .collect()
    }
}

/// A recording's channels, and the shots its burst sounds.
pub type Core<'a> = (&'a [Vec<f32>], Cadence);

/// A recipe's alternatives: each recording with the synthesized support
/// under each of its shots (once for a single report), or the support alone;
/// a loop repeats each layer at its own period and softens the seam.
pub fn compose(
    recipe: &Recipe,
    cores: &[Core],
    support: Option<&[Vec<f32>]>,
    sample_rate: f64,
) -> Vec<Buffer> {
    let authored = recipe.gain
        * if cores.is_empty() {
            recipe.synth_gain
        } else {
            1.0
        };
    let alternatives: Vec<Option<&Core>> = if cores.is_empty() {
        vec![None]
    } else {
        cores.iter().map(Some).collect()
    };
    alternatives
        .into_iter()
        .map(|core| {
            let channels = mix(recipe, core, support, sample_rate);
            let normalization = source_normalization(&channels, sample_rate, authored);
            Buffer {
                channels,
                sample_rate,
                normalization,
            }
        })
        .collect()
}

fn mix(
    recipe: &Recipe,
    core: Option<&Core>,
    support: Option<&[Vec<f32>]>,
    sample_rate: f64,
) -> Vec<Vec<f32>> {
    // Baseline synthesis keeps its original samples.
    if let (None, Some(s)) = (core, support) {
        if recipe.gain == 1.0 && recipe.synth_gain == 1.0 {
            return s.to_vec();
        }
    }
    let cadence = core.map_or(
        Cadence {
            shots: 1,
            interval_s: 0.0,
        },
        |c| c.1,
    );
    let shots: Vec<usize> = (0..cadence.shots)
        .map(|k| f64::round(k as f64 * cadence.interval_s * sample_rate) as usize)
        .collect();
    let core = core.map(|c| c.0);
    let width = core
        .map_or(0, <[_]>::len)
        .max(support.map_or(0, <[_]>::len));
    let core_len = core.map_or(0, |c| c[0].len());
    let support_len = support.map_or(0, |s| s[0].len());
    let frames = core_len.max(shots.last().expect("one shot") + support_len);
    let (gain, synth_gain) = (recipe.gain, recipe.synth_gain);
    (0..width)
        .map(|channel| {
            let recorded = core.map(|c| &c[channel % c.len()]);
            let synth = support.map(|s| &s[channel % s.len()]);
            match (recorded, synth) {
                (Some(r), Some(s)) if recipe.looping && r.len() != s.len() => {
                    // Repeat each layer at its own period, then soften the seam.
                    let fade = frames.min(f64::round(0.05 * sample_rate) as usize);
                    seamless(frames, fade, |n| {
                        (0..n)
                            .map(|f| {
                                let v = f64::from(r[f % r.len()])
                                    + synth_gain * f64::from(s[f % s.len()]);
                                (gain * v) as f32
                            })
                            .collect()
                    })
                }
                _ => {
                    let mut out = vec![0.0f32; frames];
                    if let Some(r) = recorded {
                        out[..r.len()].copy_from_slice(r);
                    }
                    if let Some(s) = synth {
                        for &at in &shots {
                            for (f, v) in s.iter().enumerate() {
                                let o = &mut out[at + f];
                                *o = (f64::from(*o) + synth_gain * f64::from(*v)) as f32;
                            }
                        }
                    }
                    for o in &mut out {
                        *o = (f64::from(*o) * gain) as f32;
                    }
                    out
                }
            }
        })
        .collect()
}
