//! Analytic release point for an unpowered round dropped by a moving carrier.

use crate::math::{V2, V3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Release {
    pub fall_s: f64,
    pub ahead_m: f64,
    pub at: V2,
}

/// Where a round dropped now with the carrier's velocity must leave it to
/// fall onto `aim`, or `None` when no fall reaches the aim's height: the
/// aim is above the carrier and its climb can't carry a dropped round up
/// to it.
pub fn release(carrier_pos: V3, carrier_vel: V3, gravity: V3, aim: V3) -> Option<Release> {
    let down = -gravity.z;
    assert!(down > 0.0 && down.is_finite());
    let height = carrier_pos.z - aim.z;
    let discriminant = carrier_vel.z * carrier_vel.z + 2.0 * down * height;
    if discriminant < 0.0 {
        return None;
    }
    let fall_s = (carrier_vel.z + discriminant.sqrt()) / down;
    let ahead = carrier_vel.xy() * fall_s;
    (fall_s >= 0.0 && fall_s.is_finite()).then(|| Release {
        fall_s,
        ahead_m: ahead.length(),
        at: aim.xy() - ahead,
    })
}
