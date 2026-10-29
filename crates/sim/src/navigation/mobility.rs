//! How a mover crosses the ground: its speeds, its footprint and what it
//! can shove, and the one speed every movement and planning read takes.
use contract::map::{MoverClass, SurfaceKind};
use contract::movement::{Locomotion, SpeedFactors};
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
    pub locomotion: Locomotion,
    /// Its locomotion's (`movement.speed_factors`).
    pub speed_factors: SpeedFactors,
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
    pub reverse_speed_factor: f64,
    /// How every vehicle drives (`movement.drive`).
    pub feel: contract::movement::DriveRules,
}

/// What a mover's speed reads of the ground under it: the walkable
/// surface (`world::Surface`) where it moves, a navigation cell's copy of
/// it where it plans.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Footing {
    /// The paving under it: one kind, or two where two roads join (the
    /// slower applies); none off any paving.
    pub paved: [Option<SurfaceKind>; 2],
    pub forest: bool,
    pub swamp: bool,
    pub slope_deg: f64,
    /// How full a crater under it is, 0 to 1: the battle's marks, which
    /// movement reads and planning never does (Q8).
    pub crater_fill: f64,
}

impl Mobility {
    /// Its speed factor of road speed on `paved`: the lowest of the kinds
    /// there, 0 off any road.
    pub fn road_factor(&self, paved: [Option<SurfaceKind>; 2]) -> f64 {
        paved
            .into_iter()
            .flatten()
            .map(|kind| self.speed_factors.surfaces.of(kind))
            .reduce(f64::min)
            .unwrap_or(0.0)
    }

    /// Travel speed over `ground`. A road (its factor above 0) takes
    /// precedence over forest and swamp and is never slower than open
    /// ground. Off it, forest and swamp each scale open-ground speed, and
    /// where both lie the slower one applies, once. Slopes slow
    /// continuously up to the map's cutoff, and craters in proportion to
    /// how full they are, never to a stop.
    pub fn speed(&self, ground: Footing) -> f64 {
        let f = &self.speed_factors;
        let road = self.road_factor(ground.paved);
        let base = if road > 0.0 {
            (self.road_mps * road).max(self.off_road_mps)
        } else {
            let forest = if ground.forest { f.forest } else { 1.0 };
            let swamp = if ground.swamp { f.swamp } else { 1.0 };
            self.off_road_mps * forest.min(swamp)
        };
        base * f.slope.factor(ground.slope_deg) * (1.0 - (1.0 - f.crater) * ground.crater_fill)
    }

    pub fn max_speed(&self) -> f64 {
        self.road_mps.max(self.off_road_mps)
    }
}
