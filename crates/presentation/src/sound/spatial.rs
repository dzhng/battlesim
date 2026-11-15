//! Shared listener direction and distance attenuation for native and browser sound.
use super::tables::{ListenerPlacement, SoundTables};
use crate::world::Vec3;
use serde::{Deserialize, Serialize};

/// Stereo position of a direction sector for a listener looking along
/// `forward`: 0 east, counter-clockwise in 45° steps.
pub fn sector_pan(sector: f64, forward: Vec3) -> f64 {
    let a = sector * std::f64::consts::PI / 4.0;
    let len = libm::hypot(forward[0], forward[1]);
    let len = if len == 0.0 { 1.0 } else { len };
    // Screen-right: forward × up.
    let (rx, ry) = (forward[1] / len, -forward[0] / len);
    (libm::cos(a) * rx + libm::sin(a) * ry).clamp(-1.0, 1.0)
}

/// The share of a sound's gain heard `d` metres off: full within `ref_m`,
/// then inverse distance easing down to `floor`, and nothing past `max_m`.
pub fn distance_gain(tables: &SoundTables, d: f64) -> f64 {
    let p = tables.audio.distance;
    if d > p.max_m {
        return 0.0;
    }
    let inverse = p.ref_m / (p.ref_m + p.rolloff * (d - p.ref_m).max(0.0));
    p.floor + (1.0 - p.floor) * inverse
}

/// The listener: where it stands and which way it looks (level).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Listener {
    pub position: Vec3,
    pub forward: Vec3,
}

impl Listener {
    /// Where the listener stands for a camera at `eye` looking at `target`
    /// with `up`, as `placement` puts it, facing the way the camera looks;
    /// looking straight down, the way its top points.
    pub fn of_camera(eye: Vec3, target: Vec3, up: [f32; 3], placement: ListenerPlacement) -> Self {
        let span = crate::world::length([0, 1, 2].map(|i| target[i] - eye[i]));
        let reach = placement.reach_heights * (eye[2] - target[2]).abs();
        let along = ((1.0 - placement.eye_share) * span).min(reach);
        let t = if span > 0.0 { along / span } else { 0.0 };
        let position = [0, 1, 2].map(|i| eye[i] + t * (target[i] - eye[i]));
        let mut forward = [target[0] - eye[0], target[1] - eye[1], 0.0];
        if (forward[0] * forward[0] + forward[1] * forward[1]).sqrt() < 1e-6 {
            forward = [f64::from(up[0]), f64::from(up[1]), 0.0];
        }
        let squared = forward[0] * forward[0] + forward[1] * forward[1];
        if squared > 0.0 {
            let inverse = 1.0 / squared.sqrt();
            forward = [forward[0] * inverse, forward[1] * inverse, 0.0];
        }
        Self { position, forward }
    }
}
