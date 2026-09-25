//! What one side is allowed to know at a completed tick. Presentation, audio,
//! picking and controllers consume only this.
use crate::command::RoutePolicy;
use crate::ids::{Tick, UnitId};
use crate::scenario::UnitKind;
use serde::{Deserialize, Serialize};

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
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObservationFrame {
    pub tick: Tick,
    pub own: Vec<OwnUnit>,
}
