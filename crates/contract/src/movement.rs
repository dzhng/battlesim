//! How ground units move over the map, whatever each one's own speeds: the
//! ground's multipliers on off-road speed, turret and bearing tolerances,
//! and how every vehicle drives.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementRules {
    pub forest_infantry_multiplier: f64,
    pub forest_vehicle_multiplier: f64,
    /// Off a road, a swamp's water multiplies a ground mover's speed by its
    /// drive's factor here, wherever it wades and however deep.
    pub swamp: SwampRules,
    pub turret_turn_deg_s: f64,
    pub bearing_tolerance_deg: f64,
    pub drive: DriveRules,
}

/// A swamp's slowdown by how a mover crosses ground: wheels bog worse
/// than tracks (decision 3). Each lies in (0, 1]: nothing stops dead.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwampRules {
    pub foot: f64,
    pub wheeled: f64,
    pub tracked: f64,
}

/// How every vehicle drives its route (Q29, Q30), whatever its own speeds
/// and turning.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriveRules {
    /// Seconds to accelerate from rest to the vehicle's full road speed.
    pub acceleration_s: f64,
    /// Seconds to brake from full road speed to rest.
    pub braking_s: f64,
    /// Seconds a destroyed vehicle takes to come to rest from full road
    /// speed: its tracks or wheels lock, so it stops harder than it brakes.
    pub wreck_stop_s: f64,
    /// Tracks turn in place beyond this heading error.
    pub turn_in_place_deg: f64,
    /// A wheeled vehicle reaches a waypoint it passes abeam within this.
    pub abeam_m: f64,
    /// Headings further off than this count as abeam or behind.
    pub abeam_deg: f64,
    /// A reversing leg of a three-point turn drives at least this far.
    pub min_leg_m: f64,
    /// The waypoint must lie this far outside the turning circle to end a leg.
    pub circle_margin_m: f64,
    /// Beyond this heading error a wheeled turn counts as progress rather
    /// than a stall while it swings away from the waypoint.
    pub turning_deg: f64,
    /// Maximum speed fraction during a manoeuvre; the turning radius and
    /// yaw rate can impose a lower limit.
    pub turn_slow: f64,
}
