//! A smoke screen: a stationary circular cloud a unit pops (`specs/smoke-screen`).
//!
//! This is the one definition of a cloud's state over time and of the smoke
//! a sight line crosses. The simulation's sensing and fog sweep, the fog's
//! upload, the drawn cloud and its sound all read it. The one deliberate copy
//! is the GPU fog's WGSL `fogSmokeStep`, which can't call Rust; the
//! renderer-core probe parity test (`tests/probes.rs`) holds it to this law.
//!
//! A cloud blooms to its full radius in `bloom_steps` equal steps, holds, then
//! thins in `fade_steps` steps over its last `fade_s`, scaling its depth per
//! metre and its concealment shortfall together. Stepping (not a smooth ramp)
//! lets every reader agree on the same state and caps how often the GPU fog
//! rebuilds. Smoke counts toward a sight line's foliage depth where the line
//! runs inside the circle and below the cloud's top, measured from the ground
//! under it (the forest canopy's rule); overlapping clouds add.
use crate::forest::FOLIAGE_SAMPLE_M;
use crate::ids::Tick;
use serde::{Deserialize, Serialize};

/// The cloud a carrier pops: authored on its smoke capability.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmokeCloudSpec {
    pub radius_m: f64,
    pub height_m: f64,
    /// From the pop to gone, bloom and fade included.
    pub life_s: f64,
    pub bloom_s: f64,
    pub fade_s: f64,
    pub bloom_steps: u8,
    pub fade_steps: u8,
    /// Foliage depth a metre of sight line inside the full cloud adds.
    pub depth_per_m: f64,
    /// The observer's range factor against a target standing in the full cloud.
    pub concealment: f64,
    /// Names the presentation's look (and sound) for this cloud.
    pub look: String,
}

impl SmokeCloudSpec {
    /// Whether this can be a cloud; a refusal names the field at fault.
    pub fn validate(&self) -> Result<(), String> {
        let positive = [
            ("radius_m", self.radius_m),
            ("height_m", self.height_m),
            ("life_s", self.life_s),
            ("bloom_s", self.bloom_s),
            ("fade_s", self.fade_s),
            ("depth_per_m", self.depth_per_m),
        ];
        if let Some((name, _)) = positive.iter().find(|(_, v)| !(v.is_finite() && *v > 0.0)) {
            return Err(format!("{name} must be positive"));
        }
        if self.bloom_steps == 0 || self.fade_steps == 0 {
            return Err("bloom_steps and fade_steps must be at least 1".into());
        }
        if self.bloom_s + self.fade_s > self.life_s {
            return Err("bloom_s and fade_s must fit within life_s".into());
        }
        if !(self.concealment >= 0.0 && self.concealment < 1.0) {
            return Err("concealment must be in [0, 1)".into());
        }
        if self.look.is_empty() {
            return Err("look must name a look".into());
        }
        Ok(())
    }
}

/// A popped cloud: where, when, and what it was popped as.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SmokeCloud {
    pub id: u32,
    /// Its centre on the ground where it was popped.
    pub centre: [f64; 3],
    pub popped: Tick,
    pub spec: SmokeCloudSpec,
}

/// A cloud at one tick: everything a sight line or a concealment test reads.
#[derive(Clone, Debug, PartialEq)]
pub struct CloudState {
    /// Counts up through bloom, full and fade; one phase is one state.
    pub phase: u8,
    pub centre: [f64; 2],
    pub radius_m: f64,
    pub height_m: f64,
    pub depth_per_m: f64,
    pub concealment: f64,
}

/// The cloud at `tick`, or `None` before its pop and once its life is over.
pub fn state(c: &SmokeCloud, tick: Tick, tick_hz: u32) -> Option<CloudState> {
    let s = &c.spec;
    let age = tick.checked_sub(c.popped)? as f64 / tick_hz as f64;
    if age >= s.life_s {
        return None;
    }
    let (bloom, fade) = (s.bloom_steps as f64, s.fade_steps as f64);
    let (phase, grown, strength) = if age < s.bloom_s {
        let k = ((age / s.bloom_s * bloom).floor()).min(bloom - 1.0);
        (k as u8, (k + 1.0) / bloom, 1.0)
    } else if age < s.life_s - s.fade_s {
        (s.bloom_steps, 1.0, 1.0)
    } else {
        let into = age - (s.life_s - s.fade_s);
        let j = ((into / s.fade_s * fade).floor()).min(fade - 1.0) + 1.0;
        (s.bloom_steps + j as u8, 1.0, 1.0 - j / (fade + 1.0))
    };
    Some(CloudState {
        phase,
        centre: [c.centre[0], c.centre[1]],
        radius_m: s.radius_m * grown,
        height_m: s.height_m,
        depth_per_m: s.depth_per_m * strength,
        concealment: 1.0 - (1.0 - s.concealment) * strength,
    })
}

