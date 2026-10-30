//! Fixed-wing close air combat. This module owns every write to a jet's
//! `dogfight_target`; movement only observes the resulting link.

use contract::ids::UnitId;
use contract::random::{mix, Rng};
use contract::scenario::FixedWingFlight;

use crate::knowledge::Track;
use crate::math::{v2, V2};
use crate::units::Unit;

use super::fixed_wing::FixedWingState;
use super::steer::Intent;

/// Whether a 3D separation has ended a close air engagement.
pub fn breaks(distance_3d: f64, air: &FixedWingFlight) -> bool {
    distance_3d > air.dogfight_break_m
}

/// Whether `pursuer` may fly a dogfight against `target` (D29c): both fly
/// on wings, and the target is alive in the pursuer's own layer. Read from
/// the layers rather than the current heights; against a rotor aircraft in
/// any layer, anything in another layer, or the ground, a winged aircraft
/// attacks in passes, and a rotor aircraft never dogfights.
pub(crate) fn can_dogfight(pursuer: &Unit, target: &Unit) -> bool {
    pursuer.fixed_wing()
        && target.fixed_wing()
        && target.alive()
        && target.layer() == pursuer.layer()
}

/// Link a firing winged aircraft to its winged target and let the target
/// answer it if unengaged.
pub(crate) fn on_fire(units: &mut [Unit], shooter: UnitId, target: UnitId) {
    let (Some(s), Some(t)) = (units.get(shooter.0 as usize), units.get(target.0 as usize)) else {
        return;
    };
    if shooter == target || !s.alive() || !can_dogfight(s, t) {
        return;
    }
    let answers = can_dogfight(t, s);
    units[shooter.0 as usize]
        .air
        .as_mut()
        .expect("an aircraft's flight state")
        .dogfight_target = Some(target);
    if answers
        && units[target.0 as usize]
            .air
            .as_ref()
            .is_some_and(|air| air.dogfight_target.is_none())
    {
        units[target.0 as usize]
            .air
            .as_mut()
            .expect("an aircraft's flight state")
            .dogfight_target = Some(shooter);
    }
}

/// Install a target an explicit air order selected; its caller has judged
/// the pair (`can_dogfight`).
pub(crate) fn set_target(unit: &mut Unit, target: UnitId) {
    if unit.alive() && unit.fixed_wing() {
        unit.air
            .as_mut()
            .expect("an aircraft's flight state")
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
pub fn intent(
    own: &FixedWingState,
    target_track: &Track,
    seed: u64,
    air: &FixedWingFlight,
) -> Intent {
    let mut rng = Rng::new(mix(seed));
    let band = air.dogfight_band_agl_m;
    let agl = band[0] + rng.unit() * (band[1] - band[0]);
    let target_velocity = target_track.velocity;
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
    let closing = (target_velocity - own_velocity).dot(offset) < 0.0;
    if distance <= air.dogfight_break_m * 0.5
        && head_on
        && (closing || target_track.velocity.length() <= 1e-6)
    {
        // A merge is a pass, not a charge through the opponent. Aim behind
        // our own nose for two seconds; once clear, ordinary lag pursuit
        // brings the jets round onto one another again.
        return Intent::Pursue {
            point: own.pos.xy() - own_forward * (own.speed_mps.max(1.0) * 2.0),
            vel: V2::default(),
            lag_s: 0.0,
            agl,
        };
    }
    Intent::Pursue {
        point: target_track.position.xy(),
        vel: target_velocity,
        lag_s: 1.5,
        agl,
    }
}

#[cfg(test)]
mod tests {
    use super::can_dogfight;
    use crate::units::{Motion, Unit};
    use contract::catalog::AltitudeLayer;
    use contract::ids::UnitId;

    /// A dogfight forms only between two winged aircraft in one layer: the
    /// layer they share, not high air, decides, and a rotor target in any
    /// layer, the pursuer's own included, gets passes. Rows that fly no law
    /// yet (a winged aircraft in low air, a rotor in high air) stand in for
    /// the pairs the catalog doesn't field.
    #[test]
    fn only_two_winged_aircraft_in_one_layer_dogfight() {
        let setup = serde_json::from_value(serde_json::json!({
            "map": { "size": [2000, 2000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
            "rules": crate::fixtures::test_rules(),
            "units": [
                { "side": "blue", "kind": "test_jet", "position": [500, 500] },
                { "side": "red", "kind": "test_heli", "position": [900, 500] }
            ],
            "events": [], "scripts": []
        }))
        .unwrap();
        let battle = crate::battle::Battle::new(&setup, 1);
        let unit = |id| battle.unit(UnitId(id)).unwrap().clone();
        let (jet, heli) = (unit(0), unit(1));
        let mut low_jet = jet.clone();
        let Motion::Air(flight) = &mut low_jet.motion else {
            unreachable!("a jet flies")
        };
        flight.layer = AltitudeLayer::LowAir;
        let mut high_heli = heli.clone();
        let Motion::Air(flight) = &mut high_heli.motion else {
            unreachable!("a helicopter flies")
        };
        flight.layer = AltitudeLayer::HighAir;
        let pairs: [(&str, &Unit, &Unit, bool); 8] = [
            ("jet on jet", &jet, &jet, true),
            ("low-air jet on low-air jet", &low_jet, &low_jet, true),
            ("jet on helicopter", &jet, &heli, false),
            ("low-air jet on helicopter", &low_jet, &heli, false),
            (
                "jet on a rotor aircraft in high air",
                &jet,
                &high_heli,
                false,
            ),
            ("low-air jet on jet", &low_jet, &jet, false),
            ("helicopter on helicopter", &heli, &heli, false),
            ("helicopter on low-air jet", &heli, &low_jet, false),
        ];
        for (what, pursuer, target, expected) in pairs {
            assert_eq!(can_dogfight(pursuer, target), expected, "{what}");
        }
    }
}
