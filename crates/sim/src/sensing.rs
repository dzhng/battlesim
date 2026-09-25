//! Optical sensing: which enemies each unit's own eyes identify this tick.
//! Rays run from the observer's eye to the target's visibility samples (a
//! vehicle's hull centre and top, each living soldier). Solid geometry blocks;
//! foliage attenuates reach continuously and blocks outright past a full run;
//! a target's own forest depth conceals it by class.
use contract::ids::{Side, UnitId};
use contract::scenario::{Rules, SensorRules, UnitKind};

use crate::math::V3;
use crate::units::Unit;
use crate::world::WorldGeometry;

/// Height of an infantry visibility sample above the soldier's feet.
const SOLDIER_SAMPLE_M: f64 = 1.0;

/// One observer identifying one target this tick.
#[derive(Clone, Debug, PartialEq)]
pub struct Sighting {
    pub observer: UnitId,
    pub target: UnitId,
    /// Indices of the target's members seen (empty for vehicles).
    pub members: Vec<usize>,
}

pub fn ground_range(kind: UnitKind, s: &SensorRules) -> f64 {
    match kind {
        UnitKind::Recon => s.recon_ground_m,
        UnitKind::Rifle | UnitKind::At => s.infantry_ground_m,
        UnitKind::Tank => s.tank_ground_m,
        UnitKind::Supply => s.supply_ground_m,
    }
}

pub fn eye(unit: &Unit, rules: &Rules) -> V3 {
    let b = &rules.bodies;
    let h = match unit.kind {
        UnitKind::Tank => b.tank_eye_m,
        UnitKind::Supply => b.supply_eye_m,
        _ => b.infantry_eye_m,
    };
    unit.position + crate::math::v3(0.0, 0.0, h)
}

/// Where a unit's sight starts: its eye, or in a building each occupied
/// perimeter slot's eye. There is no extra all-round roof sensor.
pub fn eyes(unit: &Unit, rules: &Rules) -> Vec<V3> {
    if !unit.garrisoned() {
        return vec![eye(unit, rules)];
    }
    unit.member_positions()
        .map(|p| p + crate::math::v3(0.0, 0.0, rules.bodies.infantry_eye_m))
        .collect()
}

/// Continuous concealment strength in [0, 1] from forest depth (contracts.md).
pub fn concealment_strength(infantry: bool, depth: Option<f64>, s: &SensorRules) -> f64 {
    let Some(d) = depth else { return 0.0 };
    let v = if infantry {
        s.infantry_concealment_edge_strength + d / s.infantry_concealment_ramp_m
    } else {
        (d - s.vehicle_concealment_depth_m) / s.vehicle_concealment_ramp_m
    };
    v.clamp(0.0, 1.0)
}

/// Detection-range multiplier for a target standing at `at`: 1 in the open,
/// falling toward its class's forest multiplier with concealment strength.
pub fn concealment_multiplier(
    infantry: bool,
    world: &WorldGeometry,
    at: V3,
    s: &SensorRules,
) -> f64 {
    let strength = concealment_strength(infantry, world.forest_depth(at.x, at.y), s);
    let floor = if infantry {
        s.infantry_forest_range_multiplier
    } else {
        s.vehicle_forest_range_multiplier
    };
    1.0 + (floor - 1.0) * strength
}

/// Whether `eye` identifies a sample at `target` with base range `range`.
pub fn sees_point(
    world: &WorldGeometry,
    eye: V3,
    target: V3,
    range: f64,
    concealment: f64,
    s: &SensorRules,
) -> bool {
    let distance = (target - eye).length();
    if distance > range * concealment {
        return false; // cheap reject before any ray
    }
    let foliage = world.forest_path_length(eye, target);
    if foliage >= s.forest_full_block_m {
        return false;
    }
    let reach = range * concealment * (-foliage / s.forest_attenuation_m).exp();
    distance <= reach && world.segment_clear(eye, target)
}

/// Visibility samples of a unit, with the member index they belong to.
fn samples(unit: &Unit) -> Vec<(Option<usize>, V3)> {
    match unit.hull {
        Some(h) => vec![
            (None, unit.position + crate::math::v3(0.0, 0.0, h.z)),
            (None, unit.position + crate::math::v3(0.0, 0.0, 2.0 * h.z)),
        ],
        None => unit
            .members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive())
            .map(|(k, _)| {
                (
                    Some(k),
                    unit.member_position(k) + crate::math::v3(0.0, 0.0, SOLDIER_SAMPLE_M),
                )
            })
            .collect(),
    }
}

/// Detection multiplier for a target sample: the strongest concealment of
/// its forest ground and, for a garrisoned squad, its building.
fn target_concealment(world: &WorldGeometry, target: &Unit, at: V3, rules: &Rules) -> f64 {
    let s = &rules.sensors;
    let shelter = crate::garrison::shelter(target, rules);
    concealment_multiplier(target.hull.is_none(), world, at, s)
        .min(1.0 + (s.building_range_multiplier - 1.0) * shelter)
}

/// Every sighting by `side`'s units this tick, in observer then target order.
/// Sensor rules the geometry relies on (the fog sweep stops a ray once its
/// reach has shrunk behind it).
pub fn validate(s: &SensorRules) {
    assert!(
        s.forest_attenuation_m > 0.0,
        "sensors.forest_attenuation_m must be positive"
    );
}

pub fn evaluate(world: &WorldGeometry, units: &[Unit], rules: &Rules, side: Side) -> Vec<Sighting> {
    let s = &rules.sensors;
    let mut out = Vec::new();
    // The living enemy, once: fallen squads stay in the list all battle.
    let targets: Vec<&Unit> = units
        .iter()
        .filter(|u| u.side != side && u.alive())
        .collect();
    for observer in units.iter().filter(|u| u.side == side && u.alive()) {
        let from = eyes(observer, rules);
        let range = ground_range(observer.kind, s);
        for &target in &targets {
            let spread = observer.footprint_radius() + target.footprint_radius();
            if (target.position - observer.position).length() > range + spread {
                continue;
            }
            let mut seen = Vec::new();
            let mut any = false;
            for (member, at) in samples(target) {
                let concealment = target_concealment(world, target, at, rules);
                if from
                    .iter()
                    .any(|&eye| sees_point(world, eye, at, range, concealment, s))
                {
                    any = true;
                    if let Some(k) = member {
                        seen.push(k);
                    } else {
                        break; // one hull sample is enough
                    }
                }
            }
            if any {
                out.push(Sighting {
                    observer: observer.id,
                    target: target.id,
                    members: seen,
                });
            }
        }
    }
    out
}
