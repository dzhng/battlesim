//! The one sight shape (Q4+, Q5): how far a unit's own eyes reach in each
//! direction. Spotting (`sensing`), the fog sweep (`visibility`) and the
//! publication all read it from here.
//!
//! A unit sees its ground range times a multiplier of the angle off its
//! forward direction: `front` dead ahead, `side` abeam, `rear` astern, easing
//! between them as `side·sin² + (front or rear)·cos²`. Vehicles look along
//! the tank's turret or the truck's hull; infantry sees an even 360°.
//!
//! The forward direction is a snapshot taken before the tick's fire, so this
//! tick's spotting, sweep and publication agree on it.
use contract::scenario::{Rules, SensorRules, SightShape, UnitKind};

use crate::math::wrap_angle;
use crate::units::Unit;
use crate::weapons::Arsenal;

/// A unit's sight at one tick: where it looks, its shape and its base range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sight {
    pub forward: f64,
    pub shape: SightShape,
    pub range: f64,
}

impl Sight {
    /// The farthest this sight reaches in any direction.
    pub fn max_range(&self) -> f64 {
        self.range * self.shape.front
    }

    /// Reach toward world bearing `bearing`.
    pub fn range_at(&self, bearing: f64) -> f64 {
        self.range * multiplier(&self.shape, wrap_angle(bearing - self.forward))
    }

    /// The farthest reach toward any bearing within `half` radians of
    /// `bearing`: a conservative bound for culling whole targets.
    pub fn reach_within(&self, bearing: f64, half: f64) -> f64 {
        let off = (wrap_angle(bearing - self.forward).abs() - half).max(0.0);
        self.range * multiplier(&self.shape, off)
    }
}

/// The shape's multiplier `off` radians from forward.
pub fn multiplier(shape: &SightShape, off: f64) -> f64 {
    let c = off.cos();
    let end = if c >= 0.0 { shape.front } else { shape.rear };
    shape.side * (1.0 - c * c) + end * c * c
}

/// Ground range in the open for a unit of `kind`, before shape.
pub fn base_range(kind: UnitKind, s: &SensorRules) -> f64 {
    match kind {
        UnitKind::Recon => s.recon_ground_m,
        UnitKind::Rifle | UnitKind::At => s.infantry_ground_m,
        UnitKind::Tank => s.tank_ground_m,
        UnitKind::Supply => s.supply_ground_m,
        UnitKind::Jeep => s.jeep_ground_m,
    }
}

/// The sight shape of `kind`: the fixture's for vehicles, isotropic for infantry.
pub fn shape(kind: UnitKind, s: &SensorRules) -> SightShape {
    match kind {
        UnitKind::Rifle | UnitKind::Recon | UnitKind::At => SightShape::ISOTROPIC,
        UnitKind::Tank => s.sight_shape.tank,
        UnitKind::Supply => s.sight_shape.supply,
        UnitKind::Jeep => s.sight_shape.jeep,
    }
}

/// Where `unit` looks now: a tank along its turret, everything else along
/// its hull (infantry's isotropic sight makes the choice moot).
pub fn forward(unit: &Unit, arsenal: &Arsenal) -> f64 {
    match unit.kind {
        UnitKind::Tank => turret(unit, arsenal).bearing,
        _ => unit.yaw,
    }
}

fn turret<'a>(unit: &'a Unit, arsenal: &Arsenal) -> &'a crate::weapons::Mount {
    let specs = arsenal.specs(unit.kind);
    unit.mounts
        .iter()
        .find(|m| specs[m.spec].turret)
        .expect("a tank has a turret mount (checked at setup)")
}

/// Take this tick's forward snapshot for every unit, before any fire.
pub fn snapshot(units: &mut [Unit], arsenal: &Arsenal) {
    for unit in units {
        unit.sight_forward = forward(unit, arsenal);
    }
}

/// `unit`'s sight this tick, from its snapshot.
pub fn of(unit: &Unit, rules: &Rules) -> Sight {
    Sight {
        forward: unit.sight_forward,
        shape: shape(unit.kind, &rules.sensors),
        range: base_range(unit.kind, &rules.sensors),
    }
}

/// How far `unit` sees toward world bearing `bearing` this tick.
pub fn sight_range(unit: &Unit, rules: &Rules, bearing: f64) -> f64 {
    of(unit, rules).range_at(bearing)
}

/// Shapes the geometry relies on: positive and never rising away from the
/// front, so a target cull can bound a whole arc by its nearest-to-front edge.
pub fn validate(rules: &Rules, arsenal: &Arsenal) {
    let s = &rules.sensors.sight_shape;
    for (kind, shape) in [("tank", s.tank), ("supply", s.supply), ("jeep", s.jeep)] {
        assert!(
            0.0 < shape.rear && shape.rear <= shape.side && shape.side <= shape.front,
            "sensors.sight_shape.{kind} must have 0 < rear <= side <= front"
        );
    }
    assert!(
        arsenal.specs(UnitKind::Tank).iter().any(|m| m.turret),
        "a tank looks along its turret, so it needs a turret mount"
    );
}
