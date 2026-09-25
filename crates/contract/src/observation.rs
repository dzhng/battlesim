//! What one side is allowed to know at a completed tick. Presentation, audio,
//! picking and controllers consume only this.
use crate::command::RoutePolicy;
use crate::ids::{Tick, UnitId};
use crate::scenario::UnitKind;
use serde::{Deserialize, Serialize};

/// A side-scoped handle for an identified enemy. It is not the enemy's unit
/// id, and it is retired when identification lapses past its grace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObservedTargetId(pub u32);

/// What a side knows of an identified enemy: its class and value and what its
/// sensors saw this tick. Never health, ammunition, orders or timers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdentifiedUnit {
    pub id: ObservedTargetId,
    pub kind: UnitKind,
    pub cost: u32,
    pub position: [f64; 3],
    pub yaw: f64,
    /// Observed ground velocity (from successive sightings; zero when first seen).
    pub velocity: [f64; 2],
    /// Positions of the squad members actually seen this tick (infantry only).
    pub members: Vec<[f64; 3]>,
}

/// Which ground this side can currently see: row-major cells of `cell_m`,
/// one bit per cell, set when visible.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VisibilityField {
    pub cell_m: f64,
    pub nx: u32,
    pub ny: u32,
    pub bits: Vec<u32>,
}

impl VisibilityField {
    pub fn visible(&self, x: f64, y: f64) -> bool {
        let (i, j) = ((x / self.cell_m).floor(), (y / self.cell_m).floor());
        if i < 0.0 || j < 0.0 || i >= self.nx as f64 || j >= self.ny as f64 {
            return false;
        }
        let k = j as usize * self.nx as usize + i as usize;
        self.bits[k / 32] & (1 << (k % 32)) != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveState {
    Idle,
    Moving,
    /// Yielding to friendly traffic; `blocker` says who.
    Waiting,
    /// No known route; the destination is kept and retried on relevant change.
    RouteBlocked,
}

/// A unit of the observing side: its own state is complete.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnUnit {
    pub id: UnitId,
    pub kind: UnitKind,
    pub position: [f64; 3],
    pub yaw: f64,
    /// Destination of the current movement order, if any.
    pub goal: Option<[f64; 2]>,
    pub policy: Option<RoutePolicy>,
    pub state: MoveState,
    /// The friendly unit this one is waiting for.
    pub blocker: Option<UnitId>,
    /// Remaining waypoints of the current route.
    pub route: Vec<[f64; 2]>,
    /// Destinations of the orders queued behind the current one.
    pub queue: Vec<[f64; 2]>,
    /// Living squad members' positions (infantry only).
    pub members: Vec<[f64; 3]>,
    /// Enemies this unit's own sensors identify this tick (own sensor, not shared).
    pub sees: Vec<ObservedTargetId>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObservationFrame {
    pub tick: Tick,
    pub own: Vec<OwnUnit>,
    /// Enemies identified by any friendly sensor (team-shared).
    pub identified: Vec<IdentifiedUnit>,
    pub ground_visibility: VisibilityField,
}
