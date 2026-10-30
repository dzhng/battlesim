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
    let locomotion = t.mobility.locomotion()?;
    let speed_factors = *m.speed_factors.of(locomotion);
    let vehicle = |drive| Mobility {
        off_road_mps,
        road_mps,
        locomotion,
        speed_factors,
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
            locomotion,
            speed_factors,
            half_width_m: rules.infantry_movement.path_clearance_m,
            class: MoverClass::Infantry,
            push: contract::scenario::PushClass::None,
            drive: None,
        },
        Moves::Tracked {
            turn_deg_s,
            reverse_speed_factor,
            ..
        } => vehicle(crate::navigation::Drive {
            tracked: true,
            turn_rad_s: turn_deg_s.to_radians(),
            radius_m: 0.0,
            reverse_speed_factor,
            feel: m.drive,
        }),
        Moves::Wheeled {
            turn_deg_s,
            turning_radius_m,
            reverse_speed_factor,
            ..
        } => vehicle(crate::navigation::Drive {
            tracked: false,
            turn_rad_s: turn_deg_s.to_radians(),
            radius_m: turning_radius_m,
            reverse_speed_factor,
            feel: m.drive,
        }),
    })
}
