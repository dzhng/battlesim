//! Fixed-wing close air combat. This module owns every write to a jet's
//! `dogfight_target`; movement only observes the resulting link.

use contract::catalog::AltitudeLayer;
use contract::ids::UnitId;
use contract::random::{mix, Rng};
use contract::scenario::JetAir;

use crate::knowledge::Track;
use crate::math::{v2, v3, V3};
use crate::units::{Motion, Unit};

use super::jet::JetState;
use super::steer::Intent;

/// Whether a 3D separation has ended a close air engagement.
pub fn breaks(distance_3d: f64, air: &JetAir) -> bool {
    distance_3d > air.dogfight_break_m
}

/// Set a firing jet's link and let an unengaged jet answer it.
///
/// The check is deliberately on the altitude layer rather than the current
/// height: a helicopter is an air target for weapons, but never a dogfight
/// opponent (D29b).
pub(crate) fn on_fire(units: &mut [Unit], shooter: UnitId, target: UnitId) {
    if shooter == target
        || !units.get(shooter.0 as usize).is_some_and(is_jet)
        || !units.get(target.0 as usize).is_some_and(is_jet)
    {
        return;
    }
    units[shooter.0 as usize]
        .air
        .as_mut()
        .expect("a jet has air state")
        .dogfight_target = Some(target);
    if units[target.0 as usize]
        .air
        .as_ref()
        .is_some_and(|air| air.dogfight_target.is_none())
    {
        units[target.0 as usize]
            .air
            .as_mut()
            .expect("a jet has air state")
            .dogfight_target = Some(shooter);
    }
}

/// Install a target selected by an explicit air order.
pub(crate) fn set_target(unit: &mut Unit, target: UnitId) {
    if is_jet(unit) {
        unit.air
            .as_mut()
            .expect("a jet has air state")
            .dogfight_target = Some(target);
    }
}

/// Clear a link through the single dogfight owner.
pub(crate) fn clear(unit: &mut Unit) {
    if let Some(air) = unit.air.as_mut() {
        air.dogfight_target = None;
    }
}

/// Build one tick of lag pursuit for a linked jet.
///
/// The seed is already separated by jet id by the caller, so the sampled
/// height is stable for that jet across ticks while remaining battle seeded.
pub fn intent(own: &JetState, target_track: &Track, seed: u64, air: &JetAir) -> Intent {
    let mut rng = Rng::new(mix(seed));
    let band = air.dogfight_band_agl_m;
    let agl = band[0] + rng.unit() * (band[1] - band[0]);
    let target_velocity = v3(target_track.velocity.x, target_track.velocity.y, 0.0);
    let own_forward = v2(libm::cos(own.heading), libm::sin(own.heading));
    let offset = target_track.position.xy() - own.pos.xy();
    let distance = offset.length();
    let to_target = if distance > f64::EPSILON {
        offset * (1.0 / distance)
    } else {
        v2(0.0, 0.0)
    };
    let target_forward = v2(libm::cos(target_track.yaw), libm::sin(target_track.yaw));
    let head_on = own_forward.dot(to_target) > 0.5 && target_forward.dot(-to_target) > 0.5;
    let own_velocity = own_forward * own.speed_mps;
    let closing = (target_velocity.xy() - own_velocity).dot(offset) < 0.0;
    if distance <= air.dogfight_break_m * 0.5
        && head_on
        && (closing || target_track.velocity.length() <= 1e-6)
    {
        // A merge is a pass, not a charge through the opponent. Aim behind
        // our own nose for two seconds; once clear, ordinary lag pursuit
        // brings the jets round onto one another again.
        let point = (own.pos.xy() - own_forward * (own.speed_mps.max(1.0) * 2.0)).with_z(agl);
        return Intent::Pursue {
            point,
            vel: V3::default(),
            lag_s: 0.0,
        };
    }
    Intent::Pursue {
        point: v3(target_track.position.x, target_track.position.y, agl),
        vel: target_velocity,
        lag_s: 1.5,
    }
}

fn is_jet(unit: &Unit) -> bool {
    unit.alive()
        && matches!(unit.motion, Motion::Air(flight) if flight.layer == AltitudeLayer::HighAir)
}

/// True when a link's target is a live jet. Kept here so the battle's break
/// maintenance cannot accidentally turn a helicopter into a dogfight.
pub(crate) fn valid_target(units: &[Unit], target: UnitId) -> bool {
    units.get(target.0 as usize).is_some_and(is_jet)
}
