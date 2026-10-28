//! How a mover crosses the ground: its speeds, its footprint and what it
//! can shove, and the one speed every movement and planning read takes.
use contract::map::MoverClass;
use contract::scenario::PushClass;

use crate::math::V2;

/// How a unit class moves: its speeds on each surface and how wide it is.
/// A hull to park: half its length and half its width, and the facing it
/// was ordered to keep, if it can turn on the spot to keep it. Without one
/// it parks facing the way it drives in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parking {
    pub half: V2,
    pub facing: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mobility {
    pub off_road_mps: f64,
    pub road_mps: f64,
    pub forest_multiplier: f64,
    /// Its drive's factor in a swamp's water (`movement.swamp`).
    pub swamp_multiplier: f64,
    /// Distance the footprint needs from any obstacle centre line.
    pub half_width_m: f64,
    /// Which props stop it.
    pub class: MoverClass,
    /// What it can shove aside (vehicles; infantry shoves nothing).
    pub push: PushClass,
    /// How a vehicle steers and reverses (Q29, Q30); `None` for infantry.
    pub drive: Option<Drive>,
}

/// A vehicle's kinematics, from its body row (Q29, Q30). Planning never
/// reads it: the follower owns the kinematics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drive {
    /// Tracked vehicles pivot on the spot; wheeled ones never do.
    pub tracked: bool,
    pub turn_rad_s: f64,
    /// A wheeled vehicle's tightest turn (0 for tracks).
    pub radius_m: f64,
    pub reverse_fraction: f64,
    /// How every vehicle drives (`movement.drive`).
    pub feel: contract::movement::DriveRules,
}

/// What a mover's speed reads of the ground under it: the walkable
/// surface (`world::Surface`) where it moves, a navigation cell's copy of
/// it where it plans.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Footing {
    /// The surface kind's share of road speed: 0 off any road.
    pub road_factor: f64,
    pub forest: bool,
    pub swamp: bool,
    pub slope_deg: f64,
}

impl Mobility {
    /// Travel speed over `ground`. A road (`road_factor` above 0, the
    /// surface kind's share of road speed) takes precedence over forest and
    /// swamp and is never slower than open ground. Off it, forest and swamp
    /// each multiply open-ground speed, and where both lie the slower one
    /// applies, once. Slopes slow continuously up to the shared cutoff.
    pub fn speed(&self, ground: Footing) -> f64 {
        let base = if ground.road_factor > 0.0 {
            (self.road_mps * ground.road_factor).max(self.off_road_mps)
        } else {
            let forest = if ground.forest {
                self.forest_multiplier
            } else {
                1.0
            };
            let swamp = if ground.swamp {
                self.swamp_multiplier
            } else {
                1.0
            };
            self.off_road_mps * forest.min(swamp)
        };
        base * slope_multiplier(ground.slope_deg)
    }

    pub fn max_speed(&self) -> f64 {
        self.road_mps.max(self.off_road_mps)
    }
}

/// `max(0.35, 1 - slope/50)` below the cutoff (contracts.md).
pub fn slope_multiplier(slope_deg: f64) -> f64 {
    (1.0 - slope_deg / 50.0).max(0.35)
}
