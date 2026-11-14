//! The bodies every unit shares: the one soldier frame, and the sight
//! heights soldiers and vehicles see from.
use serde::{Deserialize, Serialize};

use crate::catalog::UnitType;

/// The one infantry body every soldier shares (navigation's clearance, cover,
/// the fit authority) and round flight (the fixture's `physics` section).
/// A vehicle's body is its type's hull.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyRules {
    pub soldier_radius_m: f64,
    pub soldier_height_m: f64,
    pub infantry_eye_m: f64,
    /// Every vehicle's sight height above its hull's base, aircraft too:
    /// one height, so sight never differs by model.
    pub vehicle_eye_m: f64,
    pub infantry_muzzle_m: f64,
    /// Where rounds aim on a soldier, above his feet.
    pub infantry_aim_m: f64,
    /// Nominal aim height as a fraction of a vehicle body's full height.
    pub vehicle_aim_height_fraction: f64,
    /// A soldier's middle, above his feet: where blast and near misses
    /// reach him.
    pub infantry_center_m: f64,
    /// Angular spread multiplier while the firing unit moves (W04).
    pub moving_scatter_multiplier: f64,
    /// Extra room a round's predicted path must keep from friendly vehicles (P11).
    pub friendly_prefire_margin_m: f64,
    #[serde(flatten)]
    pub flight: crate::ballistics::FlightRules,
}

impl BodyRules {
    /// Where `unit` sees from, above its base.
    pub fn eye_m(&self, unit: &UnitType) -> f64 {
        unit.eye_m(self.infantry_eye_m, self.vehicle_eye_m)
    }
}

impl UnitType {
    /// Its sight height above its base: `vehicle_m` with a hull, else
    /// `infantry_m` (`physics.vehicle_eye_m`, `infantry_eye_m`).
    pub fn eye_m(&self, infantry_m: f64, vehicle_m: f64) -> f64 {
        self.hull().map_or(infantry_m, |_| vehicle_m)
    }
}