/// The span `[t0, t1]` of segment a→b (as fractions of it) inside the circle,
/// in plan view; `None` when it misses.
fn circle_span(centre: [f64; 2], r: f64, a: [f64; 2], b: [f64; 2]) -> Option<(f64, f64)> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (fx, fy) = (a[0] - centre[0], a[1] - centre[1]);
    let aa = dx * dx + dy * dy;
    if aa <= 0.0 {
        return None;
    }
    let bb = 2.0 * (fx * dx + fy * dy);
    let cc = fx * fx + fy * fy - r * r;
    let disc = bb * bb - 4.0 * aa * cc;
    if disc <= 0.0 {
        return None;
    }
    let root = disc.sqrt();
    let t0 = ((-bb - root) / (2.0 * aa)).max(0.0);
    let t1 = ((-bb + root) / (2.0 * aa)).min(1.0);
    (t1 > t0).then_some((t0, t1))
}

/// The exact length of segment a→b inside the circle, in plan view.
pub fn circle_chord(centre: [f64; 2], r: f64, a: [f64; 2], b: [f64; 2]) -> f64 {
    circle_span(centre, r, a, b)
        .map_or(0.0, |(t0, t1)| (t1 - t0) * length(b[0] - a[0], b[1] - a[1]))
}

/// The smoke depth of one piece a→b of a sight line at height `sight_z` over
/// ground `ground_z`: each cloud whose top is above the piece adds its chord
/// times its depth per metre.
pub fn piece_depth(
    clouds: &[CloudState],
    a: [f64; 2],
    b: [f64; 2],
    sight_z: f64,
    ground_z: f64,
) -> f64 {
    clouds
        .iter()
        .filter(|c| sight_z < ground_z + c.height_m)
        .map(|c| circle_chord(c.centre, c.radius_m, a, b) * c.depth_per_m)
        .sum()
}

/// The smoke depth of the sight line a→b: within each cloud's span, pieces of
/// at most [`FOLIAGE_SAMPLE_M`] each count where their middle is below the
/// cloud's top over `ground` there (none where `ground` has no height).
pub fn segment_depth(
    clouds: &[CloudState],
    a: [f64; 3],
    b: [f64; 3],
    ground: impl Fn(f64, f64) -> Option<f64>,
) -> f64 {
    let len = length(b[0] - a[0], b[1] - a[1]);
    let at = |t: f64| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
    let mut total = 0.0;
    for c in clouds {
        let Some((t0, t1)) = circle_span(c.centre, c.radius_m, [a[0], a[1]], [b[0], b[1]]) else {
            continue;
        };
        let n = ((t1 - t0) * len / FOLIAGE_SAMPLE_M).ceil().max(1.0) as usize;
        for k in 0..n {
            let (u0, u1) = (
                t0 + (t1 - t0) * k as f64 / n as f64,
                t0 + (t1 - t0) * (k + 1) as f64 / n as f64,
            );
            let mid = (u0 + u1) / 2.0;
            let [x, y] = at(mid);
            let z = a[2] + (b[2] - a[2]) * mid;
            if let Some(g) = ground(x, y) {
                total += piece_depth(std::slice::from_ref(c), at(u0), at(u1), z, g);
            }
        }
    }
    total
}

/// The observer's range factor against a target standing at (x, y): the
/// strongest cloud covering it, or 1 outside every cloud.
pub fn concealment_at(clouds: &[CloudState], x: f64, y: f64) -> f64 {
    clouds
        .iter()
        .filter(|c| length(x - c.centre[0], y - c.centre[1]) <= c.radius_m)
        .map(|c| c.concealment)
        .fold(1.0, f64::min)
}

/// A plan-view length (`f64::hypot` is barred for libm; sqrt is exact).
fn length(dx: f64, dy: f64) -> f64 {
    (dx * dx + dy * dy).sqrt()
}
