//! What the driver is fed: one published moment's units, each soldier and
//! mount as the side sees them, and the fallen.
use std::sync::Arc;

use crate::world::Vec3;

#[derive(Clone, Debug, Default)]
pub struct FeedSoldier {
    pub id: f64,
    /// His slot in his squad type: which soldier kind he is.
    pub slot: usize,
    /// The weapon he has in use, as published.
    pub active_mount: Option<usize>,
    pub position: Vec3,
    /// The rounds he has launched with his selected weapon (the launches the
    /// flashes and sounds read); a rise is his shot.
    pub shots: f64,
    /// Where his body stands while he leans out past his cover's edge to fire.
    pub lean: Option<[f64; 2]>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FeedMount {
    /// The soldier operating it, as published.
    pub operator: Option<f64>,
    /// World bearing, radians.
    pub bearing: f64,
    pub elevation: f64,
    pub round: usize,
    /// Rounds launched so far; a rise is a shot.
    pub shots: f64,
}

#[derive(Clone, Debug, Default)]
pub struct FeedUnit {
    pub id: f64,
    pub kind: Arc<str>,
    pub side: Arc<str>,
    pub position: Vec3,
    /// Hull heading, radians counter-clockwise from +X.
    pub yaw: f64,
    pub soldiers: Vec<FeedSoldier>,
    pub mounts: Vec<FeedMount>,
    pub deployment: Option<f64>,
    /// The squad is pinned, by the published tier (false where the side cannot know).
    pub pinned: bool,
    /// A downed airframe's (pitch, roll) as it falls, radians: nose up and
    /// right side down. None: a live unit.
    pub attitude: Option<(f64, f64)>,
    /// It stands on the ground though its type flies: a parked aircraft,
    /// shut down, posed as a ground vehicle is, its rotors still. False in
    /// battle, where a live aircraft always flies.
    pub grounded: bool,
}

#[derive(Clone, Debug)]
pub struct FeedFallen {
    pub soldier: f64,
    pub position: Vec3,
    /// His squad's heading when he fell.
    pub yaw: f64,
    pub kind: Arc<str>,
    pub slot: usize,
    pub side: Arc<str>,
}

/// One published moment, in simulation seconds.
#[derive(Clone, Debug, Default)]
pub struct FeedFrame {
    pub time: f64,
    pub units: Vec<FeedUnit>,
    /// Everyone the side knows has fallen: the same list while unchanged,
    /// so the driver reconciles corpses only when it changes.
    pub fallen: Arc<Vec<FeedFallen>>,
}
