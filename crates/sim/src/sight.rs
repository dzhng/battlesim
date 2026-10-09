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
use contract::scenario::{Rules, SightShape};

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
    let c = libm::cos(off);
    let end = if c >= 0.0 { shape.front } else { shape.rear };
    shape.side * (1.0 - c * c) + end * c * c
}

/// Where `unit` looks now: along the turret mount its optics sit on, or
/// along its hull (an even 360° sight makes the choice moot).
pub fn forward(unit: &Unit, arsenal: &Arsenal) -> f64 {
    match arsenal.optics(unit.kind) {
        Some(m) => unit.mounts[m].bearing,
        None => unit.yaw,
    }
}

/// Take this tick's forward snapshot for every unit, before any fire.
pub fn snapshot(units: &mut [Unit], arsenal: &Arsenal) {
    for unit in units {
        unit.sight_forward = forward(unit, arsenal);
    }
}

/// `unit`'s sight this tick, from its snapshot.
pub fn of(unit: &Unit, rules: &Rules) -> Sight {
    let sensors = &unit.unit_type(rules).sensors;
    Sight {
        forward: unit.sight_forward,
        shape: sensors.sight_shape,
        range: sensors.ground_m,
    }
}
