//! Authored flight data: the fixture's `physics` flight bounds and the flight
//! fields of a weapon row. Other fields in those sections belong to other
//! owners and are ignored here.
use serde::{Deserialize, Serialize};

/// Flight bounds from the fixture's `physics` section.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlightRules {
    pub gravity_mps2: f64,
    /// Largest allowed distance between a flown chord and the true curve.
    pub curve_chord_error_m: f64,
    /// Declared bound on chords per tick; authoring that needs more is invalid.
    pub max_subsegments_per_tick: u32,
    /// Lifetime of an unguided round, and the bound on any authored one.
    pub max_unguided_lifetime_s: f64,
}

/// Which ballistic arcs a weapon may choose (P03).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trajectory {
    /// Normal low arc only.
    #[default]
    Direct,
    /// A dedicated indirect-fire weapon: prefers the high arc.
    Indirect,
}

/// The flight fields of a fixture weapon row.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeaponBallistics {
    pub speed_mps: f64,
    /// One-axis angular standard deviation of launch spread, milliradians.
    pub scatter_mrad: f64,
    /// Path distance within which a passing round reports a near miss.
    pub suppression_radius_m: f64,
    /// Authored lifetime; unguided rounds without one use the physics bound.
    #[serde(default)]
    pub lifetime_s: Option<f64>,
    #[serde(default)]
    pub trajectory: Trajectory,
    /// A guided round: it flies at constant speed without gravity, steering
    /// toward its commanded point no faster than this (P05, P06).
    #[serde(default)]
    pub turn_deg_s: Option<f64>,
}
