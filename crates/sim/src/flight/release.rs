//! Analytic release point for an unpowered round dropped by a moving carrier.

use crate::math::{V2, V3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Release {
    pub fall_s: f64,
    pub ahead_m: f64,
    pub at: V2,
}

pub fn release(carrier_pos: V3, carrier_vel: V3, gravity: V3, aim: V3) -> Release {
    let down = -gravity.z;
    assert!(down > 0.0 && down.is_finite());
    let height = carrier_pos.z - aim.z;
    let fall_s =
        (carrier_vel.z + (carrier_vel.z * carrier_vel.z + 2.0 * down * height).sqrt()) / down;
    let ahead = carrier_vel.xy() * fall_s;
    Release {
        fall_s,
        ahead_m: ahead.length(),
        at: aim.xy() - ahead,
    }
}
