//! Shared listener direction and distance attenuation for native and browser sound.
use super::tables::{Distance, SoundTables};
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

/// How far `position` sounds from `listener`, the distance level, the air's
/// colour and a far recording all read. The camera is a directional mic: a
/// sound within `focus.inner_deg` of where it looks is heard as if
/// `focus.share` of its distance away, easing back to its whole distance by
/// `focus.outer_deg`, so what the player watches is louder and clearer than
/// what is off screen or behind them, though still farther than what is near.
pub fn heard_distance(tables: &SoundTables, listener: &Listener, position: &Vec3) -> f64 {
    let to = [0, 1, 2].map(|i| position[i] - listener.position[i]);
    let d = crate::world::length(to);
    let f = tables.audio.focus;
    let along = (0..3).map(|i| to[i] * listener.look[i]).sum::<f64>() / d.max(1e-9);
    let (inner, outer) = (
        libm::cos(f.inner_deg.to_radians()),
        libm::cos(f.outer_deg.to_radians()),
    );
    let t = ((along - outer) / (inner - outer)).clamp(0.0, 1.0);
    d * (1.0 - (1.0 - f.share) * t * t * (3.0 - 2.0 * t))
}

impl Distance {
    /// Distance attenuation for this source's authored reach.
    pub fn gain(&self, d: f64) -> f64 {
        if d > self.max_m {
            return 0.0;
        }
        let inverse = self.ref_m / (self.ref_m + self.rolloff * (d - self.ref_m).max(0.0));
        self.floor + (1.0 - self.floor) * inverse
    }
}

/// The listener: where it stands, which way it faces (level, for panning)
/// and which way it looks (its view, for its focus; none, all zero, hears
/// without one).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Listener {
    pub position: Vec3,
    pub forward: Vec3,
    #[serde(default)]
    pub look: Vec3,
}

impl Listener {
    /// At the origin facing north, with no view: before any camera.
    pub const START: Self = Self {
        position: [0.0; 3],
        forward: [0.0, 1.0, 0.0],
        look: [0.0; 3],
    };

    /// The listener for a camera at `eye` looking at `target` with `up`: at
    /// the eye, facing the way the camera looks; looking straight down, the
    /// way its top points.
    pub fn of_camera(eye: Vec3, target: Vec3, up: [f32; 3]) -> Self {
        let position = eye;
        let span = crate::world::length([0, 1, 2].map(|i| target[i] - eye[i]));
        let look = [0, 1, 2].map(|i| (target[i] - eye[i]) / span.max(1e-9));
        let mut forward = [target[0] - eye[0], target[1] - eye[1], 0.0];
        if (forward[0] * forward[0] + forward[1] * forward[1]).sqrt() < 1e-6 {
            forward = [f64::from(up[0]), f64::from(up[1]), 0.0];
        }
        let squared = forward[0] * forward[0] + forward[1] * forward[1];
        if squared > 0.0 {
            let inverse = 1.0 / squared.sqrt();
            forward = [forward[0] * inverse, forward[1] * inverse, 0.0];
        }
        Self {
            position,
            forward,
            look,
        }
    }
}
