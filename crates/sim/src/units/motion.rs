//! How a unit type moves: over the ground by foot or drive, or through the
//! air.

use super::*;

/// How a unit moves: over the ground, planned on the navigation grid, or
/// through the air, where no ground rule reaches it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    Ground(Mobility),
    Air(Flight),
}

/// An aircraft's own numbers; its heights are the rules' `air` section.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    pub cruise_mps: f64,
    pub turn_rad_s: f64,
    pub climb_mps: f64,
    /// Authored altitude band copied into runtime state at spawn.
    pub layer: AltitudeLayer,
    /// How it flies: on rotors (`movement::air`) or wings (`air`).
    pub model: FlightModel,
    /// Its cruise height over the ground (`AirRules::cruise_agl_m`).
    pub cruise_agl_m: f64,
}

/// How a unit of type `t` moves.
pub fn motion(t: &UnitType, rules: &Rules) -> Motion {
    match t.mobility {
        Moves::Air {
            cruise_kmh,
            turn_deg_s,
            climb_mps,
            layer,
            flight,
            ..
        } => Motion::Air(Flight {
            cruise_mps: cruise_kmh / 3.6,
            turn_rad_s: turn_deg_s.to_radians(),
            climb_mps,
            layer,
            model: flight,
            cruise_agl_m: rules.air.cruise_agl_m(&t.mobility),
        }),
        _ => Motion::Ground(ground_mobility(t, rules).expect("a ground mover")),
    }
}

/// How a ground unit of type `t` moves: on foot, or by its drive (Q29, Q30);
/// `None` for an aircraft, which never touches ground navigation.
pub fn ground_mobility(t: &UnitType, rules: &Rules) -> Option<Mobility> {
    let m = &rules.movement;
    let (off_road_mps, road_mps) = t.mobility.speeds_mps();
    let vehicle = |drive: crate::navigation::Drive| Mobility {
        off_road_mps,
        road_mps,
        forest_multiplier: m.forest_vehicle_multiplier,
        swamp_multiplier: if drive.tracked {
            m.swamp.tracked
        } else {
            m.swamp.wheeled
        },
        half_width_m: t.hull().expect("a vehicle has a hull").half_extents_m[1],
        class: MoverClass::Vehicle,
        push: t.hull().expect("a vehicle has a hull").push_class,
        drive: Some(drive),
    };
    Some(match t.mobility {
        Moves::Air { .. } => return None,
        Moves::Foot { .. } => Mobility {
            off_road_mps,
            road_mps,
            forest_multiplier: m.forest_infantry_multiplier,
            swamp_multiplier: m.swamp.foot,
            half_width_m: rules.infantry_movement.path_clearance_m,
            class: MoverClass::Infantry,
            push: contract::scenario::PushClass::None,
            drive: None,
        },
        Moves::Tracked {
            turn_deg_s,
            reverse_fraction,
            ..
        } => vehicle(crate::navigation::Drive {
            tracked: true,
            turn_rad_s: turn_deg_s.to_radians(),
            radius_m: 0.0,
            reverse_fraction,
            feel: m.drive,
        }),
        Moves::Wheeled {
            turn_deg_s,
            turning_radius_m,
            reverse_fraction,
            ..
        } => vehicle(crate::navigation::Drive {
            tracked: false,
            turn_rad_s: turn_deg_s.to_radians(),
            radius_m: turning_radius_m,
            reverse_fraction,
            feel: m.drive,
        }),
    })
}

/// Movement rules the motion model and the route planner divide and ease
/// by. (Each unit type's own numbers are the catalog's checks, at load.)
pub fn validate(rules: &Rules) {
    let m = &rules.movement;
    for (name, v) in [
        ("forest_infantry_multiplier", m.forest_infantry_multiplier),
        ("forest_vehicle_multiplier", m.forest_vehicle_multiplier),
        ("swamp.foot", m.swamp.foot),
        ("swamp.wheeled", m.swamp.wheeled),
        ("swamp.tracked", m.swamp.tracked),
    ] {
        assert!(
            v > 0.0 && v <= 1.0,
            "movement.{name} must be in (0, 1]: no forest or swamp stops a mover dead"
        );
    }
    let d = &m.drive;
    assert!(
        d.acceleration_s.is_finite()
            && d.acceleration_s > 0.0
            && d.braking_s.is_finite()
            && d.braking_s > 0.0
            && d.wreck_stop_s.is_finite()
            && d.wreck_stop_s > 0.0,
        "movement.drive acceleration, braking and wreck stop times must be finite and positive"
    );
    for (name, v) in [
        ("turn_in_place_deg", d.turn_in_place_deg),
        ("abeam_m", d.abeam_m),
        ("abeam_deg", d.abeam_deg),
        ("turning_deg", d.turning_deg),
    ] {
        assert!(v > 0.0, "movement.drive.{name} must be positive");
    }
    assert!(
        d.min_leg_m >= 0.0 && d.circle_margin_m >= 0.0,
        "movement.drive.min_leg_m and circle_margin_m must not be negative"
    );
    assert!(
        d.turn_slow > 0.0 && d.turn_slow <= 1.0,
        "movement.drive.turn_slow must be in (0, 1]: a turning vehicle never stops"
    );
}
