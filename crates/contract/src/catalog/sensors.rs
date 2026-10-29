//! Authored optical ranges and spotting.
use super::AltitudeLayer;
use crate::scenario::SightShape;
use serde::{Deserialize, Serialize};

/// A unit's one sight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sensors {
    /// Ground range in the open, before shape.
    pub ground_m: f64,
    /// Reach into low air; 0 is blind.
    pub low_air_m: f64,
    /// Reach into high air; 0 is blind.
    pub high_air_m: f64,
    /// Reach by direction as multipliers of `ground_m`; 1/1/1 sees evenly.
    pub sight_shape: SightShape,
    /// Cancels concealment above 1 and deepens it below 1; reach stays capped at full sight.
    #[serde(
        default = "ordinary_spotting",
        skip_serializing_if = "is_ordinary_spotting"
    )]
    pub spotting: f64,
    /// The turret mount the optics turn with; absent, they look along the hull.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<String>,
}

impl Sensors {
    /// The unit's authored reach into a target's altitude band.
    pub fn reach_m(&self, layer: AltitudeLayer) -> f64 {
        match layer {
            AltitudeLayer::Ground => self.ground_m,
            AltitudeLayer::LowAir => self.low_air_m,
            AltitudeLayer::HighAir => self.high_air_m,
        }
    }
}

fn ordinary_spotting() -> f64 {
    1.0
}

fn is_ordinary_spotting(value: &f64) -> bool {
    *value == 1.0
}
