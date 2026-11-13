//! Which clip a moving soldier plays: his posture's gait, in the way he
//! goes relative to his facing.
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use super::super::delta_angle;

/// Each posture's gait (`INFANTRY_MOVES`), and its clip for each way he
/// goes relative to his facing (`INFANTRY_WAYS`: ahead, left, back, right).
pub(super) const GAITS: [&str; 3] = ["walk_aim", "crouch", "crawl"];
pub(super) const MOVES: [[&str; 4]; 3] = [
    ["walk_aim_f", "walk_aim_l", "walk_aim_b", "walk_aim_r"],
    ["crouch_f", "crouch_l", "crouch_b", "crouch_r"],
    ["crawl_f", "crawl_l", "crawl_b", "crawl_r"],
];
/// The ways' bearings from his facing, counter-clockwise.
const WAY_BEARINGS: [f64; 4] = [0.0, FRAC_PI_2, PI, -FRAC_PI_2];
const WAY_NAMES: [&str; 4] = ["f", "l", "b", "r"];
/// How far past the halfway bearing between two ways he keeps the way he is
/// going, so a diagonal does not flicker between two clips, radians.
const WAY_HOLD_RAD: f64 = 0.2;

/// The way (an index of `WAY_BEARINGS`) he goes at bearing `rel` from his
/// facing: the nearest, unless `current` is already a way of `gait` and
/// still near enough.
pub(super) fn way_of(rel: f64, current: &str, gait: &str) -> usize {
    let held = current.strip_prefix(gait).and_then(|w| w.strip_prefix('_'));
    if let Some(k) = WAY_NAMES.iter().position(|&w| Some(w) == held) {
        if delta_angle(WAY_BEARINGS[k], rel).abs() <= FRAC_PI_4 + WAY_HOLD_RAD {
            return k;
        }
    }
    let mut best = 0;
    let mut off = f64::INFINITY;
    for (k, &at) in WAY_BEARINGS.iter().enumerate() {
        let d = delta_angle(at, rel).abs();
        if d < off {
            (best, off) = (k, d);
        }
    }
    best
}

/// The gait a clip moves in, or None for any other clip.
pub(super) fn gait_of_clip(clip: &str) -> Option<&str> {
    let gait = clip.rfind('_').map_or("", |cut| &clip[..cut]);
    GAITS.contains(&gait).then_some(gait)
}
