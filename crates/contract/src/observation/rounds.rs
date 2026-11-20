//! The rounds a side sees this tick: each stretch of flight it may draw, where
//! it glanced and what ended it, and the bursts.
use serde::{Deserialize, Serialize};

/// What ended a round's flight at a segment's `to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentHit {
    /// Still flying, expired, or its end is not shown.
    None,
    Ground,
    /// A vehicle hull.
    Hull,
    /// A building, wall, hedge or other prop that is not metal.
    Prop,
    Soldier,
    /// A prop whose body is metal (`PropBody.metal`): a car, wreck, lamp
    /// post or railing.
    MetalProp,
}

/// A stretch of a round's flight this side may draw this tick.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisibleSegment {
    /// The flown path, at least two points: a polyline that bends where the
    /// round ricocheted.
    pub path: Vec<[f64; 3]>,
    /// Where along `path` the round glanced off a hull this tick.
    pub ricochets: Vec<SegmentRicochet>,
    /// Fired by this side.
    pub own: bool,
    /// The round kind: an index into the rules' weapon rows in name order.
    /// An enemy tracer reveals its shooter's class (F3).
    pub kind: usize,
    /// The soldier who fired it (`Soldier.id`); `None` for a vehicle's gun.
    pub shooter_member: Option<u32>,
    /// What the round struck at the path's end this tick (shown only when
    /// that point is seen).
    pub hit: SegmentHit,
    /// Outward surface normal at the impact, world frame; `None` without one.
    pub impact_normal: Option<[f64; 3]>,
}

/// A ricochet on a segment's path: the round glanced off a hull at
/// `path[point]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SegmentRicochet {
    pub point: usize,
    /// Outward hull normal there, world frame.
    pub normal: [f64; 3],
}

/// A round's burst this tick: HE, grenades and missiles. Own blasts are all
/// published; enemy blasts use sight in the projectile's fixed band.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Blast {
    pub point: [f64; 3],
    pub radius: f64,
    /// The round kind, as on [`VisibleSegment::kind`].
    pub kind: usize,
}
