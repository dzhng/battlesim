//! Player and controller intent. Commands are validated by the authority,
//! acknowledged in order, and applied at a tick boundary.
use crate::ids::{Side, Tick, UnitId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePolicy {
    /// Minimise distance (ordinary right-click move).
    Shortest,
    /// Minimise travel time using roads and terrain speeds (double right-click).
    Fastest,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Order {
    Move {
        units: Vec<UnitId>,
        /// Client-issued token identifying this gesture for a later upgrade.
        gesture: u64,
        goal: [f64; 2],
        route: RoutePolicy,
    },
    Stop {
        units: Vec<UnitId>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub side: Side,
    /// Strictly increasing per side, starting at 1.
    pub seq: u64,
    pub order: Order,
    /// Shift: append to the queue instead of replacing it.
    #[serde(default)]
    pub queued: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum OrderError {
    /// `seq` is not the next number for this side.
    OutOfSequence {
        expected: u64,
    },
    NoUnits,
    UnknownUnit {
        unit: UnitId,
    },
    NotOwnUnit {
        unit: UnitId,
    },
    OutOfBounds,
    /// Input is disabled while a replay feeds recorded commands.
    ReplayInProgress,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandAck {
    pub seq: u64,
    /// The tick whose step applies the command (accepted or not, the tick it was judged against).
    pub applied_tick: Tick,
    pub error: Option<OrderError>,
}
