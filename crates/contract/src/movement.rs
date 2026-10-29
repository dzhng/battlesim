//! How ground units move over the map, whatever each one's own speeds:
//! how each kind of ground scales them, turret and bearing tolerances, and
//! how every vehicle drives.
use crate::map::SurfaceKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementRules {
    /// How each kind of ground scales each locomotion's own top speeds.
    pub speed_factors: SpeedFactorTable,
    pub turret_turn_deg_s: f64,
    pub bearing_tolerance_deg: f64,
    pub drive: DriveRules,
}

/// How a ground unit moves over the ground: its catalog `mobility` kind.
/// It, not the unit's size, decides how terrain slows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locomotion {
    Foot,
    Wheeled,
    Tracked,
}

impl Locomotion {
    pub const ALL: [Locomotion; 3] = [Locomotion::Foot, Locomotion::Wheeled, Locomotion::Tracked];
}

/// Every locomotion's speed factors (the rules' `movement.speed_factors`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeedFactorTable {
    pub foot: SpeedFactors,
    pub wheeled: SpeedFactors,
    pub tracked: SpeedFactors,
}

impl SpeedFactorTable {
    /// No factor speeds a mover past its road speed, which route planning's
    /// best case assumes, and no ground but a non-road surface stops it dead.
    pub(crate) fn check(&self) -> Result<(), String> {
        for (name, f) in [
            ("foot", self.foot),
            ("wheeled", self.wheeled),
            ("tracked", self.tracked),
        ] {
            for kind in SurfaceKind::ALL {
                if !(0.0..=1.0).contains(&f.surfaces.of(kind)) {
                    return Err(format!(
                        "speed_factors.{name}.surfaces.{kind:?} must lie in [0, 1]"
                    ));
                }
            }
            for (field, v) in [
                ("forest", f.forest),
                ("swamp", f.swamp),
                ("crater", f.crater),
                ("slope.min_speed_factor", f.slope.min_speed_factor),
            ] {
                if !(v > 0.0 && v <= 1.0) {
                    return Err(format!("speed_factors.{name}.{field} must lie in (0, 1]"));
                }
            }
            if !(f.slope.zero_speed_deg.is_finite() && f.slope.zero_speed_deg > 0.0) {
                return Err(format!(
                    "speed_factors.{name}.slope.zero_speed_deg must be finite and positive"
                ));
            }
        }
        Ok(())
    }

    pub fn of(&self, locomotion: Locomotion) -> &SpeedFactors {
        match locomotion {
            Locomotion::Foot => &self.foot,
            Locomotion::Wheeled => &self.wheeled,
            Locomotion::Tracked => &self.tracked,
        }
    }
}

/// How each kind of ground scales one locomotion's speed: a share of its
/// unit's own road or off-road top speed, at most 1, so the road speed is
/// always the fastest it goes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeedFactors {
    /// Of road speed on each surface kind, never below off-road speed; 0 is
    /// no road at all.
    pub surfaces: SurfaceSpeedFactors,
    /// Of off-road speed in forest, off any road.
    pub forest: f64,
    /// Of off-road speed in a swamp's water, off any road, however deep;
    /// where forest stands in it too, the slower of the two applies, once.
    pub swamp: f64,
    /// Of the speed it would have, over a full crater; a shallower one slows
    /// it in proportion (`ground.crater_full_depth`).
    pub crater: f64,
    pub slope: SlopeSpeed,
}

/// One factor per surface kind a map may pave.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceSpeedFactors {
    pub road: f64,
    pub country_road: f64,
    pub dirt_track: f64,
    pub paving: f64,
}

impl SurfaceSpeedFactors {
    pub fn of(&self, kind: SurfaceKind) -> f64 {
        match kind {
            SurfaceKind::Road => self.road,
            SurfaceKind::CountryRoad => self.country_road,
            SurfaceKind::DirtTrack => self.dirt_track,
            SurfaceKind::Paving => self.paving,
        }
    }
}

/// Speed falls in proportion to slope, reaching zero at `zero_speed_deg`,
/// but never below `min_speed_factor` on ground the map lets it climb.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlopeSpeed {
    pub zero_speed_deg: f64,
    pub min_speed_factor: f64,
}

impl SlopeSpeed {
    pub fn factor(&self, slope_deg: f64) -> f64 {
        (1.0 - slope_deg / self.zero_speed_deg).max(self.min_speed_factor)
    }
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
    /// Speed factor during a manoeuvre at most; the turning radius and yaw
    /// rate can impose a lower limit.
    pub turn_speed_factor: f64,
}
