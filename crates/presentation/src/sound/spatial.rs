//! Shared listener direction and distance attenuation for native and browser sound.
use super::tables::SoundTables;
use crate::world::Vec3;

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
